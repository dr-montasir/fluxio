use libc::size_t;

/// A more detailed error object returned by some fluxio functions.
pub struct fluxio_error(crate::Error);

/// A return code for many of fluxio's methods.
#[repr(C)]
pub enum fluxio_code {
    /// All is well.
    FLUXIO_OK,
    /// General error, details in the `fluxio_error *`.
    FLUXIO_ERROR,
    /// A function argument was invalid.
    FLUXIO_INVALID_ARG,
    /// The IO transport returned an EOF when one wasn't expected.
    ///
    /// This typically means an HTTP request or response was expected, but the
    /// connection closed cleanly without sending (all of) it.
    FLUXIO_UNEXPECTED_EOF,
    /// Aborted by a user supplied callback.
    FLUXIO_ABORTED_BY_CALLBACK,
    /// An optional fluxio feature was not enabled.
    #[cfg_attr(feature = "http2", allow(unused))]
    FLUXIO_FEATURE_NOT_ENABLED,
    /// The peer sent an HTTP message that could not be parsed.
    FLUXIO_INVALID_PEER_MESSAGE,
}

// ===== impl fluxio_error =====

impl fluxio_error {
    fn code(&self) -> fluxio_code {
        use crate::error::Kind as ErrorKind;
        use crate::error::User;

        match self.0.kind() {
            ErrorKind::Parse(_) => fluxio_code::FLUXIO_INVALID_PEER_MESSAGE,
            ErrorKind::IncompleteMessage => fluxio_code::FLUXIO_UNEXPECTED_EOF,
            ErrorKind::User(User::AbortedByCallback) => fluxio_code::FLUXIO_ABORTED_BY_CALLBACK,
            // TODO: add more variants
            _ => fluxio_code::FLUXIO_ERROR,
        }
    }

    fn print_to(&self, dst: &mut [u8]) -> usize {
        use std::io::Write;

        let mut dst = std::io::Cursor::new(dst);

        // A write! error doesn't matter. As much as possible will have been
        // written, and the Cursor position will know how far that is (even
        // if that is zero).
        let _ = write!(dst, "{}", &self.0);
        dst.position() as usize
    }
}

ffi_fn! {
    /// Frees a `fluxio_error`.
    fn fluxio_error_free(err: *mut fluxio_error) {
        drop(non_null!(Box::from_raw(err) ?= ()));
    }
}

ffi_fn! {
    /// Get an equivalent `fluxio_code` from this error.
    fn fluxio_error_code(err: *const fluxio_error) -> fluxio_code {
        non_null!(&*err ?= fluxio_code::FLUXIO_INVALID_ARG).code()
    }
}

ffi_fn! {
    /// Print the details of this error to a buffer.
    ///
    /// The `dst_len` value must be the maximum length that the buffer can
    /// store.
    ///
    /// The return value is number of bytes that were written to `dst`.
    fn fluxio_error_print(err: *const fluxio_error, dst: *mut u8, dst_len: size_t) -> size_t {
        let dst = unsafe {
            std::slice::from_raw_parts_mut(dst, dst_len)
        };
        non_null!(&*err ?= 0).print_to(dst)
    }
}
