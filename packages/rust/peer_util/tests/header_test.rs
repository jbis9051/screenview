use std::borrow::Cow;

use common::messages::{
    rvd::{FrameData, RvdMessage},
    sel::{SelMessage, TransportDataPeerMessageUnreliable},
    svsc::{SessionDataSend, SvscMessage},
    wpskka::{TransportDataMessageUnreliable, WpskkaMessage},
    Data,
    Message,
};

const DIRECT_HEADER_LEN: usize = 13;
const SIGNAL_HEADER_LEN: usize = 41;

#[test]
fn header_test() {
    let rvd = RvdMessage::FrameData(FrameData {
        display_id: 0,
        data: Data(Cow::Owned(Vec::new())),
    });
    let wpskka = WpskkaMessage::TransportDataMessageUnreliable(TransportDataMessageUnreliable {
        counter: 0,
        data: Data(Cow::Owned(rvd.to_bytes().unwrap())),
    });
    let wpskka_bytes = wpskka.to_bytes().unwrap();
    let wpskka_len = wpskka_bytes.len();
    let svsc = SvscMessage::SessionDataSend(SessionDataSend {
        data: Data(Cow::Owned(wpskka_bytes)),
    });
    let sel = SelMessage::TransportDataPeerMessageUnreliable(TransportDataPeerMessageUnreliable {
        peer_id: [0; 16],
        counter: 0,
        data: Data(Cow::Owned(svsc.to_bytes().unwrap())),
    });
    let sel_len = sel.to_bytes().unwrap().len();

    assert_eq!(wpskka_len, DIRECT_HEADER_LEN);
    assert_eq!(sel_len, SIGNAL_HEADER_LEN);
}
