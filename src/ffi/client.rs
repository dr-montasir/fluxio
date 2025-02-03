use std::ptr;
use std::sync::Arc;

use libc::c_int;

use crate::client::conn;
use crate::rt::Executor as _;

use super::error::fluxio_code;
use super::http_types::{fluxio_request, fluxio_response};
use super::io::fluxio_io;
use super::task::{fluxio_executor, fluxio_task, fluxio_task_return_type, AsTaskType, WeakExec};

/// An options builder to configure an HTTP client connection.
pub struct fluxio_clientconn_options {
    builder: conn::Builder,
    /// Use a `Weak` to prevent cycles.
    exec: WeakExec,
}

/// An HTTP client connection handle.
///
/// These are used to send a request on a single connection. It's possible to
/// send multiple requests on a single connection, such as when HTTP/1
/// keep-alive or HTTP/2 is used.
pub struct fluxio_clientconn {
    tx: conn::SendRequest<crate::Body>,
}

// ===== impl fluxio_clientconn =====

ffi_fn! {
    /// Starts an HTTP client connection handshake using the provided IO transport
    /// and options.
    ///
    /// Both the `io` and the `options` are consumed in this function call.
    ///
    /// The returned `fluxio_task *` must be polled with an executor until the
    /// handshake completes, at which point the value can be taken.
    fn fluxio_clientconn_handshake(io: *mut fluxio_io, options: *mut fluxio_clientconn_options) -> *mut fluxio_task {
        let options = non_null! { Box::from_raw(options) ?= ptr::null_mut() };
        let io = non_null! { Box::from_raw(io) ?= ptr::null_mut() };

        Box::into_raw(fluxio_task::boxed(async move {
            options.builder.handshake::<_, crate::Body>(io)
                .await
                .map(|(tx, conn)| {
                    options.exec.execute(Box::pin(async move {
                        let _ = conn.await;
                    }));
                    fluxio_clientconn { tx }
                })
        }))
    } ?= std::ptr::null_mut()
}

ffi_fn! {
    /// Send a request on the client connection.
    ///
    /// Returns a task that needs to be polled until it is ready. When ready, the
    /// task yields a `fluxio_response *`.
    fn fluxio_clientconn_send(conn: *mut fluxio_clientconn, req: *mut fluxio_request) -> *mut fluxio_task {
        let mut req = non_null! { Box::from_raw(req) ?= ptr::null_mut() };

        // Update request with original-case map of headers
        req.finalize_request();

        let fut = non_null! { &mut *conn ?= ptr::null_mut() }.tx.send_request(req.0);

        let fut = async move {
            fut.await.map(fluxio_response::wrap)
        };

        Box::into_raw(fluxio_task::boxed(fut))
    } ?= std::ptr::null_mut()
}

ffi_fn! {
    /// Free a `fluxio_clientconn *`.
    fn fluxio_clientconn_free(conn: *mut fluxio_clientconn) {
        drop(non_null! { Box::from_raw(conn) ?= () });
    }
}

unsafe impl AsTaskType for fluxio_clientconn {
    fn as_task_type(&self) -> fluxio_task_return_type {
        fluxio_task_return_type::FLUXIO_TASK_CLIENTCONN
    }
}

// ===== impl fluxio_clientconn_options =====

ffi_fn! {
    /// Creates a new set of HTTP clientconn options to be used in a handshake.
    fn fluxio_clientconn_options_new() -> *mut fluxio_clientconn_options {
        let builder = conn::Builder::new();

        Box::into_raw(Box::new(fluxio_clientconn_options {
            builder,
            exec: WeakExec::new(),
        }))
    } ?= std::ptr::null_mut()
}

ffi_fn! {
    /// Set the whether or not header case is preserved.
    ///
    /// Pass `0` to allow lowercase normalization (default), `1` to retain original case.
    fn fluxio_clientconn_options_set_preserve_header_case(opts: *mut fluxio_clientconn_options, enabled: c_int) {
        let opts = non_null! { &mut *opts ?= () };
        opts.builder.http1_preserve_header_case(enabled != 0);
    }
}

ffi_fn! {
    /// Set the whether or not header order is preserved.
    ///
    /// Pass `0` to allow reordering (default), `1` to retain original ordering.
    fn fluxio_clientconn_options_set_preserve_header_order(opts: *mut fluxio_clientconn_options, enabled: c_int) {
        let opts = non_null! { &mut *opts ?= () };
        opts.builder.http1_preserve_header_order(enabled != 0);
    }
}

ffi_fn! {
    /// Free a `fluxio_clientconn_options *`.
    fn fluxio_clientconn_options_free(opts: *mut fluxio_clientconn_options) {
        drop(non_null! { Box::from_raw(opts) ?= () });
    }
}

ffi_fn! {
    /// Set the client background task executor.
    ///
    /// This does not consume the `options` or the `exec`.
    fn fluxio_clientconn_options_exec(opts: *mut fluxio_clientconn_options, exec: *const fluxio_executor) {
        let opts = non_null! { &mut *opts ?= () };

        let exec = non_null! { Arc::from_raw(exec) ?= () };
        let weak_exec = fluxio_executor::downgrade(&exec);
        std::mem::forget(exec);

        opts.builder.executor(weak_exec.clone());
        opts.exec = weak_exec;
    }
}

ffi_fn! {
    /// Set the whether to use HTTP2.
    ///
    /// Pass `0` to disable, `1` to enable.
    fn fluxio_clientconn_options_http2(opts: *mut fluxio_clientconn_options, enabled: c_int) -> fluxio_code {
        #[cfg(feature = "http2")]
        {
            let opts = non_null! { &mut *opts ?= fluxio_code::FLUXIO_INVALID_ARG };
            opts.builder.http2_only(enabled != 0);
            fluxio_code::FLUXIO_OK
        }

        #[cfg(not(feature = "http2"))]
        {
            drop(opts);
            drop(enabled);
            fluxio_code::FLUXIO_FEATURE_NOT_ENABLED
        }
    }
}

ffi_fn! {
    /// Set the whether to include a copy of the raw headers in responses
    /// received on this connection.
    ///
    /// Pass `0` to disable, `1` to enable.
    ///
    /// If enabled, see `fluxio_response_headers_raw()` for usage.
    fn fluxio_clientconn_options_headers_raw(opts: *mut fluxio_clientconn_options, enabled: c_int) -> fluxio_code {
        let opts = non_null! { &mut *opts ?= fluxio_code::FLUXIO_INVALID_ARG };
        opts.builder.http1_headers_raw(enabled != 0);
        fluxio_code::FLUXIO_OK
    }
}
