use ngx::core::Array;
use ngx::ffi::{
    ngx_array_push, ngx_command_t, ngx_conf_t, ngx_http_core_module, ngx_http_handler_pt,
    ngx_http_phases_NGX_HTTP_ACCESS_PHASE, ngx_http_request_t, ngx_int_t, ngx_module_t, ngx_str_t, ngx_uint_t,
};
use ngx::http::{Command, CommandContext, MergeConfigError};
use ngx::{core, core::Status, http};
use ngx::{http_request_handler, ngx_log_debug_http};
use std::os::raw::{c_char, c_void};
use std::ptr::{addr_of, NonNull};

struct Module;

impl http::HTTPModule for Module {
    type MainConf = ();
    type SrvConf = ();
    type LocConf = ModuleConfig;

    unsafe extern "C" fn postconfiguration(cf: *mut ngx_conf_t) -> ngx_int_t {
        let cmcf = http::ngx_http_conf_get_module_main_conf(cf, &*addr_of!(ngx_http_core_module));

        let h = ngx_array_push(&mut (*cmcf).phases[ngx_http_phases_NGX_HTTP_ACCESS_PHASE as usize].handlers)
            as *mut ngx_http_handler_pt;
        if h.is_null() {
            return core::Status::NGX_ERROR.into();
        }
        // set an Access phase handler
        *h = Some(curl_access_handler);
        core::Status::NGX_OK.into()
    }
}

#[derive(Debug, Default)]
struct ModuleConfig {
    enable: bool,
}

const COMMAND: Command<ModuleConfig> = Command::new(c"curl", http::ArgType::Flag, &[CommandContext::Loc], set_curl);

#[used]
pub static mut MODULE: ngx_module_t = ngx::http_module_conf!(Module, [LocConf: COMMAND]);

// Generate the `ngx_modules` table with exported modules.
// This feature is required to build a 'cdylib' dynamic module outside of the NGINX buildsystem.
#[cfg(feature = "export-modules")]
ngx::ngx_modules!(MODULE);

impl http::Merge for ModuleConfig {
    fn merge(&mut self, prev: &ModuleConfig) -> Result<(), MergeConfigError> {
        if prev.enable {
            self.enable = true;
        };
        Ok(())
    }
}

http_request_handler!(curl_access_handler, |request: &mut http::Request| {
    let co = unsafe { request.get_module_loc_conf::<ModuleConfig>(&*addr_of!(MODULE)) };
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

fn set_curl(conf: &mut ModuleConfig, args: Array<ngx_str_t>) -> Result<(), ()> {
    let val = args[1].to_str();

    // set default value optionally
    conf.enable = false;

    if val.len() == 2 && val.eq_ignore_ascii_case("on") {
        conf.enable = true;
    } else if val.len() == 3 && val.eq_ignore_ascii_case("off") {
        conf.enable = false;
    }

    Ok(())
}
