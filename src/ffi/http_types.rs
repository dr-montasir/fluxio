use bytes::Bytes;
use libc::{c_int, size_t};
use std::ffi::c_void;

use super::body::{fluxio_body, fluxio_buf};
use super::error::fluxio_code;
use super::task::{fluxio_task_return_type, AsTaskType};
use super::{UserDataPointer, FLUXIO_ITER_CONTINUE};
use crate::ext::{HeaderCaseMap, OriginalHeaderOrder};
use crate::header::{HeaderName, HeaderValue};
use crate::{Body, HeaderMap, Method, Request, Response, Uri};

/// An HTTP request.
pub struct fluxio_request(pub(super) Request<Body>);

/// An HTTP response.
pub struct fluxio_response(pub(super) Response<Body>);

/// An HTTP header map.
///
/// These can be part of a request or response.
pub struct fluxio_headers {
    pub(super) headers: HeaderMap,
    orig_casing: HeaderCaseMap,
    orig_order: OriginalHeaderOrder,
}

#[derive(Debug)]
pub(crate) struct ReasonPhrase(pub(crate) Bytes);

pub(crate) struct RawHeaders(pub(crate) fluxio_buf);

pub(crate) struct OnInformational {
    func: fluxio_request_on_informational_callback,
    data: UserDataPointer,
}

type fluxio_request_on_informational_callback = extern "C" fn(*mut c_void, *mut fluxio_response);

// ===== impl fluxio_request =====

ffi_fn! {
    /// Construct a new HTTP request.
    fn fluxio_request_new() -> *mut fluxio_request {
        Box::into_raw(Box::new(fluxio_request(Request::new(Body::empty()))))
    } ?= std::ptr::null_mut()
}

ffi_fn! {
    /// Free an HTTP request if not going to send it on a client.
    fn fluxio_request_free(req: *mut fluxio_request) {
        drop(non_null!(Box::from_raw(req) ?= ()));
    }
}

ffi_fn! {
    /// Set the HTTP Method of the request.
    fn fluxio_request_set_method(req: *mut fluxio_request, method: *const u8, method_len: size_t) -> fluxio_code {
        let bytes = unsafe {
            std::slice::from_raw_parts(method, method_len as usize)
        };
        let req = non_null!(&mut *req ?= fluxio_code::FLUXIO_INVALID_ARG);
        match Method::from_bytes(bytes) {
            Ok(m) => {
                *req.0.method_mut() = m;
                fluxio_code::FLUXIO_OK
            },
            Err(_) => {
                fluxio_code::FLUXIO_INVALID_ARG
            }
        }
    }
}

ffi_fn! {
    /// Set the URI of the request.
    ///
    /// The request's URI is best described as the `request-target` from the RFCs. So in HTTP/1,
    /// whatever is set will get sent as-is in the first line (GET $uri HTTP/1.1). It
    /// supports the 4 defined variants, origin-form, absolute-form, authority-form, and
    /// asterisk-form.
    ///
    /// The underlying type was built to efficiently support HTTP/2 where the request-target is
    /// split over :scheme, :authority, and :path. As such, each part can be set explicitly, or the
    /// type can parse a single contiguous string and if a scheme is found, that slot is "set". If
    /// the string just starts with a path, only the path portion is set. All pseudo headers that
    /// have been parsed/set are sent when the connection type is HTTP/2.
    ///
    /// To set each slot explicitly, use `fluxio_request_set_uri_parts`.
    fn fluxio_request_set_uri(req: *mut fluxio_request, uri: *const u8, uri_len: size_t) -> fluxio_code {
        let bytes = unsafe {
            std::slice::from_raw_parts(uri, uri_len as usize)
        };
        let req = non_null!(&mut *req ?= fluxio_code::FLUXIO_INVALID_ARG);
        match Uri::from_maybe_shared(bytes) {
            Ok(u) => {
                *req.0.uri_mut() = u;
                fluxio_code::FLUXIO_OK
            },
            Err(_) => {
                fluxio_code::FLUXIO_INVALID_ARG
            }
        }
    }
}

