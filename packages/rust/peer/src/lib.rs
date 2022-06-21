#![deny(rust_2018_idioms)]
#![allow(clippy::new_without_default, clippy::ptr_arg)]

use crate::{
    io::Source,
    rvd::{RvdClientInform, RvdHostInform},
    svsc_handler::SvscInform,
    wpskka::{WpskkaClientInform, WpskkaHostInform},
};

pub mod capture;
pub mod helpers;
pub mod io;

/// The handlers should be fully complaint with the spec. They will only produce errors when the protocol is violated. Handled errors (such as a protocol mismatch) will result in an error event but return Ok().
pub mod higher_handler;
pub mod rvd;
pub mod wpskka;


pub mod handler_stack;
pub mod lower;
pub mod sel_handler;
pub mod svsc_handler;

pub enum InformEvent {
    TransportShutdown(Source),
    SvscInform(SvscInform),
    RvdClientInform(RvdClientInform),
    RvdHostInform(RvdHostInform),
    WpskkaClientInform(WpskkaClientInform),
    WpskkaHostInform(WpskkaHostInform),
}

pub enum ChanneledMessage<T> {
    Reliable(T),
    Unreliable(T),
}

impl<T> ChanneledMessage<T> {
    pub fn map<F, U>(self, f: F) -> ChanneledMessage<U>
    where F: FnOnce(T) -> U {
        match self {
            Self::Reliable(msg) => ChanneledMessage::Reliable(f(msg)),
            Self::Unreliable(msg) => ChanneledMessage::Unreliable(f(msg)),
        }
    }
}

#[cold]
pub(crate) fn debug<T: std::fmt::Debug>(val: &T) -> String {
    use std::fmt::Write;

    let mut buf = String::new();
    buf.write_fmt(format_args!("{:?}", val))
        .expect("Debug implementation returned an error unexpectedly");
    buf
}
