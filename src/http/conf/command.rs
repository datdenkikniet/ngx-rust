#![allow(missing_docs)]

use std::ffi::{c_void, CStr};

use nginx_sys::*;

type Set<T> = fn(&[ngx_str_t], &mut T) -> Result<(), ()>;

pub struct Command<T> {
    name: &'static CStr,
    post: Option<*mut c_void>,
    set: Option<Set<T>>,
    ty: u32,
    offset: usize,
}

impl<T> Command<T> {
    pub const fn new_count(name: &'static CStr, ty: ArgCount, allowed_contexts: &[CommandContext]) -> Self {
        Self::new(name, ArgType::Count(ty), allowed_contexts)
    }

    pub const fn new(name: &'static CStr, ty: ArgType, allowed_contexts: &[CommandContext]) -> Self {
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
            set: None,
            ty,
            offset: 0,
        }
    }

    pub const fn post(mut self, post: *mut c_void) -> Self {
        self.post = Some(post);
        self
    }

    pub const fn set(mut self, set: Set<T>) -> Self {
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandContext {
    Main,
    Http,
    Srv,
    Loc,
    Ups,
    ServerIf,
    LocationIf,
    LimitExcept,
}

impl CommandContext {
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

#[derive(Debug, Clone, Copy, PartialEq)]

pub enum ArgType {
    None,
    // TODO: what does supporting this entail?
    // Block,
    Flag,
    Count(ArgCount),
}

impl ArgType {
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
