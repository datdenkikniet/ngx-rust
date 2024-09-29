use http::HeaderMap;
use ngx::core::Array;
use ngx::ffi::{
    nginx_version, ngx_command_t, ngx_conf_t, ngx_http_module_t, ngx_http_request_t, ngx_int_t, ngx_module_t,
    ngx_str_t, ngx_uint_t, NGX_CONF_TAKE1, NGX_HTTP_LOC_CONF, NGX_HTTP_MODULE, NGX_HTTP_SRV_CONF,
    NGX_RS_HTTP_LOC_CONF_OFFSET, NGX_RS_MODULE_SIGNATURE,
};
use ngx::{core, core::Status, http::*};
use ngx::{http_request_handler, module_context, ngx_log_debug_http, ngx_null_command, ngx_string};
use std::os::raw::{c_char, c_void};
use std::ptr::addr_of;

unsafe fn args<'a>(conf: *mut ngx_conf_t) -> Option<Array<'a, ngx_str_t>> {
    Array::new_raw((*conf).args)
}

struct Module;

impl SafeHttpModule for Module {
    type MainConf = ();
    type SrvConf = ();
    type LocConf = ModuleConfig;

    fn module() -> *const ngx_module_t {
        unsafe { addr_of!(ngx_http_awssigv4_module) }
    }

    fn postconfiguration(mut cf: Config) -> Result<(), Error> {
        cf.core_main_conf()
            .add_phase_handler(Phase::PreContent, awssigv4_header_handler)
            .map_err(|_| Error::Error)
    }
}

#[derive(Debug, Default)]
struct ModuleConfig {
    enable: bool,
    access_key: String,
    secret_key: String,
    s3_bucket: String,
    s3_endpoint: String,
}

