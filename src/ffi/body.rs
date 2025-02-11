use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ptr;
use std::task::{Context, Poll};

use http::HeaderMap;
use libc::{c_int, size_t};

use super::task::{fluxio_context, fluxio_task, fluxio_task_return_type, AsTaskType};
use super::{UserDataPointer, FLUXIO_ITER_CONTINUE};
use crate::body::{Body, Bytes, HttpBody as _};

/// A streaming HTTP body.
pub struct fluxio_body(pub(super) Body);

/// A buffer of bytes that is sent or received on a `fluxio_body`.
pub struct fluxio_buf(pub(crate) Bytes);

pub(crate) struct UserBody {
    data_func: fluxio_body_data_callback,
    userdata: *mut c_void,
}

// ===== Body =====

type fluxio_body_foreach_callback = extern "C" fn(*mut c_void, *const fluxio_buf) -> c_int;

type fluxio_body_data_callback =
    extern "C" fn(*mut c_void, *mut fluxio_context<'_>, *mut *mut fluxio_buf) -> c_int;

ffi_fn! {
    /// Create a new "empty" body.
    ///
    /// If not configured, this body acts as an empty payload.
    fn fluxio_body_new() -> *mut fluxio_body {
        Box::into_raw(Box::new(fluxio_body(Body::empty())))
    } ?= ptr::null_mut()
}

ffi_fn! {
    /// Free a `fluxio_body *`.
    fn fluxio_body_free(body: *mut fluxio_body) {
        drop(non_null!(Box::from_raw(body) ?= ()));
    }
}

ffi_fn! {
    /// Return a task that will poll the body for the next buffer of data.
    ///
    /// The task value may have different types depending on the outcome:
    ///
    /// - `FLUXIO_TASK_BUF`: Success, and more data was received.
    /// - `FLUXIO_TASK_ERROR`: An error retrieving the data.
    /// - `FLUXIO_TASK_EMPTY`: The body has finished streaming data.
    ///
    /// This does not consume the `fluxio_body *`, so it may be used to again.
    /// However, it MUST NOT be used or freed until the related task completes.
    fn fluxio_body_data(body: *mut fluxio_body) -> *mut fluxio_task {
        // This doesn't take ownership of the Body, so don't allow destructor
        let mut body = ManuallyDrop::new(non_null!(Box::from_raw(body) ?= ptr::null_mut()));

        Box::into_raw(fluxio_task::boxed(async move {
            body.0.data().await.map(|res| res.map(fluxio_buf))
        }))
    } ?= ptr::null_mut()
}

ffi_fn! {
    /// Return a task that will poll the body and execute the callback with each
    /// body chunk that is received.
    ///
    /// The `fluxio_buf` pointer is only a borrowed reference, it cannot live outside
    /// the execution of the callback. You must make a copy to retain it.
    ///
    /// The callback should return `FLUXIO_ITER_CONTINUE` to continue iterating
    /// chunks as they are received, or `FLUXIO_ITER_BREAK` to cancel.
    ///
    /// This will consume the `fluxio_body *`, you shouldn't use it anymore or free it.
    fn fluxio_body_foreach(body: *mut fluxio_body, func: fluxio_body_foreach_callback, userdata: *mut c_void) -> *mut fluxio_task {
        let mut body = non_null!(Box::from_raw(body) ?= ptr::null_mut());
        let userdata = UserDataPointer(userdata);

        Box::into_raw(fluxio_task::boxed(async move {
            while let Some(item) = body.0.data().await {
                let chunk = item?;
                if FLUXIO_ITER_CONTINUE != func(userdata.0, &fluxio_buf(chunk)) {
                    return Err(crate::Error::new_user_aborted_by_callback());
                }
            }
            Ok(())
        }))
    } ?= ptr::null_mut()
}

ffi_fn! {
    /// Set userdata on this body, which will be passed to callback functions.
    fn fluxio_body_set_userdata(body: *mut fluxio_body, userdata: *mut c_void) {
        let b = non_null!(&mut *body ?= ());
        b.0.as_ffi_mut().userdata = userdata;
    }
}

ffi_fn! {
    /// Set the data callback for this body.
    ///
    /// The callback is called each time fluxio needs to send more data for the
    /// body. It is passed the value from `fluxio_body_set_userdata`.
    ///
    /// If there is data available, the `fluxio_buf **` argument should be set
    /// to a `fluxio_buf *` containing the data, and `FLUXIO_POLL_READY` should
    /// be returned.
    ///
    /// Returning `FLUXIO_POLL_READY` while the `fluxio_buf **` argument points
    /// to `NULL` will indicate the body has completed all data.
    ///
    /// If there is more data to send, but it isn't yet available, a
    /// `fluxio_waker` should be saved from the `fluxio_context *` argument, and
    /// `FLUXIO_POLL_PENDING` should be returned. You must wake the saved waker
    /// to signal the task when data is available.
    ///
    /// If some error has occurred, you can return `FLUXIO_POLL_ERROR` to abort
    /// the body.
    fn fluxio_body_set_data_func(body: *mut fluxio_body, func: fluxio_body_data_callback) {
        let b = non_null!{ &mut *body ?= () };
        b.0.as_ffi_mut().data_func = func;
    }
}

// ===== impl UserBody =====

impl UserBody {
    pub(crate) fn new() -> UserBody {
        UserBody {
            data_func: data_noop,
            userdata: std::ptr::null_mut(),
        }
    }

    pub(crate) fn poll_data(&mut self, cx: &mut Context<'_>) -> Poll<Option<crate::Result<Bytes>>> {
        let mut out = std::ptr::null_mut();
        match (self.data_func)(self.userdata, fluxio_context::wrap(cx), &mut out) {
            super::task::FLUXIO_POLL_READY => {
                if out.is_null() {
                    Poll::Ready(None)
                } else {
                    let buf = unsafe { Box::from_raw(out) };
                    Poll::Ready(Some(Ok(buf.0)))
                }
            }
            super::task::FLUXIO_POLL_PENDING => Poll::Pending,
            super::task::FLUXIO_POLL_ERROR => {
                Poll::Ready(Some(Err(crate::Error::new_body_write_aborted())))
            }
            unexpected => Poll::Ready(Some(Err(crate::Error::new_body_write(format!(
                "unexpected fluxio_body_data_func return code {}",
                unexpected
            ))))),
        }
    }

    pub(crate) fn poll_trailers(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<crate::Result<Option<HeaderMap>>> {
        Poll::Ready(Ok(None))
    }
}

/// cbindgen:ignore
extern "C" fn data_noop(
    _userdata: *mut c_void,
    _: *mut fluxio_context<'_>,
    _: *mut *mut fluxio_buf,
) -> c_int {
    super::task::FLUXIO_POLL_READY
}

unsafe impl Send for UserBody {}
unsafe impl Sync for UserBody {}

// ===== Bytes =====

ffi_fn! {
    /// Create a new `fluxio_buf *` by copying the provided bytes.
    ///
    /// This makes an owned copy of the bytes, so the `buf` argument can be
    /// freed or changed afterwards.
    ///
    /// This returns `NULL` if allocating a new buffer fails.
    fn fluxio_buf_copy(buf: *const u8, len: size_t) -> *mut fluxio_buf {
        let slice = unsafe {
            std::slice::from_raw_parts(buf, len)
        };
        Box::into_raw(Box::new(fluxio_buf(Bytes::copy_from_slice(slice))))
    } ?= ptr::null_mut()
}

ffi_fn! {
    /// Get a pointer to the bytes in this buffer.
    ///
    /// This should be used in conjunction with `fluxio_buf_len` to get the length
    /// of the bytes data.
    ///
    /// This pointer is borrowed data, and not valid once the `fluxio_buf` is
    /// consumed/freed.
    fn fluxio_buf_bytes(buf: *const fluxio_buf) -> *const u8 {
        unsafe { (*buf).0.as_ptr() }
    } ?= ptr::null()
}

ffi_fn! {
    /// Get the length of the bytes this buffer contains.
    fn fluxio_buf_len(buf: *const fluxio_buf) -> size_t {
        unsafe { (*buf).0.len() }
    }
}

ffi_fn! {
    /// Free this buffer.
    fn fluxio_buf_free(buf: *mut fluxio_buf) {
        drop(unsafe { Box::from_raw(buf) });
    }
}

unsafe impl AsTaskType for fluxio_buf {
    fn as_task_type(&self) -> fluxio_task_return_type {
        fluxio_task_return_type::FLUXIO_TASK_BUF
    }
}
