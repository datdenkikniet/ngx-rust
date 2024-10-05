use std::{marker::PhantomData, ptr::NonNull};

use nginx_sys::{ngx_array_push, ngx_array_t};

/// An nginx array.
///
/// `T` should be limited to non-[`Drop`] types as there
/// is no way to explicitly drop values in the array.
pub struct Array<'a, T> {
    array: NonNull<ngx_array_t>,
    _phantom: PhantomData<&'a T>,
}

impl<'a, T> Array<'a, T> {
    /// Create a new [`Array`] from a raw pointer.
    ///
    /// If `T` has drop logic, pushing to the array created from
    /// this pointer will leak memory, as [`Drop`] is not ran
    /// for any elements.
    ///
    /// # SAFETY
    /// The pointer must provide exclusive access to the underlying
    /// `ngx_array_t` for the lifetime of the created [`Array`].
    pub unsafe fn new(array: NonNull<ngx_array_t>) -> Self {
        Self {
            array,
            _phantom: Default::default(),
        }
    }

    /// Try to push a new value to the array.
    pub fn push(&mut self, value: T) -> Result<(), ()> {
        let new_value_ptr = unsafe { ngx_array_push(self.array.as_ptr()) };

        if new_value_ptr.is_null() {
            return Err(());
        }

        unsafe { std::ptr::write(new_value_ptr as _, value) };
        Ok(())
    }
}

impl<T> core::ops::Deref for Array<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        let value = unsafe { *self.array.as_ptr() };

        let start = value.elts as *const T;
        let len = value.nelts;

        unsafe { std::slice::from_raw_parts(start, len) }
    }
}

impl<T> core::ops::DerefMut for Array<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let value = unsafe { *self.array.as_ptr() };

        let start = value.elts as *mut T;
        let len = value.nelts;

        unsafe { std::slice::from_raw_parts_mut(start, len) }
    }
}
