use crate::ffi::*;

use std::{ffi::CStr, os::raw::c_void};

/// # Safety
///
/// The caller has provided a valid `ngx_conf_t` that points to valid memory and is non-null.
pub unsafe fn ngx_http_conf_get_module_main_conf(
    cf: *mut ngx_conf_t,
    module: *const ngx_module_t,
) -> *mut ngx_http_core_main_conf_t {
    let http_conf_ctx = (*cf).ctx as *mut ngx_http_conf_ctx_t;
    *(*http_conf_ctx).main_conf.add((*module).ctx_index) as *mut ngx_http_core_main_conf_t
}

/// # Safety
///
/// The caller has provided a valid `ngx_conf_t` that points to valid memory and is non-null.
pub unsafe fn ngx_http_conf_get_module_srv_conf(cf: *mut ngx_conf_t, module: *const ngx_module_t) -> *mut c_void {
    let http_conf_ctx = (*cf).ctx as *mut ngx_http_conf_ctx_t;
    *(*http_conf_ctx).srv_conf.add((*module).ctx_index)
}

/// # Safety
///
/// The caller has provided a valid `ngx_conf_t` that points to valid memory and is non-null.
pub unsafe fn ngx_http_conf_get_module_loc_conf(
    cf: *mut ngx_conf_t,
    module: &ngx_module_t,
) -> *mut ngx_http_core_loc_conf_t {
    let http_conf_ctx = (*cf).ctx as *mut ngx_http_conf_ctx_t;
    *(*http_conf_ctx).loc_conf.add(module.ctx_index) as *mut ngx_http_core_loc_conf_t
}

/// # Safety
///
/// The caller has provided a value `ngx_http_upstream_srv_conf_t. If the `us` argument is null, a
/// None Option is returned; however, if the `us` internal fields are invalid or the module index
/// is out of bounds failures may still occur.
pub unsafe fn ngx_http_conf_upstream_srv_conf_immutable<T>(
    us: *const ngx_http_upstream_srv_conf_t,
    module: &ngx_module_t,
) -> Option<*const T> {
    if us.is_null() {
        return None;
    }
    Some(*(*us).srv_conf.add(module.ctx_index) as *const T)
}

/// # Safety
///
/// The caller has provided a value `ngx_http_upstream_srv_conf_t. If the `us` argument is null, a
/// None Option is returned; however, if the `us` internal fields are invalid or the module index
/// is out of bounds failures may still occur.
pub unsafe fn ngx_http_conf_upstream_srv_conf_mutable<T>(
    us: *const ngx_http_upstream_srv_conf_t,
    module: &ngx_module_t,
) -> Option<*mut T> {
    if us.is_null() {
        return None;
    }
    Some(*(*us).srv_conf.add(module.ctx_index) as *mut T)
}

pub struct CommandBuilder {
    name: &'static CStr,
    post: Option<*mut c_void>,
    set: Option<unsafe extern "C" fn(*mut ngx_conf_s, *mut ngx_command_s, *mut c_void) -> *mut i8>,
    ty: u32,
    conf_offset: ConfOffset,
    offset: usize,
}

impl CommandBuilder {
    pub const fn new(name: &'static CStr, conf_offset: ConfOffset) -> Self {
        Self {
            name,
            post: None,
            set: None,
            ty: 0,
            conf_offset,
            offset: 0,
        }
    }

    pub const fn post(mut self, post: *mut c_void) -> Self {
        self.post = Some(post);
        self
    }

    pub const fn ty(mut self, ty: u32) -> Self {
        self.ty = ty;
        self
    }

    pub const fn set(
        mut self,
        set: unsafe extern "C" fn(*mut ngx_conf_s, *mut ngx_command_s, *mut c_void) -> *mut i8,
    ) -> Self {
        self.set = Some(set);
        self
    }

    pub const fn build(&self) -> ngx_command_t {
        let name = ngx_str_t {
            len: self.name.count_bytes(),
            data: self.name.as_ptr() as _,
        };

        let post = if let Some(post) = self.post {
            post
        } else {
            std::ptr::null_mut()
        };

        ngx_command_t {
            name,
            type_: self.ty as _,
            set: self.set,
            conf: self.conf_offset.into_conf_offset(),
            offset: self.offset,
            post,
        }
    }
}

impl From<CommandBuilder> for ngx_command_t {
    fn from(value: CommandBuilder) -> Self {
        value.build()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfOffset {
    Main,
    Srv,
    Loc,
}

impl ConfOffset {
    const fn into_conf_offset(&self) -> usize {
        match self {
            ConfOffset::Main => NGX_RS_HTTP_MAIN_CONF_OFFSET,
            ConfOffset::Srv => NGX_RS_HTTP_SRV_CONF_OFFSET,
            ConfOffset::Loc => NGX_RS_HTTP_LOC_CONF_OFFSET,
        }
    }
}