ffi_fn! {
    /// Set the URI of the request with separate scheme, authority, and
    /// path/query strings.
    ///
    /// Each of `scheme`, `authority`, and `path_and_query` should either be
    /// null, to skip providing a component, or point to a UTF-8 encoded
    /// string. If any string pointer argument is non-null, its corresponding
    /// `len` parameter must be set to the string's length.
    fn fluxio_request_set_uri_parts(
        req: *mut fluxio_request,
        scheme: *const u8,
        scheme_len: size_t,
        authority: *const u8,
        authority_len: size_t,
        path_and_query: *const u8,
        path_and_query_len: size_t
    ) -> fluxio_code {
        let mut builder = Uri::builder();
        if !scheme.is_null() {
            let scheme_bytes = unsafe {
                std::slice::from_raw_parts(scheme, scheme_len as usize)
            };
            builder = builder.scheme(scheme_bytes);
        }
        if !authority.is_null() {
            let authority_bytes = unsafe {
                std::slice::from_raw_parts(authority, authority_len as usize)
            };
            builder = builder.authority(authority_bytes);
        }
        if !path_and_query.is_null() {
            let path_and_query_bytes = unsafe {
                std::slice::from_raw_parts(path_and_query, path_and_query_len as usize)
            };
            builder = builder.path_and_query(path_and_query_bytes);
        }
        match builder.build() {
            Ok(u) => {
                *unsafe { &mut *req }.0.uri_mut() = u;
                fluxio_code::FLUXIO_OK
            },
            Err(_) => {
                fluxio_code::FLUXIO_INVALID_ARG
            }
        }
    }
}

ffi_fn! {
    /// Set the preferred HTTP version of the request.
    ///
    /// The version value should be one of the `FLUXIO_HTTP_VERSION_` constants.
    ///
    /// Note that this won't change the major HTTP version of the connection,
    /// since that is determined at the handshake step.
    fn fluxio_request_set_version(req: *mut fluxio_request, version: c_int) -> fluxio_code {
        use http::Version;

        let req = non_null!(&mut *req ?= fluxio_code::FLUXIO_INVALID_ARG);
        *req.0.version_mut() = match version {
            super::FLUXIO_HTTP_VERSION_NONE => Version::HTTP_11,
            super::FLUXIO_HTTP_VERSION_1_0 => Version::HTTP_10,
            super::FLUXIO_HTTP_VERSION_1_1 => Version::HTTP_11,
            super::FLUXIO_HTTP_VERSION_2 => Version::HTTP_2,
            _ => {
                // We don't know this version
                return fluxio_code::FLUXIO_INVALID_ARG;
            }
        };
        fluxio_code::FLUXIO_OK
    }
}

ffi_fn! {
    /// Gets a reference to the HTTP headers of this request
    ///
    /// This is not an owned reference, so it should not be accessed after the
    /// `fluxio_request` has been consumed.
    fn fluxio_request_headers(req: *mut fluxio_request) -> *mut fluxio_headers {
        fluxio_headers::get_or_default(unsafe { &mut *req }.0.extensions_mut())
    } ?= std::ptr::null_mut()
}

ffi_fn! {
    /// Set the body of the request.
    ///
    /// The default is an empty body.
    ///
    /// This takes ownership of the `fluxio_body *`, you must not use it or
    /// free it after setting it on the request.
    fn fluxio_request_set_body(req: *mut fluxio_request, body: *mut fluxio_body) -> fluxio_code {
        let body = non_null!(Box::from_raw(body) ?= fluxio_code::FLUXIO_INVALID_ARG);
        let req = non_null!(&mut *req ?= fluxio_code::FLUXIO_INVALID_ARG);
        *req.0.body_mut() = body.0;
        fluxio_code::FLUXIO_OK
    }
}

