use crate::services::svsc_handler::SvscInform;

mod helpers;
pub mod rvd_client_handler;
pub mod rvd_host_handler;
pub mod sel_handler;
pub mod svsc_handler;
pub mod wpskka_client_handler;
pub mod wpskka_host_handler;

pub enum InformEvent {
    SvscInform(SvscInform),
}
