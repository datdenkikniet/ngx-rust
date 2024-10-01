mod conf;
mod module;
mod module_safe;
mod request;
mod status;
mod upstream;

pub use conf::*;
pub use module::*;
pub use module_safe::*;
pub use request::*;
pub use status::*;

/// Define a HTTP module.
#[macro_export]
macro_rules! define_http_module {
    ($module:ty, [$($commands:ident),*$(,)?]) => {{
        impl ngx::http::ModuleDefinition for $module {
            fn module() -> *const ngx::ffi::ngx_module_t {
                unsafe { addr_of!(MODULE) }
            }
        }

        #[used]
        static CTX: ngx_http_module_t = ngx::module_context!($module);

        #[used]
        static mut COMMANDS: [ngx_command_t; ngx::count!($($commands,)*) + 1] = ngx::commands!($($commands,)*);

        #[used]
        static mut MODULE: ngx_module_t = ngx_module_t {
            ctx_index: ngx::ffi::ngx_uint_t::MAX,
            index: ngx::ffi::ngx_uint_t::MAX,
            name: std::ptr::null_mut(),
            spare0: 0,
            spare1: 0,
            version: ngx::ffi::nginx_version as _,
            signature: ngx::ffi::NGX_RS_MODULE_SIGNATURE.as_ptr() as _,

            ctx: addr_of!(CTX) as _,
            commands: unsafe { addr_of!(COMMANDS) as _ },
            type_: ngx::ffi::NGX_HTTP_MODULE as _,

            init_master: None,
            init_module: None,
            init_process: None,
            init_thread: None,
            exit_thread: None,
            exit_process: None,
            exit_master: None,

            spare_hook0: 0,
            spare_hook1: 0,
            spare_hook2: 0,
            spare_hook3: 0,
            spare_hook4: 0,
            spare_hook5: 0,
            spare_hook6: 0,
            spare_hook7: 0,
        };

        unsafe { MODULE }
    }};
}