ffi_fn! {
    /// Set an informational (1xx) response callback.
    ///
    /// The callback is called each time fluxio receives an informational (1xx)
    /// response for this request.
    ///
    /// The third argument is an opaque user data pointer, which is passed to
    /// the callback each time.
    ///
    /// The callback is passed the `void *` data pointer, and a
    /// `fluxio_response *` which can be inspected as any other response. The
    /// body of the response will always be empty.
    ///
    /// NOTE: The `fluxio_response *` is just borrowed data, and will not
    /// be valid after the callback finishes. You must copy any data you wish
    /// to persist.
    fn fluxio_request_on_informational(req: *mut fluxio_request, callback: fluxio_request_on_informational_callback, data: *mut c_void) -> fluxio_code {
        let ext = OnInformational {
            func: callback,
            data: UserDataPointer(data),
        };
        let req = non_null!(&mut *req ?= fluxio_code::FLUXIO_INVALID_ARG);
        req.0.extensions_mut().insert(ext);
        fluxio_code::FLUXIO_OK
    }
}

impl fluxio_request {
    pub(super) fn finalize_request(&mut self) {
        if let Some(headers) = self.0.extensions_mut().remove::<fluxio_headers>() {
            *self.0.headers_mut() = headers.headers;
            self.0.extensions_mut().insert(headers.orig_casing);
            self.0.extensions_mut().insert(headers.orig_order);
        }
    }
}

// ===== impl fluxio_response =====

ffi_fn! {
    /// Free an HTTP response after using it.
    fn fluxio_response_free(resp: *mut fluxio_response) {
        drop(non_null!(Box::from_raw(resp) ?= ()));
    }
}

ffi_fn! {
    /// Get the HTTP-Status code of this response.
    ///
    /// It will always be within the range of 100-599.
    fn fluxio_response_status(resp: *const fluxio_response) -> u16 {
        non_null!(&*resp ?= 0).0.status().as_u16()
    }
}

ffi_fn! {
    /// Get a pointer to the reason-phrase of this response.
    ///
    /// This buffer is not null-terminated.
    ///
    /// This buffer is owned by the response, and should not be used after
    /// the response has been freed.
    ///
    /// Use `fluxio_response_reason_phrase_len()` to get the length of this
    /// buffer.
    fn fluxio_response_reason_phrase(resp: *const fluxio_response) -> *const u8 {
        non_null!(&*resp ?= std::ptr::null()).reason_phrase().as_ptr()
    } ?= std::ptr::null()
}

ffi_fn! {
    /// Get the length of the reason-phrase of this response.
    ///
    /// Use `fluxio_response_reason_phrase()` to get the buffer pointer.
    fn fluxio_response_reason_phrase_len(resp: *const fluxio_response) -> size_t {
        non_null!(&*resp ?= 0).reason_phrase().len()
    }
}

ffi_fn! {
    /// Get a reference to the full raw headers of this response.
    ///
    /// You must have enabled `fluxio_clientconn_options_headers_raw()`, or this
    /// will return NULL.
    ///
    /// The returned `fluxio_buf *` is just a reference, owned by the response.
    /// You need to make a copy if you wish to use it after freeing the
    /// response.
    ///
    /// The buffer is not null-terminated, see the `fluxio_buf` functions for
    /// getting the bytes and length.
    fn fluxio_response_headers_raw(resp: *const fluxio_response) -> *const fluxio_buf {
        let resp = non_null!(&*resp ?= std::ptr::null());
        match resp.0.extensions().get::<RawHeaders>() {
            Some(raw) => &raw.0,
            None => std::ptr::null(),
        }
    } ?= std::ptr::null()
}

