use crate::core::NGX_CONF_ERROR;
use crate::core::*;
use crate::ffi::*;

use std::os::raw::{c_char, c_void};
use std::ptr::NonNull;

/// MergeConfigError - configuration cannot be merged with levels above.
#[derive(Debug)]
pub enum MergeConfigError {
    /// No value provided for configuration argument
    NoValue,
}

impl std::error::Error for MergeConfigError {}

impl std::fmt::Display for MergeConfigError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MergeConfigError::NoValue => "no value".fmt(fmt),
        }
    }
}

/// The `Merge` trait provides a method for merging configuration down through each level.
///
/// A module configuration should implement this trait for setting its configuration throughout
/// each level.
pub trait Merge {
    /// Module merge function.
    ///
    /// # Returns
    /// Result, Ok on success or MergeConfigError on failure.
    fn merge(&mut self, prev: &Self) -> Result<(), MergeConfigError>;
}

impl Merge for () {
    fn merge(&mut self, _prev: &Self) -> Result<(), MergeConfigError> {
        Ok(())
    }
}

/// The `HTTPModule` trait provides the NGINX configuration stage interface.
///
/// These functions allocate structures, initialize them, and merge through the configuration
/// layers.
///
/// See https://nginx.org/en/docs/dev/development_guide.html#adding_new_modules for details.
pub trait RawHttpModule {
    /// Configuration in the `http` block.
    type MainConf: Merge + Default;
    /// Configuration in a `server` block within the `http` block.
    type SrvConf: Merge + Default;
    /// Configuration in a `location` block within the `http` block.
    type LocConf: Merge + Default;

    /// # Safety
    ///
    /// Callers should provide valid non-null `ngx_conf_t` arguments. Implementers must
    /// guard against null inputs or risk runtime errors.
    unsafe extern "C" fn preconfiguration(_cf: *mut ngx_conf_t) -> ngx_int_t {
        Status::NGX_OK.into()
    }

    /// # Safety
    ///
    /// Callers should provide valid non-null `ngx_conf_t` arguments. Implementers must
    /// guard against null inputs or risk runtime errors.
    unsafe extern "C" fn postconfiguration(_cf: *mut ngx_conf_t) -> ngx_int_t {
        Status::NGX_OK.into()
    }

    /// # Safety
    ///
    /// Callers should provide valid non-null `ngx_conf_t` arguments. Implementers must
    /// guard against null inputs or risk runtime errors.
    unsafe extern "C" fn create_main_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let mut pool = Pool::from_ngx_pool((*cf).pool);

        let pointer = if let Some(non_null) = pool.allocate(Self::MainConf::default()) {
            non_null.as_ptr()
        } else {
            core::ptr::null()
        };

        pointer as _
    }

    /// # Safety
    ///
    /// Callers should provide valid non-null `ngx_conf_t` arguments. Implementers must
    /// guard against null inputs or risk runtime errors.
    unsafe extern "C" fn init_main_conf(_cf: *mut ngx_conf_t, _conf: *mut c_void) -> *mut c_char {
        NGX_CONF_OK as _
    }

    /// # Safety
    ///
    /// Callers should provide valid non-null `ngx_conf_t` arguments. Implementers must
    /// guard against null inputs or risk runtime errors.
    unsafe extern "C" fn create_srv_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let mut pool = Pool::from_ngx_pool((*cf).pool);

        let pointer = if let Some(non_null) = pool.allocate(Self::SrvConf::default()) {
            non_null.as_ptr()
        } else {
            core::ptr::null()
        };

        pointer as _
    }

    /// # Safety
    ///
    /// Callers should provide valid non-null `ngx_conf_t` arguments. Implementers must
    /// guard against null inputs or risk runtime errors.
    unsafe extern "C" fn merge_srv_conf(_cf: *mut ngx_conf_t, prev: *mut c_void, conf: *mut c_void) -> *mut c_char {
        let prev = &mut *(prev as *mut Self::SrvConf);
        let conf = &mut *(conf as *mut Self::SrvConf);
        match conf.merge(prev) {
            Ok(_) => NGX_CONF_OK as _,
            Err(_) => NGX_CONF_ERROR as _,
        }
    }

    /// # Safety
    ///
    /// Callers should provide valid non-null `ngx_conf_t` arguments. Implementers must
    /// guard against null inputs or risk runtime errors.
    unsafe extern "C" fn create_loc_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let mut pool = Pool::from_ngx_pool((*cf).pool);

        let pointer = if let Some(non_null) = pool.allocate(Self::LocConf::default()) {
            non_null.as_ptr()
        } else {
            core::ptr::null()
        };

        pointer as _
    }

    /// # Safety
    ///
    /// Callers should provide valid non-null `ngx_conf_t` arguments. Implementers must
    /// guard against null inputs or risk runtime errors.
    unsafe extern "C" fn merge_loc_conf(_cf: *mut ngx_conf_t, prev: *mut c_void, conf: *mut c_void) -> *mut c_char {
        let prev = &mut *(prev as *mut Self::LocConf);
        let conf = &mut *(conf as *mut Self::LocConf);
        match conf.merge(prev) {
            Ok(_) => NGX_CONF_OK as _,
            Err(_) => NGX_CONF_ERROR as _,
        }
    }
}

#[allow(unused_variables)]
pub trait HttpModule {
    /// Configuration in the `http` block.
    type MainConf: Merge + Default;
    /// Configuration in a `server` block within the `http` block.
    type SrvConf: Merge + Default;
    /// Configuration in a `location` block within the `http` block.
    type LocConf: Merge + Default;

