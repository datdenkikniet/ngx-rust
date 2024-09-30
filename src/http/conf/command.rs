#![allow(missing_docs)]

use std::ffi::{c_void, CStr};

use nginx_sys::*;

type Set<T> = fn(&mut T, &[ngx_str_t]) -> Result<(), ()>;

pub struct CommandBuilder<T> {
    name: &'static CStr,
    post: Option<*mut c_void>,
    set: Option<Set<T>>,
    ty: u32,
    offset: usize,
}

impl<T> CommandBuilder<T> {
    pub const fn new(name: &'static CStr) -> Self {
        Self {
            name,
            post: None,
            set: None,
            ty: 0,
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

    pub const fn set(mut self, set: fn(&mut T, &[ngx_str_t]) -> Result<(), ()>) -> Self {
        self.set = Some(set);
        self
    }

    pub const fn build_partial(&self) -> ngx_command_t {
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
            set: None,
            conf: 0,
            offset: self.offset,
            post,
        }
    }

    pub const fn handler(&self) -> Option<Set<T>> {
        self.set
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfOffset {
    Main,
    Srv,
    Loc,
}

impl ConfOffset {
    pub const fn into_conf_offset(&self) -> usize {
        match self {
            ConfOffset::Main => NGX_RS_HTTP_MAIN_CONF_OFFSET,
            ConfOffset::Srv => NGX_RS_HTTP_SRV_CONF_OFFSET,
            ConfOffset::Loc => NGX_RS_HTTP_LOC_CONF_OFFSET,
        }
    }
}

#[macro_export]
macro_rules! define_command {
    ($conf_type:ty, $handler:ident) => {
        $crate::paste::paste! {
            #[no_mangle]
            extern "C" fn [<__raw_c_handler_ $handler>](
                cf: *mut ngx_conf_t,
                _cmd: *mut ngx_command_t,
                conf: *mut c_void,
            ) -> *mut c_char {
                let conf = unsafe { (conf as *mut $conf_type).as_mut() }.unwrap();
                let args = unsafe { Array::<ngx_str_t>::new_raw((*cf).args) }.unwrap();
                let args = &args[1..];

                let the_fn: fn(&mut $conf_type, &[ngx_str_t]) -> Result<(), ()> = $handler;
                let output: Result<(), ()> = the_fn(conf, args);

                if output.is_ok() {
                    $crate::core::NGX_CONF_OK as _
                } else {
                    $crate::core::NGX_CONF_ERROR as _
                }
            }
        }
    };
}