ffi_fn! {
    /// Get the HTTP version used by this response.
    ///
    /// The returned value could be:
    ///
    /// - `FLUXIO_HTTP_VERSION_1_0`
    /// - `FLUXIO_HTTP_VERSION_1_1`
    /// - `FLUXIO_HTTP_VERSION_2`
    /// - `FLUXIO_HTTP_VERSION_NONE` if newer (or older).
    fn fluxio_response_version(resp: *const fluxio_response) -> c_int {
        use http::Version;

        match non_null!(&*resp ?= 0).0.version() {
            Version::HTTP_10 => super::FLUXIO_HTTP_VERSION_1_0,
            Version::HTTP_11 => super::FLUXIO_HTTP_VERSION_1_1,
            Version::HTTP_2 => super::FLUXIO_HTTP_VERSION_2,
            _ => super::FLUXIO_HTTP_VERSION_NONE,
        }
    }
}

ffi_fn! {
    /// Gets a reference to the HTTP headers of this response.
    ///
    /// This is not an owned reference, so it should not be accessed after the
    /// `fluxio_response` has been freed.
    fn fluxio_response_headers(resp: *mut fluxio_response) -> *mut fluxio_headers {
        fluxio_headers::get_or_default(unsafe { &mut *resp }.0.extensions_mut())
    } ?= std::ptr::null_mut()
}

ffi_fn! {
    /// Take ownership of the body of this response.
    ///
    /// It is safe to free the response even after taking ownership of its body.
    fn fluxio_response_body(resp: *mut fluxio_response) -> *mut fluxio_body {
        let body = std::mem::take(non_null!(&mut *resp ?= std::ptr::null_mut()).0.body_mut());
        Box::into_raw(Box::new(fluxio_body(body)))
    } ?= std::ptr::null_mut()
}

impl fluxio_response {
    pub(super) fn wrap(mut resp: Response<Body>) -> fluxio_response {
        let headers = std::mem::take(resp.headers_mut());
        let orig_casing = resp
            .extensions_mut()
            .remove::<HeaderCaseMap>()
            .unwrap_or_else(HeaderCaseMap::default);
        let orig_order = resp
            .extensions_mut()
            .remove::<OriginalHeaderOrder>()
            .unwrap_or_else(OriginalHeaderOrder::default);
        resp.extensions_mut().insert(fluxio_headers {
            headers,
            orig_casing,
            orig_order,
        });

        fluxio_response(resp)
    }

    fn reason_phrase(&self) -> &[u8] {
        if let Some(reason) = self.0.extensions().get::<ReasonPhrase>() {
            return &reason.0;
        }

        if let Some(reason) = self.0.status().canonical_reason() {
            return reason.as_bytes();
        }

        &[]
    }
}

unsafe impl AsTaskType for fluxio_response {
    fn as_task_type(&self) -> fluxio_task_return_type {
        fluxio_task_return_type::FLUXIO_TASK_RESPONSE
    }
}

// ===== impl Headers =====

type fluxio_headers_foreach_callback =
    extern "C" fn(*mut c_void, *const u8, size_t, *const u8, size_t) -> c_int;

impl fluxio_headers {
    pub(super) fn get_or_default(ext: &mut http::Extensions) -> &mut fluxio_headers {
        if let None = ext.get_mut::<fluxio_headers>() {
            ext.insert(fluxio_headers::default());
        }

        ext.get_mut::<fluxio_headers>().unwrap()
    }
}

