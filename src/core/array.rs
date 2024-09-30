use std::marker::PhantomData;

use nginx_sys::{ngx_array_push, ngx_array_t};

/// An nginx array.
///
/// `T` should be limited to non-[`Drop`] types as there
/// is no way to explicitly drop values in the array.
pub struct Array<'a, T> {
    array: *mut ngx_array_t,
    _phantom: PhantomData<&'a T>,
}

impl<'a, T> Array<'a, T> {
    /// Create a new [`NgxArray`] from a raw pointer.
    ///
    /// If `T` has drop logic, pushing to the array created from
    /// this pointer will leak memory, as [`Drop`] is not ran
    /// for any elements.
    ///
    /// # SAFETY
    /// The lifetime `'a` of `Self` must not outlive the lifetime
    /// of the passed-in pointer.
    pub unsafe fn new_raw(array: *mut ngx_array_t) -> Option<Self> {
        let array = array.as_mut()?;
        Some(Self::new(array))
    }

    /// Create a new wrapper around [`ngx_array_t`]
    ///
    /// If `T` has drop logic, pushing to the array created from
    /// this pointer will leak memory, as [`Drop`] is not ran
    /// for any elements.
    pub fn new(array: &'a mut ngx_array_t) -> Self {
        Self {
            array,
            _phantom: Default::default(),
        }
    }

    /// Try to push a new value to the array.
    pub fn push(&mut self, value: T) -> Result<(), ()> {
        let new_value_ptr = unsafe { ngx_array_push(self.array) };

        if new_value_ptr.is_null() {
            return Err(());
        }

        unsafe { std::ptr::write(new_value_ptr as _, value) };
        Ok(())
    }
}

impl<'a, T> core::ops::Deref for Array<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        // SAFETY: `self.array` is a valid pointer.
        let array = unsafe { *self.array };

        let ptr = array.elts as *const T;
        let n_elements = array.nelts;

        // SAFETY: `ptr` points to `n_elements` valid `T`s that
        // are valid for `'a`.
        unsafe { core::slice::from_raw_parts(ptr, n_elements) }
    }
}

impl<'a, T> core::ops::DerefMut for Array<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: `self.array` is a valid pointer.
        let array = unsafe { *self.array };

        let ptr = array.elts as *mut T;
        let n_elements = array.nelts;

        // SAFETY: `ptr` points to `n_elements` valid `T`s that
        // are valid for `'a`.
        unsafe { core::slice::from_raw_parts_mut(ptr, n_elements) }
    }
}
