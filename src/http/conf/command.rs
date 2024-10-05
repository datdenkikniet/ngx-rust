use std::ffi::{c_void, CStr};

use nginx_sys::*;

use crate::core::Array;

// TODO: this can return an error of type &'static CStr?
type Set<T> = fn(&mut T, args: Array<ngx_str_t>) -> Result<(), ()>;

/// A builder struct for a [`ngx_command_t`].
pub struct Command<T> {
    name: &'static CStr,
    post: Option<*mut c_void>,

    // FFI says this is an option, but ngx calls this unconditionally, so definitely not optional!
    set: Set<T>,
    ty: u32,
    offset: usize,
}

impl<T> Command<T> {
    /// Create a new [`Command`] that takes the specified count of arguments.
    pub const fn new_count(
        name: &'static CStr,
        ty: ArgCount,
        allowed_contexts: &[CommandContext],
        set: Set<T>,
    ) -> Self {
        Self::new(name, ArgType::Count(ty), allowed_contexts, set)
    }

    /// Create a new [`Command`] with the provided configuration.
    pub const fn new(name: &'static CStr, ty: ArgType, allowed_contexts: &[CommandContext], set: Set<T>) -> Self {
        let mut ty = ty.into_cmd_ty();
        let mut idx = 0;
        loop {
            ty |= allowed_contexts[idx].into_cmd_ty();
            idx += 1;

            if idx == allowed_contexts.len() {
                break;
            }
        }

        Self {
            name,
            post: None,
            set,
            ty,
            offset: 0,
        }
    }

    /// Set the `post` handler for this command.
    pub const fn post(mut self, post: *mut c_void) -> Self {
        self.post = Some(post);
        self
    }

    /// Build this command.
    ///
    /// The `set` should generally be a wrapper around the value returned by [`Command::set`]
    pub const fn build(
        &self,
        conf: ConfOffset,
        set: unsafe extern "C" fn(*mut ngx_conf_t, *mut ngx_command_t, *mut c_void) -> *mut i8,
    ) -> ngx_command_t {
        // This string is valid for `'static`, so conjuring an `ngx_str_t`
        // containing it is OK.
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
            set: Some(set),
            conf: conf.into_conf_offset(),
            offset: self.offset,
            post,
        }
    }

    /// Get the `set` handler for this [`Command`].
    pub const fn set(&self) -> Set<T> {
        self.set
    }
}

/// The configuration offset to use for a command.
///
/// This offset determines what type of pointer is passed to the [`ngx_command_t::set`] callback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfOffset {
    /// The main configuration.
    Main,
    /// The server configuration.
    Srv,
    /// The location configuration.
    Loc,
}

impl ConfOffset {
    /// Get the raw value of this conf offset to use in [`ngx_command_t::type_`].
    pub const fn into_conf_offset(&self) -> usize {
        match self {
            ConfOffset::Main => NGX_RS_HTTP_MAIN_CONF_OFFSET,
            ConfOffset::Srv => NGX_RS_HTTP_SRV_CONF_OFFSET,
            ConfOffset::Loc => NGX_RS_HTTP_LOC_CONF_OFFSET,
        }
    }
}

/// The contexts in which a configuration command is valid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandContext {
    /// The main configuration block.
    Main,
    /// The `http` configuration block.
    Http,
    /// A `server` configuration block within the `http` block.
    Srv,
    /// A `location` block within the `http` block.
    Loc,
    /// An `upstream` block within the `http` block.
    Ups,
    /// An `if` block within a `server` block within the `http` block.
    ServerIf,
    /// An `if` block within a `location` block within the `http` block.
    LocationIf,
    /// In a `limit_except` block within the `http` block.
    LimitExcept,
}

impl CommandContext {
    /// Get the raw value of this command context to use in [`ngx_command_t::type_`].
    pub const fn into_cmd_ty(&self) -> u32 {
        match self {
            CommandContext::Main => NGX_MAIN_CONF,
            CommandContext::Http => NGX_HTTP_MAIN_CONF,
            CommandContext::Srv => NGX_HTTP_SRV_CONF,
            CommandContext::Loc => NGX_HTTP_LOC_CONF,
            CommandContext::Ups => NGX_HTTP_UPS_CONF,
            CommandContext::ServerIf => NGX_HTTP_SIF_CONF,
            CommandContext::LocationIf => NGX_HTTP_LIF_CONF,
            CommandContext::LimitExcept => NGX_HTTP_LMT_CONF,
        }
    }
}

/// The amount of arguments that a command accepts.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArgCount {
    OneOrMore,
    TwoOrMore,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    OneOrTwo,
    OneOrThree,
    TwoOrThree,
    OneOrTwoOrThree,
    OneOrThreeOrTwoOrFour,
}

/// The type of arguments that a command accepts.
#[derive(Debug, Clone, Copy, PartialEq)]

pub enum ArgType {
    /// No arguments.
    None,
    // TODO: what does supporting this entail?
    // Block,
    /// Only `on` or `off`.
    Flag,
    /// A specific amount of arguments.
    Count(ArgCount),
}

impl ArgType {
    /// Get the raw value of this argument type/count to use in [`ngx_command_t::type_`].
    pub const fn into_cmd_ty(&self) -> u32 {
        match self {
            ArgType::None => NGX_CONF_NOARGS,
            ArgType::Flag => NGX_CONF_FLAG,
            ArgType::Count(arg_count) => match arg_count {
                ArgCount::OneOrMore => NGX_CONF_1MORE,
                ArgCount::TwoOrMore => NGX_CONF_2MORE,
                ArgCount::One => NGX_CONF_TAKE1,
                ArgCount::Two => NGX_CONF_TAKE2,
                ArgCount::Three => NGX_CONF_TAKE3,
                ArgCount::Four => NGX_CONF_TAKE4,
                ArgCount::Five => NGX_CONF_TAKE5,
                ArgCount::Six => NGX_CONF_TAKE6,
                ArgCount::Seven => NGX_CONF_TAKE7,
                ArgCount::OneOrTwo => NGX_CONF_TAKE12,
                ArgCount::OneOrThree => NGX_CONF_TAKE13,
                ArgCount::TwoOrThree => NGX_CONF_TAKE23,
                ArgCount::OneOrTwoOrThree => NGX_CONF_TAKE123,
                ArgCount::OneOrThreeOrTwoOrFour => NGX_CONF_TAKE1234,
            },
        }
    }
}
