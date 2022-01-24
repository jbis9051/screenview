mod helpers;
pub mod rvd;
pub mod sel_handler;
pub mod svsc_handler;
pub mod wpskka;

use crate::services::{rvd::RvdInform, svsc_handler::SvscInform, wpskka::WpskkaClientInform};

pub enum InformEvent {
    SvscInform(SvscInform),
    RvdInform(RvdInform),
    WpskkaClientInform(WpskkaClientInform),
}
