use crate::ffi::*;

use std::{ptr::NonNull, slice};

/// The `Buffer` trait provides methods for working with an nginx buffer (`ngx_buf_t`).
pub trait Buffer {
    /// Returns a raw pointer to the underlying `ngx_buf_t` of the buffer.
    fn as_ngx_buf(&self) -> *const ngx_buf_t;

    /// Returns a mutable raw pointer to the underlying `ngx_buf_t` of the buffer.
    fn as_ngx_buf_mut(&mut self) -> *mut ngx_buf_t;

    /// Returns the buffer contents as a byte slice.
    ///
    /// # Safety
    /// This function is marked as unsafe because it involves raw pointer manipulation.
    fn as_bytes(&self) -> &[u8] {
        let buf = self.as_ngx_buf();
        unsafe { slice::from_raw_parts((*buf).pos, self.len()) }
    }

    /// Returns the length of the buffer contents.
    ///
    /// # Safety
    /// This function is marked as unsafe because it involves raw pointer manipulation.
    fn len(&self) -> usize {
        let buf = self.as_ngx_buf();
        unsafe {
            let pos = (*buf).pos;
            let last = (*buf).last;
            assert!(last >= pos);
            usize::wrapping_sub(last as _, pos as _)
        }
    }

    /// Returns `true` if the buffer is empty, i.e., it has zero length.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sets the `last_buf` flag of the buffer.
    ///
    /// # Arguments
    ///
    /// * `last` - A boolean indicating whether the buffer is the last buffer in a request.
    fn set_last_buf(&mut self, last: bool) {
        let buf = self.as_ngx_buf_mut();
        unsafe {
            (*buf).set_last_buf(if last { 1 } else { 0 });
        }
    }

    /// Sets the `last_in_chain` flag of the buffer.
    ///
    /// # Arguments
    ///
    /// * `last` - A boolean indicating whether the buffer is the last buffer in a chain of buffers.
    fn set_last_in_chain(&mut self, last: bool) {
        let buf = self.as_ngx_buf_mut();
        unsafe {
            (*buf).set_last_in_chain(if last { 1 } else { 0 });
        }
    }
}

/// The `MutableBuffer` trait extends the `Buffer` trait and provides methods for working with a mutable buffer.
pub trait MutableBuffer: Buffer {
    /// Returns a mutable reference to the buffer contents as a byte slice.
    ///
    /// # Safety
    /// This function is marked as unsafe because it involves raw pointer manipulation.
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        let buf = self.as_ngx_buf_mut();
        unsafe { slice::from_raw_parts_mut((*buf).pos, self.len()) }
    }
}

/// Wrapper struct for a temporary buffer, providing methods for working with an `ngx_buf_t`.
pub struct TemporaryBuffer(NonNull<ngx_buf_t>);

impl TemporaryBuffer {
    /// Creates a new `TemporaryBuffer` from an `ngx_buf_t` pointer.
    ///
    /// # Panics
    /// Panics if the given buffer pointer is null.
    pub fn from_ngx_buf(buf: NonNull<ngx_buf_t>) -> TemporaryBuffer {
        TemporaryBuffer(buf)
    }
}

impl Buffer for TemporaryBuffer {
    /// Returns the underlying `ngx_buf_t` pointer as a raw pointer.
    fn as_ngx_buf(&self) -> *const ngx_buf_t {
        self.0.as_ptr()
    }

    /// Returns a mutable reference to the underlying `ngx_buf_t` pointer.
    fn as_ngx_buf_mut(&mut self) -> *mut ngx_buf_t {
        self.0.as_ptr()
    }
}

impl MutableBuffer for TemporaryBuffer {
    /// Returns a mutable reference to the buffer contents as a byte slice.
    ///
    /// # Safety
    /// This function is marked as unsafe because it involves raw pointer manipulation.
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut((*self.0.as_ptr()).pos, self.len()) }
    }
}

/// Wrapper struct for a memory buffer, providing methods for working with an `ngx_buf_t`.
pub struct MemoryBuffer(NonNull<ngx_buf_t>);

impl MemoryBuffer {
    /// Creates a new `MemoryBuffer` from an `ngx_buf_t` pointer.
    ///
    /// # Panics
    /// Panics if the given buffer pointer is null.
    pub fn from_ngx_buf(buf: NonNull<ngx_buf_t>) -> MemoryBuffer {
        MemoryBuffer(buf)
    }
}

impl Buffer for MemoryBuffer {
    /// Returns the underlying `ngx_buf_t` pointer as a raw pointer.
    fn as_ngx_buf(&self) -> *const ngx_buf_t {
        self.0.as_ptr()
    }

    /// Returns a mutable reference to the underlying `ngx_buf_t` pointer.
    fn as_ngx_buf_mut(&mut self) -> *mut ngx_buf_t {
        self.0.as_ptr()
    }
}
