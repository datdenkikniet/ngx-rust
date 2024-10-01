/// Build a configuration command handler. This macro returns
/// a correctly configured [`ngx_command_t`](nginx_sys::ngx_command_t).
///
/// # Basic example
///
/// ```rust
/// # use ngx::http::{SafeHttpModule, CommandBuilder, MergeConfigError};
/// # use ngx::ffi::{ngx_str_t, NGX_HTTP_LOC_CONF, NGX_CONF_TAKE1};
/// struct Module;
///
/// #[derive(Default)]
/// struct LocConf(u128);
/// # impl ngx::http::Merge for LocConf {
/// #     fn merge(&mut self, prev: &Self) -> Result<(), MergeConfigError> {
/// #         self.0 = prev.0;
/// #         Ok(())
/// #     }
/// # }
///
/// impl ngx::http::SafeHttpModule for Module {
///     type MainConf = ();
///     type SrvConf = ();
///     type LocConf = LocConf;
///     # fn module() -> *const ngx::ffi::ngx_module_t { std::ptr::null_mut() }
/// }
///
/// fn set_loc_conf(args: &[ngx_str_t], conf: &mut LocConf) -> Result<(), ()> {
///     let first_arg: u128 = args[0].as_str().parse().map_err(|_| ())?;
///     conf.0 += first_arg;
///     Ok(())
/// }
///
/// const MY_COMMAND: ngx::ffi::ngx_command_t = ngx::command!(
///     Module::LocConf,
///     CommandBuilder::new(c"set_value")
///         .ty(NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1)
///         .set(set_loc_conf)
/// );
/// ```
///
/// # Valid forms
///
/// ```rust,ignore
/// // A type implementing `SafeHttpModule`
/// type Module;
///
/// ngx::command!(Module::MainConf, /* expr: CommandBuilder::<Module::MainConf> */);
/// ngx::command!(Module::SrvConf, /* expr: CommandBuilder::<Module::SrvConf> */);
/// ngx::command!(Module::LocConf, /* expr: CommandBuilder::<Module::LocConf> */);
/// ```
#[macro_export]
macro_rules! command {
    ($module:tt::$conf:tt, $builder:expr) => {{
        use $crate::ffi::{ngx_str_t, ngx_command_t, ngx_conf_t};
        use $crate::core::Array;
        use std::ffi::{c_char, c_void};

        type ConfType = <$module as $crate::http::SafeHttpModule>::$conf;

        const BUILDER: CommandBuilder<ConfType> = $builder;

        #[allow(non_snake_case)]
        unsafe extern "C" fn __raw_c_handler_(
            cf: *mut ngx_conf_t,
            _cmd: *mut ngx_command_t,
            conf: *mut c_void,
        ) -> *mut c_char {
            const HANDLER: fn(&[ngx_str_t], &mut ConfType) -> Result<(), ()> =
                if let Some(handler) = BUILDER.handler() {
                    handler
                } else {
                    fn set_no_op<T>(_args: &[ngx_str_t], _cf: &mut T) -> Result<(), ()> {
                        Ok(())
                    }

                    set_no_op
                };

            let conf = unsafe { (conf as *mut ConfType).as_mut() }.unwrap();
            let args = unsafe { Array::<ngx_str_t>::new_raw((*cf).args) }.unwrap();
            let args = &args[1..];

            let output: Result<(), ()> = HANDLER(args, conf);

            if output.is_ok() {
                $crate::core::NGX_CONF_OK as _
            } else {
                $crate::core::NGX_CONF_ERROR as _
            }
        }

        let mut built = BUILDER.build_partial();

        if BUILDER.handler().is_some() {
            built.set = Some(__raw_c_handler_);
        }

        built.conf = $crate::command!(offset: $conf).into_conf_offset();

        built
    }};

    (offset: MainConf) => {
        $crate::http::ConfOffset::Main
    };

    (offset: LocConf) => {
        $crate::http::ConfOffset::Loc
    };

    (offset: SrvConf) => {
        $crate::http::ConfOffset::Srv
    };
}

/// Define an array of [`ngx_command_t`](crate::ffi::ngx_command_t) commands,
/// with correctly trailing null command.
#[macro_export]
macro_rules! commands {
    ($($command:expr),*$(,)?) => {
        [$($command,)* $crate::ngx_null_command!()]
    }
}