    fn preconfiguration(cf: &mut ngx_conf_t) -> Result<(), ()> {
        Ok(())
    }

    fn postconfiguration(cf: &mut ngx_conf_t) -> Result<(), ()> {
        Ok(())
    }

    fn create_main_conf(cf: &mut ngx_conf_t) -> Result<NonNull<Self::MainConf>, ()> {
        let mut pool = unsafe { Pool::from_ngx_pool(cf.pool) };
        pool.allocate(Self::MainConf::default()).ok_or(())
    }

    fn init_main_conf(cf: &mut ngx_conf_t, conf: &mut Self::MainConf) -> Result<(), ()> {
        Ok(())
    }

    fn create_srv_conf(cf: &mut ngx_conf_t) -> Result<NonNull<Self::SrvConf>, ()> {
        let mut pool = unsafe { Pool::from_ngx_pool(cf.pool) };
        pool.allocate(Self::SrvConf::default()).ok_or(())
    }

    fn merge_srv_conf(cf: &mut ngx_conf_t, prev: &mut Self::SrvConf, conf: &mut Self::SrvConf) -> Result<(), ()> {
        conf.merge(prev).map_err(|_| ())
    }

    fn create_loc_conf(cf: &mut ngx_conf_t) -> Result<NonNull<Self::LocConf>, ()> {
        let mut pool = unsafe { Pool::from_ngx_pool(cf.pool) };
        pool.allocate(Self::LocConf::default()).ok_or(())
    }

    fn merge_loc_conf(cf: &mut ngx_conf_t, prev: &mut Self::LocConf, conf: &mut Self::LocConf) -> Result<(), ()> {
        conf.merge(prev).map_err(|_| ())
    }
}

macro_rules! non_null_mut_or {
    ($value:ident, $error:expr) => {
        if let Some(mut value) = NonNull::new($value as _) {
            unsafe { value.as_mut() }
        } else {
            return $error;
        }
    };
}

impl<T> RawHttpModule for T
where
    T: HttpModule,
{
    type MainConf = <Self as HttpModule>::MainConf;

    type SrvConf = <Self as HttpModule>::SrvConf;

    type LocConf = <Self as HttpModule>::LocConf;

    unsafe extern "C" fn preconfiguration(cf: *mut ngx_conf_t) -> ngx_int_t {
        let cf = non_null_mut_or!(cf, Status::NGX_ERROR.into());

        if <Self as HttpModule>::preconfiguration(cf).is_ok() {
            Status::NGX_OK.into()
        } else {
            Status::NGX_ERROR.into()
        }
    }

    unsafe extern "C" fn postconfiguration(cf: *mut ngx_conf_t) -> ngx_int_t {
        let cf = non_null_mut_or!(cf, Status::NGX_ERROR.into());

        if <Self as HttpModule>::postconfiguration(cf).is_ok() {
            Status::NGX_OK.into()
        } else {
            Status::NGX_ERROR.into()
        }
    }

    unsafe extern "C" fn create_main_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let cf = non_null_mut_or!(cf, std::ptr::null_mut());

        if let Ok(ptr) = <Self as HttpModule>::create_main_conf(cf) {
            ptr.as_ptr() as _
        } else {
            std::ptr::null_mut()
        }
    }

    unsafe extern "C" fn init_main_conf(cf: *mut ngx_conf_t, conf: *mut c_void) -> *mut c_char {
        let cf = non_null_mut_or!(cf, NGX_CONF_ERROR as _);
        let conf = non_null_mut_or!(conf, NGX_CONF_ERROR as _);

        if <Self as HttpModule>::init_main_conf(cf, conf).is_ok() {
            NGX_CONF_OK as _
        } else {
            NGX_CONF_ERROR as _
        }
    }

    unsafe extern "C" fn create_srv_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let cf = non_null_mut_or!(cf, std::ptr::null_mut());

        if let Ok(conf) = <Self as HttpModule>::create_srv_conf(cf) {
            conf.as_ptr() as _
        } else {
            std::ptr::null_mut()
        }
    }

    unsafe extern "C" fn merge_srv_conf(cf: *mut ngx_conf_t, prev: *mut c_void, conf: *mut c_void) -> *mut c_char {
        let cf = non_null_mut_or!(cf, NGX_CONF_ERROR as _);
        let prev = non_null_mut_or!(prev, NGX_CONF_ERROR as _);
        let conf = non_null_mut_or!(conf, NGX_CONF_ERROR as _);

        if <Self as HttpModule>::merge_srv_conf(cf, prev, conf).is_ok() {
            NGX_CONF_OK as _
        } else {
            NGX_CONF_ERROR as _
        }
    }

    unsafe extern "C" fn create_loc_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let cf = non_null_mut_or!(cf, std::ptr::null_mut());

        if let Ok(conf) = <Self as HttpModule>::create_loc_conf(cf) {
            conf.as_ptr() as _
        } else {
            std::ptr::null_mut()
        }
    }

    unsafe extern "C" fn merge_loc_conf(cf: *mut ngx_conf_t, prev: *mut c_void, conf: *mut c_void) -> *mut c_char {
        let cf = non_null_mut_or!(cf, NGX_CONF_ERROR as _);
        let prev = non_null_mut_or!(prev, NGX_CONF_ERROR as _);
        let conf = non_null_mut_or!(conf, NGX_CONF_ERROR as _);

        if <Self as HttpModule>::merge_loc_conf(cf, prev, conf).is_ok() {
            NGX_CONF_OK as _
        } else {
            NGX_CONF_ERROR as _
        }
    }
}
