#![allow(missing_docs)]

use std::{
    ffi::{c_char, c_void},
    marker::PhantomData,
    ptr::NonNull,
};

use nginx_sys::{ngx_conf_t, ngx_int_t};

use crate::core::*;

use super::{HTTPModule, Merge, MergeConfigError};

pub struct NgxConf<'a> {
    inner: *mut ngx_conf_t,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> NgxConf<'a> {
    /// # SAFETY
    /// The lifetime of `Self` must correspond to the
    /// lifetime of `ngx_conf_t`
    pub unsafe fn new(inner: *mut ngx_conf_t) -> Option<Self> {
        if inner.is_null() || !inner.is_aligned() {
            return None;
        }

        Some(Self {
            inner,
            _phantom: Default::default(),
        })
    }

    pub fn allocate<T>(&self, value: T) -> Option<&'a mut T> {
        let mut pool = unsafe { Pool::from_ngx_pool((*self.inner).pool) };
        pool.allocate(value).map(|mut v| unsafe { v.as_mut() })
    }
}

pub enum Error {
    Error,
    Again,
    Busy,
    Declined,
    Abort,
}

impl From<Error> for Status {
    fn from(value: Error) -> Self {
        match value {
            Error::Error => Status::NGX_ERROR,
            Error::Again => Status::NGX_AGAIN,
            Error::Busy => Status::NGX_BUSY,
            Error::Declined => Status::NGX_DECLINED,
            Error::Abort => Status::NGX_ABORT,
        }
    }
}

pub trait SafeHttpModule {
    /// Configuration in the `http` block.
    type MainConf: Merge + Default;
    /// Configuration in a `server` block within the `http` block.
    type SrvConf: Merge + Default;
    /// Configuration in a `location` block within the `http` block.
    type LocConf: Merge + Default;

    fn preconfiguration(cf: &mut ngx_conf_t) -> Result<(), Error> {
        Ok(())
    }

    fn postconfiguration(cf: &mut ngx_conf_t) -> Result<(), Error> {
        Ok(())
    }

    fn create_main_conf(cf: &mut ngx_conf_t) -> Option<NonNull<Self::MainConf>> {
        let mut pool = unsafe { Pool::from_ngx_pool((*cf).pool) };
        pool.allocate(Default::default())
    }

    fn init_main_conf(cf: &mut ngx_conf_t, conf: &mut Self::MainConf) -> Result<(), ()> {
        Ok(())
    }

    fn create_srv_conf<'a>(cf: &mut ngx_conf_t) -> Option<NonNull<Self::SrvConf>> {
        let mut pool = unsafe { Pool::from_ngx_pool((*cf).pool) };
        pool.allocate(Default::default())
    }

    fn merge_srv_conf(
        cf: &mut ngx_conf_t,
        prev: &mut Self::SrvConf,
        conf: &mut Self::SrvConf,
    ) -> Result<(), MergeConfigError> {
        conf.merge(prev)
    }

    fn create_loc_conf(cf: &mut ngx_conf_t) -> Option<NonNull<Self::LocConf>> {
        let mut pool = unsafe { Pool::from_ngx_pool((*cf).pool) };
        pool.allocate(Default::default())
    }

    fn merge_loc_conf(
        cf: &mut ngx_conf_t,
        prev: &mut Self::LocConf,
        conf: &mut Self::LocConf,
    ) -> Result<(), MergeConfigError> {
        prev.merge(conf)
    }
}

