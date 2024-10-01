use ngx::ffi::{
    ngx_command_t, ngx_http_module_t, ngx_http_request_t, ngx_int_t, ngx_module_t, ngx_str_t, NGX_CONF_TAKE1,
    NGX_HTTP_LOC_CONF,
};
use ngx::http::{CommandBuilder, MergeConfigError};
use ngx::{core, core::Status, http};
use ngx::{http_request_handler, ngx_log_debug_http};
use std::os::raw::c_char;
use std::ptr::addr_of;

struct Module;

impl http::SafeHttpModule for Module {
    type MainConf = ();
    type SrvConf = ();
    type LocConf = ModuleConfig;

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

impl http::Merge for ModuleConfig {
    fn merge(&mut self, prev: &ModuleConfig) -> Result<(), MergeConfigError> {
        if prev.enable {
            self.enable = true;
        };
        Ok(())
    }
}

http_request_handler!(curl_access_handler, |request: &mut http::Request| {
    let co = unsafe { request.get_module_loc_conf::<ModuleConfig>(&*addr_of!(NGX_HTTP_CURL_MODULE)) };
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

fn ngx_http_curl_commands_set_enable(args: &[ngx_str_t], conf: &mut ModuleConfig) -> Result<(), ()> {
    let val = args[0].as_str();

    // set default value optionally
    conf.enable = false;

    if val.eq_ignore_ascii_case("on") {
        conf.enable = true;
    } else if val.eq_ignore_ascii_case("off") {
        conf.enable = false;
    }

    Ok(())
}

const COMMAND: ngx_command_t = ngx::command!(
    Module::LocConf,
    CommandBuilder::new(c"curl")
        .ty(NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1)
        .set(ngx_http_curl_commands_set_enable)
);

pub static mut NGX_HTTP_CURL_MODULE: ngx_module_t = ngx::define_http_module!(Module, [COMMAND]);

// Generate the `ngx_modules` table with exported modules.
// This feature is required to build a 'cdylib' dynamic module outside of the NGINX buildsystem.
#[cfg(feature = "export-modules")]
ngx::ngx_modules!(NGX_HTTP_CURL_MODULE);