#[no_mangle]
#[allow(non_upper_case_globals)]
static mut ngx_http_awssigv4_commands: [ngx_command_t; 6] = [
    ngx_command_t {
        name: ngx_string!("awssigv4"),
        type_: (NGX_HTTP_LOC_CONF | NGX_HTTP_SRV_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_awssigv4_commands_set_enable),
        conf: NGX_RS_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("awssigv4_access_key"),
        type_: (NGX_HTTP_LOC_CONF | NGX_HTTP_SRV_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_awssigv4_commands_set_access_key),
        conf: NGX_RS_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("awssigv4_secret_key"),
        type_: (NGX_HTTP_LOC_CONF | NGX_HTTP_SRV_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_awssigv4_commands_set_secret_key),
        conf: NGX_RS_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("awssigv4_s3_bucket"),
        type_: (NGX_HTTP_LOC_CONF | NGX_HTTP_SRV_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_awssigv4_commands_set_s3_bucket),
        conf: NGX_RS_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("awssigv4_s3_endpoint"),
        type_: (NGX_HTTP_LOC_CONF | NGX_HTTP_SRV_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_awssigv4_commands_set_s3_endpoint),
        conf: NGX_RS_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_null_command!(),
];

#[no_mangle]
#[allow(non_upper_case_globals)]
static ngx_http_awssigv4_module_ctx: ngx_http_module_t = module_context!(Module);

// Generate the `ngx_modules` table with exported modules.
// This feature is required to build a 'cdylib' dynamic module outside of the NGINX buildsystem.
#[cfg(feature = "export-modules")]
ngx::ngx_modules!(ngx_http_awssigv4_module);

#[no_mangle]
#[used]
#[allow(non_upper_case_globals)]
pub static mut ngx_http_awssigv4_module: ngx_module_t = ngx_module_t {
    ctx_index: ngx_uint_t::MAX,
    index: ngx_uint_t::MAX,
    name: std::ptr::null_mut(),
    spare0: 0,
    spare1: 0,
    version: nginx_version as ngx_uint_t,
    signature: NGX_RS_MODULE_SIGNATURE.as_ptr() as *const c_char,

    ctx: &ngx_http_awssigv4_module_ctx as *const _ as *mut _,
    commands: unsafe { &ngx_http_awssigv4_commands[0] as *const _ as *mut _ },
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

impl Merge for ModuleConfig {
    fn merge(&mut self, prev: &ModuleConfig) -> Result<(), MergeConfigError> {
        if prev.enable {
            self.enable = true;
        };

        if self.access_key.is_empty() {
            self.access_key = String::from(if !prev.access_key.is_empty() {
                &prev.access_key
            } else {
                ""
            });
        }
        if self.enable && self.access_key.is_empty() {
            return Err(MergeConfigError::NoValue);
        }

        if self.secret_key.is_empty() {
            self.secret_key = String::from(if !prev.secret_key.is_empty() {
                &prev.secret_key
            } else {
                ""
            });
        }
        if self.enable && self.secret_key.is_empty() {
            return Err(MergeConfigError::NoValue);
        }

        if self.s3_bucket.is_empty() {
            self.s3_bucket = String::from(if !prev.s3_bucket.is_empty() {
                &prev.s3_bucket
            } else {
                ""
            });
        }
        if self.enable && self.s3_bucket.is_empty() {
            return Err(MergeConfigError::NoValue);
        }

        if self.s3_endpoint.is_empty() {
            self.s3_endpoint = String::from(if !prev.s3_endpoint.is_empty() {
                &prev.s3_endpoint
            } else {
                "s3.amazonaws.com"
            });
        }
        Ok(())
    }
}

#[no_mangle]
extern "C" fn ngx_http_awssigv4_commands_set_enable(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    conf: *mut c_void,
) -> *mut c_char {
    let conf = unsafe { (conf as *mut ModuleConfig).as_mut() }.unwrap();
    let args = unsafe { args(cf) }.unwrap();
    let val = args[1].to_str();

    // set default value optionally
    conf.enable = false;

    if val.len() == 2 && val.eq_ignore_ascii_case("on") {
        conf.enable = true;
    } else if val.len() == 3 && val.eq_ignore_ascii_case("off") {
        conf.enable = false;
    }

    std::ptr::null_mut()
}

#[no_mangle]
extern "C" fn ngx_http_awssigv4_commands_set_access_key(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    conf: *mut c_void,
) -> *mut c_char {
    let conf = unsafe { (conf as *mut ModuleConfig).as_mut() }.unwrap();
    let args = unsafe { args(cf) }.unwrap();
    conf.access_key = args[1].to_string();

    std::ptr::null_mut()
}

#[no_mangle]
extern "C" fn ngx_http_awssigv4_commands_set_secret_key(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    conf: *mut c_void,
) -> *mut c_char {
    let conf = unsafe { (conf as *mut ModuleConfig).as_mut() }.unwrap();
    let args = unsafe { args(cf) }.unwrap();

    conf.secret_key = args[1].to_string();

    std::ptr::null_mut()
}

#[no_mangle]
extern "C" fn ngx_http_awssigv4_commands_set_s3_bucket(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    conf: *mut c_void,
) -> *mut c_char {
    let conf = unsafe { (conf as *mut ModuleConfig).as_mut() }.unwrap();
    let args = unsafe { args(cf) }.unwrap();

    conf.s3_bucket = args[1].to_string();
    if conf.s3_bucket.len() == 1 {
        println!("Validation failed");
        return ngx::core::NGX_CONF_ERROR as _;
    }

    std::ptr::null_mut()
}

#[no_mangle]
extern "C" fn ngx_http_awssigv4_commands_set_s3_endpoint(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    conf: *mut c_void,
) -> *mut c_char {
    let conf = unsafe { (conf as *mut ModuleConfig).as_mut() }.unwrap();
    let args = unsafe { args(cf) }.unwrap();

    conf.s3_endpoint = args[1].to_string();

    std::ptr::null_mut()
}

http_request_handler!(awssigv4_header_handler, |request: &mut Request| {
    // get Module Config from request
    let conf = unsafe { request.get_module_loc_conf::<ModuleConfig>(&*addr_of!(ngx_http_awssigv4_module)) };
    let conf = conf.unwrap();
    ngx_log_debug_http!(request, "AWS signature V4 module {}", {
        if conf.enable {
            "enabled"
        } else {
            "disabled"
        }
    });
    if !conf.enable {
        return core::Status::NGX_DECLINED;
    }

    // TODO: build url properly from the original URL from client
    let method = request.method();
    if !matches!(method, ngx::http::Method::HEAD | ngx::http::Method::GET) {
        return HTTPStatus::FORBIDDEN.into();
    }

    let datetime = chrono::Utc::now();
    let uri = match request.unparsed_uri().to_str() {
        Ok(v) => format!("https://{}.{}{}", conf.s3_bucket, conf.s3_endpoint, v),
        Err(_) => return core::Status::NGX_DECLINED,
    };

    let datetime_now = datetime.format("%Y%m%dT%H%M%SZ");
    let datetime_now = datetime_now.to_string();

    let signature = {
        // NOTE: aws_sign_v4::AwsSign::new() implementation requires a HeaderMap.
        // Iterate over requests headers_in and copy into HeaderMap
        // Copy only headers that will be used to sign the request
        let mut headers = HeaderMap::new();
        for (name, value) in request.headers_in_iterator() {
            match name.to_lowercase().as_str() {
                "host" => {
                    headers.insert(http::header::HOST, value.parse().unwrap());
                }
                &_ => {}
            };
        }
        headers.insert("X-Amz-Date", datetime_now.parse().unwrap());
        ngx_log_debug_http!(request, "headers {:?}", headers);
        ngx_log_debug_http!(request, "method {:?}", method);
        ngx_log_debug_http!(request, "uri {:?}", uri);
        ngx_log_debug_http!(request, "datetime_now {:?}", datetime_now);

        let s = aws_sign_v4::AwsSign::new(
            method.as_str(),
            &uri,
            &datetime,
            &headers,
            "us-east-1",
            conf.access_key.as_str(),
            conf.secret_key.as_str(),
            "s3",
            "",
        );
        s.sign()
    };

    request.add_header_in("authorization", signature.as_str());
    request.add_header_in("X-Amz-Date", datetime_now.as_str());

    // done signing, let's print values we have in request.headers_out, request.headers_in
    for (name, value) in request.headers_out_iterator() {
        ngx_log_debug_http!(request, "headers_out {}: {}", name, value);
    }
    for (name, value) in request.headers_in_iterator() {
        ngx_log_debug_http!(request, "headers_in  {}: {}", name, value);
    }

    core::Status::NGX_OK
});
