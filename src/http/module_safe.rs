#![allow(missing_docs)]

use std::{
    ffi::{c_char, c_void},
    marker::PhantomData,
    ptr::{addr_of, NonNull},
};

use nginx_sys::*;

use crate::core::*;

use super::{HTTPModule, Merge, MergeConfigError};

pub struct Config<'a> {
    inner: NonNull<ngx_conf_t>,
    module: *const ngx_module_t,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> Config<'a> {
    /// # SAFETY
    /// The lifetime of `Self` must correspond to the
    /// lifetime of `ngx_conf_t`
    pub unsafe fn new(inner: *mut ngx_conf_t, module: *const ngx_module_t) -> Option<Self> {
        if inner.is_null() || !inner.is_aligned() {
            return None;
        }

        if module.is_null() || !module.is_aligned() {
            return None;
        }

        let inner = NonNull::new(inner)?;

        Some(Self {
            inner,
            module,
            _phantom: Default::default(),
        })
    }

    pub fn allocate<T>(&self, value: T) -> Option<NonNull<T>> {
        let mut pool = unsafe { Pool::from_ngx_pool((*self.inner.as_ptr()).pool) };
        pool.allocate(value)
    }

    pub fn core_main_conf(&mut self) -> CoreMainConf {
        let core_module = unsafe { addr_of!(ngx_http_core_module) };
        let ptr = unsafe { super::ngx_http_conf_get_module_main_conf(self.inner.as_ptr(), core_module) };
        unsafe { CoreMainConf::new(ptr).unwrap() }
    }
}

pub struct CoreMainConf<'a> {
    conf: NonNull<ngx_http_core_main_conf_t>,
    _phantom: PhantomData<&'a ()>,
}

impl CoreMainConf<'_> {
    /// # SAFETY
    /// `conf` must be valid for `'a`.
    pub(crate) unsafe fn new(conf: *mut ngx_http_core_main_conf_t) -> Option<Self> {
        Some(Self {
            conf: NonNull::new(conf)?,
            _phantom: Default::default(),
        })
    }

    pub fn add_phase_handler(
        &mut self,
        phase: Phase,
        handler: extern "C" fn(*mut ngx_http_request_t) -> ngx_int_t,
    ) -> Result<(), ()> {
        let phases = unsafe { &mut (*self.conf.as_ptr()).phases };
        let phases = &mut phases[phase as usize].handlers;

        NgxArray::new(phases).push(handler)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    PostRead,
    ServerRewrite,
    FindConfig,
    Rewrite,
    PostRewrite,
    PreAccess,
    Access,
    PostAccess,
    PreContent,
    Content,
    Log,
}

impl From<Phase> for ngx_http_phases {
    fn from(value: Phase) -> Self {
        match value {
            Phase::PostRead => ngx_http_phases_NGX_HTTP_POST_READ_PHASE,
            Phase::ServerRewrite => ngx_http_phases_NGX_HTTP_SERVER_REWRITE_PHASE,
            Phase::FindConfig => ngx_http_phases_NGX_HTTP_FIND_CONFIG_PHASE,
            Phase::Rewrite => ngx_http_phases_NGX_HTTP_REWRITE_PHASE,
            Phase::PostRewrite => ngx_http_phases_NGX_HTTP_POST_REWRITE_PHASE,
            Phase::PreAccess => ngx_http_phases_NGX_HTTP_PREACCESS_PHASE,
            Phase::Access => ngx_http_phases_NGX_HTTP_ACCESS_PHASE,
            Phase::PostAccess => ngx_http_phases_NGX_HTTP_POST_ACCESS_PHASE,
            Phase::PreContent => ngx_http_phases_NGX_HTTP_PRECONTENT_PHASE,
            Phase::Content => ngx_http_phases_NGX_HTTP_CONTENT_PHASE,
            Phase::Log => ngx_http_phases_NGX_HTTP_LOG_PHASE,
        }
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

    /// Get a pointer to the NGINX-initialized [`ngx_module_t`] that defines this
    /// module.
    fn module() -> *const ngx_module_t;

    fn preconfiguration(cf: Config) -> Result<(), Error> {
        Ok(())
    }

    fn postconfiguration(cf: Config) -> Result<(), Error> {
        Ok(())
    }

    fn create_main_conf<'a>(cf: Config<'a>) -> Option<NonNull<Self::MainConf>> {
        cf.allocate(Default::default())
    }

    fn init_main_conf(cf: Config, conf: &mut Self::MainConf) -> Result<(), ()> {
        Ok(())
    }

    fn create_srv_conf<'a>(cf: Config<'a>) -> Option<NonNull<Self::SrvConf>> {
        cf.allocate(Default::default())
    }

    fn merge_srv_conf(cf: Config, prev: &mut Self::SrvConf, conf: &mut Self::SrvConf) -> Result<(), MergeConfigError> {
        conf.merge(prev)
    }

    fn create_loc_conf<'a>(cf: Config<'a>) -> Option<NonNull<Self::LocConf>> {
        cf.allocate(Default::default())
    }

    fn merge_loc_conf(cf: Config, prev: &mut Self::LocConf, conf: &mut Self::LocConf) -> Result<(), MergeConfigError> {
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
        let cf = if let Some(cf) = Config::new(cf, <Self as SafeHttpModule>::module()) {
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
        let cf = if let Some(cf) = Config::new(cf, <Self as SafeHttpModule>::module()) {
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
        let cf = if let Some(cf) = Config::new(cf, <Self as SafeHttpModule>::module()) {
            cf
        } else {
            return std::ptr::null_mut() as _;
        };

        let pointer = <Self as SafeHttpModule>::create_main_conf(cf);

        pointer.map(|v| v.as_ptr()).unwrap_or(std::ptr::null_mut()) as _
    }

    unsafe extern "C" fn init_main_conf(cf: *mut ngx_conf_t, conf: *mut c_void) -> *mut c_char {
        let cf = if let Some(cf) = Config::new(cf, <Self as SafeHttpModule>::module()) {
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
        let cf = if let Some(cf) = Config::new(cf, <Self as SafeHttpModule>::module()) {
            cf
        } else {
            return std::ptr::null_mut() as _;
        };

        let pointer = <Self as SafeHttpModule>::create_srv_conf(cf);

        pointer.map(|v| v.as_ptr()).unwrap_or(std::ptr::null_mut()) as _
    }

    unsafe extern "C" fn merge_srv_conf(cf: *mut ngx_conf_t, prev: *mut c_void, conf: *mut c_void) -> *mut c_char {
        let cf = if let Some(cf) = Config::new(cf, <Self as SafeHttpModule>::module()) {
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
        let cf = if let Some(cf) = Config::new(cf, <Self as SafeHttpModule>::module()) {
            cf
        } else {
            return std::ptr::null_mut() as _;
        };

        let pointer = <Self as SafeHttpModule>::create_loc_conf(cf);

        pointer.map(|v| v.as_ptr()).unwrap_or(std::ptr::null_mut()) as _
    }

    unsafe extern "C" fn merge_loc_conf(cf: *mut ngx_conf_t, prev: *mut c_void, conf: *mut c_void) -> *mut c_char {
        let cf = if let Some(cf) = Config::new(cf, <Self as SafeHttpModule>::module()) {
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
