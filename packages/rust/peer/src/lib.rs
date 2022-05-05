extern crate core;

use crate::{
    rvd::{RvdClientInform, RvdHostInform},
    svsc_handler::SvscInform,
    wpskka::{WpskkaClientInform, WpskkaHostInform},
};

pub mod helpers;
pub mod io;

/// The handlers should be fully complaint with the spec. They will only produce errors when the protocol is violated. Handled errors (such as a protocol mismatch) will result in an error event but return Ok().
pub mod higher_handler;
pub mod rvd;
pub mod wpskka;


pub mod sel_handler;
pub mod svsc_handler;


pub enum InformEvent {
    SvscInform(SvscInform),
    RvdClientInform(RvdClientInform),
    RvdHostInform(RvdHostInform),
    WpskkaClientInform(WpskkaClientInform),
    WpskkaHostInform(WpskkaHostInform),
}


#[cold]
pub(crate) fn debug<T: std::fmt::Debug>(val: &T) -> String {
    use std::fmt::Write;

    let mut buf = String::new();
    buf.write_fmt(format_args!("{:?}", val))
        .expect("Debug implementation returned an error unexpectedly");
    buf
}
