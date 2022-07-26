use std::borrow::Cow;

use common::{
    messages::{
        rvd::{FrameData, RvdMessage},
        sel::{SelMessage, TransportDataPeerMessageUnreliable},
        svsc::{SessionDataSend, SvscMessage},
        wpskka::{TransportDataMessageUnreliable, WpskkaMessage},
        Data,
        Message,
    },
    sel_cipher::encrypt,
};
use peer::helpers::cipher_unreliable_peer::CipherUnreliablePeer;
use peer_util::frame_data_mtu::{DIRECT_HEADER_LEN, SIGNAL_HEADER_LEN};

#[test]
fn header_test() {
    let mut cipher = CipherUnreliablePeer::new(
        b"yellow submarineyellow submarine".to_vec(),
        b"submarine yellow".to_vec(),
    );

    let rvd = RvdMessage::FrameData(FrameData {
        display_id: 0,
        data: Data(Cow::Owned(Vec::new())),
    });
    let (ciphertext, counter) = cipher.encrypt(&rvd.to_bytes().unwrap()).unwrap();

    let wpskka = WpskkaMessage::TransportDataMessageUnreliable(TransportDataMessageUnreliable {
        counter,
        data: Data(Cow::Owned(ciphertext)),
    });
    let wpskka_bytes = wpskka.to_bytes().unwrap();
    let wpskka_len = wpskka_bytes.len();
    let svsc = SvscMessage::SessionDataSend(SessionDataSend {
        data: Data(Cow::Owned(wpskka_bytes)),
    });
    // If anyone is looking here for how to use the ciphers this is super wrong and just for testing purposes, don't do this
    let (ciphertext, counter) = cipher.encrypt(&svsc.to_bytes().unwrap()).unwrap();
    let sel = SelMessage::TransportDataPeerMessageUnreliable(TransportDataPeerMessageUnreliable {
        peer_id: [0; 16],
        counter,
        data: Data(Cow::Owned(ciphertext)),
    });
    let sel_len = sel.to_bytes().unwrap().len();

    assert_eq!(wpskka_len, DIRECT_HEADER_LEN);
    assert_eq!(sel_len, SIGNAL_HEADER_LEN);
}
