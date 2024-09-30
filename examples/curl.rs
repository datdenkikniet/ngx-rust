use ngx::core::Array;
use ngx::ffi::{
    nginx_version, ngx_command_t, ngx_conf_t, ngx_http_module_t, ngx_http_request_t, ngx_int_t, ngx_module_t,
    ngx_str_t, ngx_uint_t, NGX_CONF_TAKE1, NGX_HTTP_LOC_CONF, NGX_HTTP_MODULE, NGX_RS_MODULE_SIGNATURE,
};
use ngx::http::{CommandBuilder, MergeConfigError};
use ngx::{core, core::Status, http};
use ngx::{http_request_handler, module_context, ngx_log_debug_http, ngx_null_command};
use std::os::raw::{c_char, c_void};
use std::ptr::addr_of;

struct Module;

impl http::SafeHttpModule for Module {
    type MainConf = ();
    type SrvConf = ();
    type LocConf = ModuleConfig;

    fn module() -> *const ngx_module_t {
        unsafe { addr_of!(ngx_http_curl_module) }
    }

    fn postconfiguration(mut cf: http::Config) -> Result<(), http::Error> {
        let mut cmcf = cf.core_main_conf();
        cmcf.add_phase_handler(http::Phase::Access, curl_access_handler)
            .map_err(|_| http::Error::Error)
    }
}

#[derive(Debug, Default)]
struct ModuleConfig {
    enable: bool,
}

const COMMAND: ngx_command_t = ngx::command!(
    Module,
    LocConf,
    CommandBuilder::new(c"curl")
        .ty(NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1)
        .set(ngx_http_curl_commands_set_enable)
);

#[no_mangle]
#[allow(non_upper_case_globals)]
static mut ngx_http_curl_commands: [ngx_command_t; 2] = [COMMAND, ngx_null_command!()];

// Generate the `ngx_modules` table with exported modules.
// This feature is required to build a 'cdylib' dynamic module outside of the NGINX buildsystem.
#[cfg(feature = "export-modules")]
ngx::ngx_modules!(ngx_http_curl_module);

#[no_mangle]
#[allow(non_upper_case_globals)]
static ngx_http_curl_module_ctx: ngx_http_module_t = module_context!(Module);

#[no_mangle]
#[used]
#[allow(non_upper_case_globals)]
pub static mut ngx_http_curl_module: ngx_module_t = ngx_module_t {
    ctx_index: ngx_uint_t::MAX,
    index: ngx_uint_t::MAX,
    name: std::ptr::null_mut(),
    spare0: 0,
    spare1: 0,
    version: nginx_version as ngx_uint_t,
    signature: NGX_RS_MODULE_SIGNATURE.as_ptr() as *const c_char,

    ctx: &ngx_http_curl_module_ctx as *const _ as *mut _,
    commands: unsafe { addr_of!(ngx_http_curl_commands) as _ },
    type_: NGX_HTTP_MODULE as ngx_uint_t,

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

impl http::Merge for ModuleConfig {
    fn merge(&mut self, prev: &ModuleConfig) -> Result<(), MergeConfigError> {
        if prev.enable {
            self.enable = true;
        };
        Ok(())
    }
}

http_request_handler!(curl_access_handler, |request: &mut http::Request| {
    let co = unsafe { request.get_module_loc_conf::<ModuleConfig>(&*addr_of!(ngx_http_curl_module)) };
    let co = co.expect("module config is none");

    ngx_log_debug_http!(request, "curl module enabled: {}", co.enable);

    match co.enable {
        true => {
            if request
                .user_agent()
                .is_some_and(|ua| ua.as_bytes().starts_with(b"curl"))
            {
                http::HTTPStatus::FORBIDDEN.into()
            } else {
                core::Status::NGX_DECLINED
            }
        }
        false => core::Status::NGX_DECLINED,
    }
});

#[no_mangle]
fn ngx_http_curl_commands_set_enable(conf: &mut ModuleConfig, args: &[ngx_str_t]) -> Result<(), ()> {
    let val = args[0].to_str();

    // set default value optionally
    conf.enable = false;

    if val.eq_ignore_ascii_case("on") {
        conf.enable = true;
    } else if val.eq_ignore_ascii_case("off") {
        conf.enable = false;
    }

    Ok(())
}
