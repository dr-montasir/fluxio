#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <sys/select.h>
#include <assert.h>

#include <sys/types.h>
#include <sys/socket.h>
#include <netdb.h>
#include <string.h>

#include "fluxio.h"

struct conn_data
{
  int fd;
  fluxio_waker *read_waker;
  fluxio_waker *write_waker;
};

static size_t read_cb(void *userdata, fluxio_context *ctx, uint8_t *buf, size_t buf_len)
{
  struct conn_data *conn = (struct conn_data *)userdata;
  ssize_t ret = read(conn->fd, buf, buf_len);

  if (ret >= 0)
  {
    return ret;
  }

  if (errno != EAGAIN)
  {
    // kaboom
    return FLUXIO_IO_ERROR;
  }

  // would block, register interest
  if (conn->read_waker != NULL)
  {
    fluxio_waker_free(conn->read_waker);
  }
  conn->read_waker = fluxio_context_waker(ctx);
  return FLUXIO_IO_PENDING;
}

static size_t write_cb(void *userdata, fluxio_context *ctx, const uint8_t *buf, size_t buf_len)
{
  struct conn_data *conn = (struct conn_data *)userdata;
  ssize_t ret = write(conn->fd, buf, buf_len);

  if (ret >= 0)
  {
    return ret;
  }

  if (errno != EAGAIN)
  {
    // kaboom
    return FLUXIO_IO_ERROR;
  }

  // would block, register interest
  if (conn->write_waker != NULL)
  {
    fluxio_waker_free(conn->write_waker);
  }
  conn->write_waker = fluxio_context_waker(ctx);
  return FLUXIO_IO_PENDING;
}

static void free_conn_data(struct conn_data *conn)
{
  if (conn->read_waker)
  {
    fluxio_waker_free(conn->read_waker);
    conn->read_waker = NULL;
  }
  if (conn->write_waker)
  {
    fluxio_waker_free(conn->write_waker);
    conn->write_waker = NULL;
  }

  free(conn);
}

static int connect_to(const char *host, const char *port)
{
  struct addrinfo hints;
  memset(&hints, 0, sizeof(struct addrinfo));
  hints.ai_family = AF_UNSPEC;
  hints.ai_socktype = SOCK_STREAM;

  struct addrinfo *result, *rp;
  if (getaddrinfo(host, port, &hints, &result) != 0)
  {
    printf("dns failed for %s\n", host);
    return -1;
  }

  int sfd;
  for (rp = result; rp != NULL; rp = rp->ai_next)
  {
    sfd = socket(rp->ai_family, rp->ai_socktype, rp->ai_protocol);
    if (sfd == -1)
    {
      continue;
    }

    if (connect(sfd, rp->ai_addr, rp->ai_addrlen) != -1)
    {
      break;
    }

    close(sfd);
  }

  freeaddrinfo(result);

  // no address succeeded
  if (rp == NULL)
  {
    printf("connect failed for %s\n", host);
    return -1;
  }

  return sfd;
}

static int print_each_header(void *userdata,
                             const uint8_t *name,
                             size_t name_len,
                             const uint8_t *value,
                             size_t value_len)
{
  printf("%.*s: %.*s\n", (int)name_len, name, (int)value_len, value);
  return FLUXIO_ITER_CONTINUE;
}

static int print_each_chunk(void *userdata, const fluxio_buf *chunk)
{
  const uint8_t *buf = fluxio_buf_bytes(chunk);
  size_t len = fluxio_buf_len(chunk);

  write(1, buf, len);

  return FLUXIO_ITER_CONTINUE;
}

typedef enum
{
  EXAMPLE_NOT_SET = 0, // tasks we don't know about won't have a userdata set
  EXAMPLE_HANDSHAKE,
  EXAMPLE_SEND,
  EXAMPLE_RESP_BODY
} example_id;

#define STR_ARG(XX) (uint8_t *)XX, strlen(XX)

