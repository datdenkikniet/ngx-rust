use crate::core::buffer::{Buffer, MemoryBuffer, TemporaryBuffer};
use crate::ffi::*;

use std::mem::MaybeUninit;
use std::os::raw::c_void;
use std::ptr::NonNull;
use std::{mem, ptr};

/// Wrapper struct for an `ngx_pool_t` pointer, providing methods for working with memory pools.
pub struct Pool(*mut ngx_pool_t);

impl Pool {
    fn alloc(&mut self, size: usize) -> *mut c_void {
        unsafe { ngx_palloc(self.0, size) }
    }

    /// Creates a new `Pool` from an `ngx_pool_t` pointer.
    ///
    /// # Safety
    /// The caller must ensure that a valid `ngx_pool_t` pointer is provided, pointing to valid memory and non-null.
    /// A null argument will cause an assertion failure and panic.
    pub unsafe fn from_ngx_pool(pool: *mut ngx_pool_t) -> Pool {
        assert!(!pool.is_null());
        Pool(pool)
    }

    /// Creates a buffer of the specified size in the memory pool.
    ///
    /// Returns `Some(TemporaryBuffer)` if the buffer is successfully created, or `None` if allocation fails.
    pub fn create_buffer(&mut self, size: usize) -> Option<TemporaryBuffer> {
        let buf = unsafe { ngx_create_temp_buf(self.0, size) };
        let buf = NonNull::new(buf)?;
        Some(TemporaryBuffer::from_ngx_buf(buf))
    }

    /// Creates a buffer from a string in the memory pool.
    ///
    /// Returns `Some(TemporaryBuffer)` if the buffer is successfully created, or `None` if allocation fails.
    pub fn create_buffer_from_str(&mut self, str: &str) -> Option<TemporaryBuffer> {
        let mut buffer = self.create_buffer(str.len())?;
        unsafe {
            let buf = buffer.as_ngx_buf_mut();
            ptr::copy_nonoverlapping(str.as_ptr(), (*buf).pos, str.len());
            (*buf).last = (*buf).pos.add(str.len());
        }
        Some(buffer)
    }

    /// Creates a buffer from a static string in the memory pool.
    ///
    /// Returns `Some(MemoryBuffer)` if the buffer is successfully created, or `None` if allocation fails.
    pub fn create_buffer_from_static_str(&mut self, str: &'static str) -> Option<MemoryBuffer> {
        // We cast away const, but buffers with the memory flag are read-only
        let start = str.as_ptr() as *mut u8;
        let end = unsafe { start.add(str.len()) };

        let buf = self.allocate(ngx_buf_t {
            start,
            pos: start,
            last: end,
            end,
            file_pos: 0,
            file_last: 0,
            tag: ptr::null_mut(),
            file: ptr::null_mut(),
            shadow: ptr::null_mut(),
            _bitfield_1: Default::default(),
            _bitfield_align_1: Default::default(),
            num: 0,
        })?;

        // SAFETY: buf is a non-null, aligned pointer of type ngx_buf_t.
        unsafe { *buf.as_ptr() }.set_memory(1);

        Some(MemoryBuffer::from_ngx_buf(buf))
    }

    /// Adds a cleanup handler for a value in the memory pool.
    ///
    /// Returns `Ok(())` if the cleanup handler is successfully added, or `Err(())` if the cleanup handler cannot be added.
    ///
    /// # Safety
    /// This function is marked as unsafe because it involves raw pointer manipulation.
    fn add_cleanup_for_value<T>(&mut self, value: NonNull<T>) -> Result<(), ()> {
        let cln = unsafe { ngx_pool_cleanup_add(self.0, 0) };
        if cln.is_null() {
            return Err(());
        }

        unsafe {
            *cln = ngx_pool_cleanup_s {
                handler: Some(cleanup_type::<T>),
                data: value.as_ptr() as _,
                next: ptr::null_mut() as _,
            };
        }

        Ok(())
    }

    /// Allocates memory for a value of a specified type and zeroes it. This does _not_ add a cleanup handler.
    ///
    /// Returns `Some` on success, else `None`.
    ///
    /// The pointer is valid as long as the pool backing this [`Pool`] exists.
    pub fn allocate_uninit_zeroed<T>(&mut self) -> Option<NonNull<MaybeUninit<T>>> {
        if std::mem::size_of::<T>() == 0 {
            return Some(NonNull::dangling());
        }

        let p = unsafe { ngx_pcalloc(self.0, mem::size_of::<T>()) } as *mut MaybeUninit<T>;
        NonNull::new(p)
    }

    /// Allocates memory for a value of a specified type and adds a cleanup handler to the memory pool.
    ///
    /// Returns `Some` on success, else `None`.
    ///
    /// The pointer is valid as long as the pool backing this [`Pool`] exists.
    pub fn allocate<T>(&mut self, value: T) -> Option<NonNull<T>> {
        if std::mem::size_of::<T>() == 0 {
            return Some(NonNull::dangling());
        }

        let p = self.alloc(mem::size_of::<T>()) as _;
        let p = NonNull::new(p)?;

        unsafe {
            ptr::write(p.as_ptr(), value);
            if self.add_cleanup_for_value(p).is_err() {
                ptr::drop_in_place(p.as_ptr());
                return None;
            };
        }

        Some(p)
    }

    /// Allocate a memory region of size `len`.
    ///
    /// The pointer is valid as long as the pool backing this [`Pool`] exists.
    pub fn allocate_raw(&mut self, len: usize) -> Option<NonNull<u8>> {
        NonNull::new(self.alloc(len) as _)
    }
}

/// Cleanup handler for a specific type `T`.
///
/// This function is called when cleaning up a value of type `T` in an FFI context.
///
/// # Safety
/// This function is marked as unsafe due to the raw pointer manipulation and the assumption that `data` is a valid pointer to `T`.
///
/// # Arguments
///
/// * `data` - A raw pointer to the value of type `T` to be cleaned up.
unsafe extern "C" fn cleanup_type<T>(data: *mut c_void) {
    ptr::drop_in_place(data as *mut T);
}