ffi_fn! {
    /// Iterates the headers passing each name and value pair to the callback.
    ///
    /// The `userdata` pointer is also passed to the callback.
    ///
    /// The callback should return `FLUXIO_ITER_CONTINUE` to keep iterating, or
    /// `FLUXIO_ITER_BREAK` to stop.
    fn fluxio_headers_foreach(headers: *const fluxio_headers, func: fluxio_headers_foreach_callback, userdata: *mut c_void) {
        let headers = non_null!(&*headers ?= ());
        // For each header name/value pair, there may be a value in the casemap
        // that corresponds to the HeaderValue. So, we iterator all the keys,
        // and for each one, try to pair the originally cased name with the value.
        //
        // TODO: consider adding http::HeaderMap::entries() iterator
        let mut ordered_iter =  headers.orig_order.get_in_order().peekable();
        if ordered_iter.peek().is_some() {
            for (name, idx) in ordered_iter {
                let (name_ptr, name_len) = if let Some(orig_name) = headers.orig_casing.get_all(name).nth(*idx) {
                    (orig_name.as_ref().as_ptr(), orig_name.as_ref().len())
                } else {
                    (
                    name.as_str().as_bytes().as_ptr(),
                    name.as_str().as_bytes().len(),
                    )
                };

                let val_ptr;
                let val_len;
                if let Some(value) = headers.headers.get_all(name).iter().nth(*idx) {
                    val_ptr = value.as_bytes().as_ptr();
                    val_len = value.as_bytes().len();
                } else {
                    // Stop iterating, something has gone wrong.
                    return;
                }

                if FLUXIO_ITER_CONTINUE != func(userdata, name_ptr, name_len, val_ptr, val_len) {
                    return;
                }
            }
        } else {
            for name in headers.headers.keys() {
                let mut names = headers.orig_casing.get_all(name);

                for value in headers.headers.get_all(name) {
                    let (name_ptr, name_len) = if let Some(orig_name) = names.next() {
                        (orig_name.as_ref().as_ptr(), orig_name.as_ref().len())
                    } else {
                        (
                            name.as_str().as_bytes().as_ptr(),
                            name.as_str().as_bytes().len(),
                        )
                    };

                    let val_ptr = value.as_bytes().as_ptr();
                    let val_len = value.as_bytes().len();

                    if FLUXIO_ITER_CONTINUE != func(userdata, name_ptr, name_len, val_ptr, val_len) {
                        return;
                    }
                }
            }
        }
    }
}

ffi_fn! {
    /// Sets the header with the provided name to the provided value.
    ///
    /// This overwrites any previous value set for the header.
    fn fluxio_headers_set(headers: *mut fluxio_headers, name: *const u8, name_len: size_t, value: *const u8, value_len: size_t) -> fluxio_code {
        let headers = non_null!(&mut *headers ?= fluxio_code::FLUXIO_INVALID_ARG);
        match unsafe { raw_name_value(name, name_len, value, value_len) } {
            Ok((name, value, orig_name)) => {
                headers.headers.insert(&name, value);
                headers.orig_casing.insert(name.clone(), orig_name.clone());
                headers.orig_order.insert(name);
                fluxio_code::FLUXIO_OK
            }
            Err(code) => code,
        }
    }
}

ffi_fn! {
    /// Adds the provided value to the list of the provided name.
    ///
    /// If there were already existing values for the name, this will append the
    /// new value to the internal list.
    fn fluxio_headers_add(headers: *mut fluxio_headers, name: *const u8, name_len: size_t, value: *const u8, value_len: size_t) -> fluxio_code {
        let headers = non_null!(&mut *headers ?= fluxio_code::FLUXIO_INVALID_ARG);

        match unsafe { raw_name_value(name, name_len, value, value_len) } {
            Ok((name, value, orig_name)) => {
                headers.headers.append(&name, value);
                headers.orig_casing.append(&name, orig_name.clone());
                headers.orig_order.append(name);
                fluxio_code::FLUXIO_OK
            }
            Err(code) => code,
        }
    }
}

impl Default for fluxio_headers {
    fn default() -> Self {
        Self {
            headers: Default::default(),
            orig_casing: HeaderCaseMap::default(),
            orig_order: OriginalHeaderOrder::default(),
        }
    }
}