impl<T> HTTPModule for T
where
    T: SafeHttpModule,
{
    type MainConf = <Self as SafeHttpModule>::MainConf;

    type SrvConf = <Self as SafeHttpModule>::SrvConf;

    type LocConf = <Self as SafeHttpModule>::LocConf;

    unsafe extern "C" fn preconfiguration(cf: *mut ngx_conf_t) -> ngx_int_t {
        let cf = if let Some(cf) = cf.as_mut() {
            cf
        } else {
            return Status::NGX_ERROR.into();
        };

        let status: Status = match <Self as SafeHttpModule>::preconfiguration(cf) {
            Ok(()) => Status::NGX_OK,
            Err(e) => e.into(),
        };

        status.into()
    }

    unsafe extern "C" fn postconfiguration(cf: *mut ngx_conf_t) -> ngx_int_t {
        let cf = if let Some(cf) = cf.as_mut() {
            cf
        } else {
            return Status::NGX_ERROR.into();
        };

        let status: Status = match <Self as SafeHttpModule>::postconfiguration(cf) {
            Ok(()) => Status::NGX_OK,
            Err(e) => e.into(),
        };

        status.into()
    }

    unsafe extern "C" fn create_main_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let cf = if let Some(cf) = cf.as_mut() {
            cf
        } else {
            return std::ptr::null_mut() as _;
        };

        let pointer = <Self as SafeHttpModule>::create_main_conf(cf);

        pointer.map(|v| v.as_ptr()).unwrap_or(std::ptr::null_mut()) as _
    }

    unsafe extern "C" fn init_main_conf(cf: *mut ngx_conf_t, conf: *mut c_void) -> *mut c_char {
        let cf = if let Some(cf) = cf.as_mut() {
            cf
        } else {
            return NGX_CONF_ERROR as _;
        };

        let conf = if let Some(conf) = (conf as *mut <Self as SafeHttpModule>::MainConf).as_mut() {
            conf
        } else {
            return NGX_CONF_ERROR as _;
        };

        match <Self as SafeHttpModule>::init_main_conf(cf, conf) {
            Ok(_) => NGX_CONF_OK as _,
            Err(_) => NGX_CONF_ERROR as _,
        }
    }

    unsafe extern "C" fn create_srv_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let cf = if let Some(cf) = cf.as_mut() {
            cf
        } else {
            return std::ptr::null_mut() as _;
        };

        let pointer = <Self as SafeHttpModule>::create_srv_conf(cf);

        pointer.map(|v| v.as_ptr()).unwrap_or(std::ptr::null_mut()) as _
    }

    unsafe extern "C" fn merge_srv_conf(cf: *mut ngx_conf_t, prev: *mut c_void, conf: *mut c_void) -> *mut c_char {
        let cf = if let Some(cf) = cf.as_mut() {
            cf
        } else {
            return NGX_CONF_ERROR as _;
        };

        let prev = if let Some(prev) = (prev as *mut Self::SrvConf).as_mut() {
            prev
        } else {
            return NGX_CONF_ERROR as _;
        };

        let conf = if let Some(conf) = (conf as *mut Self::SrvConf).as_mut() {
            conf
        } else {
            return NGX_CONF_ERROR as _;
        };

        match <Self as SafeHttpModule>::merge_srv_conf(cf, prev, conf) {
            Ok(_) => NGX_CONF_OK as _,
            Err(_) => NGX_CONF_ERROR as _,
        }
    }

    unsafe extern "C" fn create_loc_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let cf = if let Some(cf) = cf.as_mut() {
            cf
        } else {
            return std::ptr::null_mut() as _;
        };

        let pointer = <Self as SafeHttpModule>::create_loc_conf(cf);

        pointer.map(|v| v.as_ptr()).unwrap_or(std::ptr::null_mut()) as _
    }

    unsafe extern "C" fn merge_loc_conf(cf: *mut ngx_conf_t, prev: *mut c_void, conf: *mut c_void) -> *mut c_char {
        let cf = if let Some(cf) = cf.as_mut() {
            cf
        } else {
            return NGX_CONF_ERROR as _;
        };

        let prev = if let Some(prev) = (prev as *mut Self::LocConf).as_mut() {
            prev
        } else {
            return NGX_CONF_ERROR as _;
        };

        let conf = if let Some(conf) = (conf as *mut Self::LocConf).as_mut() {
            conf
        } else {
            return NGX_CONF_ERROR as _;
        };

        match <Self as SafeHttpModule>::merge_loc_conf(cf, prev, conf) {
            Ok(_) => NGX_CONF_OK as _,
            Err(_) => NGX_CONF_ERROR as _,
        }
    }
}
