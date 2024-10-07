/// Define a new command.
///
/// This macro takes a [`HTTPModule`](crate::http::HTTPModule), the type of config (`MainConf`, `SrvConf` or `LocConf`)
/// and an expression that evaluates to a [`Command`](crate::http::Command).
// Rustfmt is wrong/obnouxious here, so ignore for now.
#[rustfmt::skip]
#[macro_export]
macro_rules! command {
    ($module:ty, $config_type:ident, $command:expr) => {{
    
        type Config = $crate::command!(ty: $module, $config_type);
        const OFFSET: $crate::http::ConfOffset = $crate::command!(offset: $config_type);

        const __COMMAND: Command<Config> = $command;
        const SET: fn(&mut Config, Array<$crate::ffi::ngx_str_t>) -> Result<(), ()> = COMMAND.set();

        extern "C" fn set(cf: *mut ngx_conf_t, _cmd: *mut ngx_command_t, conf: *mut c_void) -> *mut i8 {
            // SAFETY: the set call has exclusive access to the provided configuration
            // object, which is of the type specified by the offset, which is plumbed
            // into the `HTTPModule` correctly.
            let config =  unsafe { (conf as *mut Config).as_mut().unwrap() };

            // SAFETY: `cf.args` is valid for at least the duration of this function.
            let args = unsafe { Array::<ngx_str_t>::new(NonNull::new((*cf).args).unwrap()) };

            if SET(config, args).is_ok() {
                $crate::core::NGX_CONF_OK as _
            } else {
                $crate::core::NGX_CONF_ERROR as _
            }
        }

        __COMMAND.build(OFFSET, set)
    }};

    (ty: $module:ty, $ty:ident) => {
        <$module as $crate::http::RawHttpModule>::$ty
    };

    (offset: LocConf) => {
        $crate::http::ConfOffset::Loc
    };

    (offset: MainConf) => {
        $crate::http::ConfOffset::MainConf
    };

    (offset: SrvConf) => {
        $crate::http::ConfOffset::SrvConf
    };
}