int main(int argc, char *argv[])
{
  const char *host = argc > 1 ? argv[1] : "httpbin.org";
  const char *port = argc > 2 ? argv[2] : "80";
  const char *path = argc > 3 ? argv[3] : "/";
  printf("connecting to port %s on %s...\n", port, host);

  int fd = connect_to(host, port);
  if (fd < 0)
  {
    return 1;
  }

  printf("connected to %s, now get %s\n", host, path);
  if (fcntl(fd, F_SETFL, O_NONBLOCK) != 0)
  {
    printf("failed to set socket to non-blocking\n");
    return 1;
  }

  fd_set fds_read;
  fd_set fds_write;
  fd_set fds_excep;

  struct conn_data *conn = malloc(sizeof(struct conn_data));

  conn->fd = fd;
  conn->read_waker = NULL;
  conn->write_waker = NULL;

  // Hookup the IO
  fluxio_io *io = fluxio_io_new();
  fluxio_io_set_userdata(io, (void *)conn);
  fluxio_io_set_read(io, read_cb);
  fluxio_io_set_write(io, write_cb);

  printf("http handshake (fluxio v%s) ...\n", fluxio_version());

  // We need an executor generally to poll futures
  const fluxio_executor *exec = fluxio_executor_new();

  // Prepare client options
  fluxio_clientconn_options *opts = fluxio_clientconn_options_new();
  fluxio_clientconn_options_exec(opts, exec);

  fluxio_task *handshake = fluxio_clientconn_handshake(io, opts);
  fluxio_task_set_userdata(handshake, (void *)EXAMPLE_HANDSHAKE);

  // Let's wait for the handshake to finish...
  fluxio_executor_push(exec, handshake);

  // In case a task errors...
  fluxio_error *err;

  // The polling state machine!
  while (1)
  {
    // Poll all ready tasks and act on them...
    while (1)
    {
      fluxio_task *task = fluxio_executor_poll(exec);
      if (!task)
      {
        break;
      }
      switch ((example_id)fluxio_task_userdata(task))
      {
      case EXAMPLE_HANDSHAKE:;
        if (fluxio_task_type(task) == FLUXIO_TASK_ERROR)
        {
          printf("handshake error!\n");
          err = fluxio_task_value(task);
          goto fail;
        }
        assert(fluxio_task_type(task) == FLUXIO_TASK_CLIENTCONN);

        printf("preparing http request ...\n");

        fluxio_clientconn *client = fluxio_task_value(task);
        fluxio_task_free(task);

        // Prepare the request
        fluxio_request *req = fluxio_request_new();
        if (fluxio_request_set_method(req, STR_ARG("GET")))
        {
          printf("error setting method\n");
          return 1;
        }
        if (fluxio_request_set_uri(req, STR_ARG(path)))
        {
          printf("error setting uri\n");
          return 1;
        }

        fluxio_headers *req_headers = fluxio_request_headers(req);
        fluxio_headers_set(req_headers, STR_ARG("Host"), STR_ARG(host));

        // Send it!
        fluxio_task *send = fluxio_clientconn_send(client, req);
        fluxio_task_set_userdata(send, (void *)EXAMPLE_SEND);
        printf("sending ...\n");
        fluxio_executor_push(exec, send);

        // For this example, no longer need the client
        fluxio_clientconn_free(client);

        break;
      case EXAMPLE_SEND:;
        if (fluxio_task_type(task) == FLUXIO_TASK_ERROR)
        {
          printf("send error!\n");
          err = fluxio_task_value(task);
          goto fail;
        }
        assert(fluxio_task_type(task) == FLUXIO_TASK_RESPONSE);

        // Take the results
        fluxio_response *resp = fluxio_task_value(task);
        fluxio_task_free(task);

        uint16_t http_status = fluxio_response_status(resp);
        const uint8_t *rp = fluxio_response_reason_phrase(resp);
        size_t rp_len = fluxio_response_reason_phrase_len(resp);

        printf("\nResponse Status: %d %.*s\n", http_status, (int)rp_len, rp);

        fluxio_headers *headers = fluxio_response_headers(resp);
        fluxio_headers_foreach(headers, print_each_header, NULL);
        printf("\n");

        fluxio_body *resp_body = fluxio_response_body(resp);
        fluxio_task *foreach = fluxio_body_foreach(resp_body, print_each_chunk, NULL);
        fluxio_task_set_userdata(foreach, (void *)EXAMPLE_RESP_BODY);
        fluxio_executor_push(exec, foreach);

        // No longer need the response
        fluxio_response_free(resp);

        break;
      case EXAMPLE_RESP_BODY:;
        if (fluxio_task_type(task) == FLUXIO_TASK_ERROR)
        {
          printf("body error!\n");
          err = fluxio_task_value(task);
          goto fail;
        }

        assert(fluxio_task_type(task) == FLUXIO_TASK_EMPTY);

        printf("\n -- Done! -- \n");

        // Cleaning up before exiting
        fluxio_task_free(task);
        fluxio_executor_free(exec);
        free_conn_data(conn);

        return 0;
      case EXAMPLE_NOT_SET:
        // A background task for fluxio completed...
        fluxio_task_free(task);
        break;
      }
    }

    // All futures are pending on IO work, so select on the fds.

    FD_ZERO(&fds_read);
    FD_ZERO(&fds_write);
    FD_ZERO(&fds_excep);

    if (conn->read_waker)
    {
      FD_SET(conn->fd, &fds_read);
    }
    if (conn->write_waker)
    {
      FD_SET(conn->fd, &fds_write);
    }

    int sel_ret = select(conn->fd + 1, &fds_read, &fds_write, &fds_excep, NULL);

    if (sel_ret < 0)
    {
      printf("select() error\n");
      return 1;
    }

    if (FD_ISSET(conn->fd, &fds_read))
    {
      fluxio_waker_wake(conn->read_waker);
      conn->read_waker = NULL;
    }

    if (FD_ISSET(conn->fd, &fds_write))
    {
      fluxio_waker_wake(conn->write_waker);
      conn->write_waker = NULL;
    }
  }

  return 0;

fail:
  if (err)
  {
    printf("error code: %d\n", fluxio_error_code(err));
    // grab the error details
    char errbuf[256];
    size_t errlen = fluxio_error_print(err, errbuf, sizeof(errbuf));
    printf("details: %.*s\n", (int)errlen, errbuf);

    // clean up the error
    fluxio_error_free(err);
  }
  return 1;
}