unsafe fn raw_name_value(
    name: *const u8,
    name_len: size_t,
    value: *const u8,
    value_len: size_t,
) -> Result<(HeaderName, HeaderValue, Bytes), fluxio_code> {
    let name = std::slice::from_raw_parts(name, name_len);
    let orig_name = Bytes::copy_from_slice(name);
    let name = match HeaderName::from_bytes(name) {
        Ok(name) => name,
        Err(_) => return Err(fluxio_code::FLUXIO_INVALID_ARG),
    };
    let value = std::slice::from_raw_parts(value, value_len);
    let value = match HeaderValue::from_bytes(value) {
        Ok(val) => val,
        Err(_) => return Err(fluxio_code::FLUXIO_INVALID_ARG),
    };

    Ok((name, value, orig_name))
}

// ===== impl OnInformational =====

impl OnInformational {
    pub(crate) fn call(&mut self, resp: Response<Body>) {
        let mut resp = fluxio_response::wrap(resp);
        (self.func)(self.data.0, &mut resp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headers_foreach_cases_preserved() {
        let mut headers = fluxio_headers::default();

        let name1 = b"Set-CookiE";
        let value1 = b"a=b";
        fluxio_headers_add(
            &mut headers,
            name1.as_ptr(),
            name1.len(),
            value1.as_ptr(),
            value1.len(),
        );

        let name2 = b"SET-COOKIE";
        let value2 = b"c=d";
        fluxio_headers_add(
            &mut headers,
            name2.as_ptr(),
            name2.len(),
            value2.as_ptr(),
            value2.len(),
        );

        let mut vec = Vec::<u8>::new();
        fluxio_headers_foreach(&headers, concat, &mut vec as *mut _ as *mut c_void);

        assert_eq!(vec, b"Set-CookiE: a=b\r\nSET-COOKIE: c=d\r\n");

        extern "C" fn concat(
            vec: *mut c_void,
            name: *const u8,
            name_len: usize,
            value: *const u8,
            value_len: usize,
        ) -> c_int {
            unsafe {
                let vec = &mut *(vec as *mut Vec<u8>);
                let name = std::slice::from_raw_parts(name, name_len);
                let value = std::slice::from_raw_parts(value, value_len);
                vec.extend(name);
                vec.extend(b": ");
                vec.extend(value);
                vec.extend(b"\r\n");
            }
            FLUXIO_ITER_CONTINUE
        }
    }

    #[cfg(all(feature = "http1", feature = "ffi"))]
    #[test]
    fn test_headers_foreach_order_preserved() {
        let mut headers = fluxio_headers::default();

        let name1 = b"Set-CookiE";
        let value1 = b"a=b";
        fluxio_headers_add(
            &mut headers,
            name1.as_ptr(),
            name1.len(),
            value1.as_ptr(),
            value1.len(),
        );

        let name2 = b"Content-Encoding";
        let value2 = b"gzip";
        fluxio_headers_add(
            &mut headers,
            name2.as_ptr(),
            name2.len(),
            value2.as_ptr(),
            value2.len(),
        );

        let name3 = b"SET-COOKIE";
        let value3 = b"c=d";
        fluxio_headers_add(
            &mut headers,
            name3.as_ptr(),
            name3.len(),
            value3.as_ptr(),
            value3.len(),
        );

        let mut vec = Vec::<u8>::new();
        fluxio_headers_foreach(&headers, concat, &mut vec as *mut _ as *mut c_void);

        println!("{}", std::str::from_utf8(&vec).unwrap());
        assert_eq!(
            vec,
            b"Set-CookiE: a=b\r\nContent-Encoding: gzip\r\nSET-COOKIE: c=d\r\n"
        );

        extern "C" fn concat(
            vec: *mut c_void,
            name: *const u8,
            name_len: usize,
            value: *const u8,
            value_len: usize,
        ) -> c_int {
            unsafe {
                let vec = &mut *(vec as *mut Vec<u8>);
                let name = std::slice::from_raw_parts(name, name_len);
                let value = std::slice::from_raw_parts(value, value_len);
                vec.extend(name);
                vec.extend(b": ");
                vec.extend(value);
                vec.extend(b"\r\n");
            }
            FLUXIO_ITER_CONTINUE
        }
    }
}
