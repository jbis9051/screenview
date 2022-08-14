use bytes::Bytes;
use peer_util::decoder::Decoder;
use rtp::packet::Packet;
use webrtc_util::{Marshal, Unmarshal};

#[test]
fn decoder_test() {
    let mut decoder = Decoder::new().expect("could not construct decoder");
    let rtp = include_bytes!("img.rtp").to_vec();

    let mut frames = decoder
        .process(rtp.clone())
        .expect("could not decode frame");

    let mut fake = Packet::unmarshal(&mut Bytes::from(rtp)).expect("unable to unmarshal");
    fake.header.sequence_number += 1;
    fake.payload = Bytes::from(vec![0x08]);

    frames.extend(
        decoder
            .process(fake.marshal().expect("unable to marshal").to_vec())
            .expect("could not decode frame"),
    );

    assert_eq!(frames.len(), 1);
    let frame = frames.remove(0);
    assert!(!frame.data.is_empty());
    assert_eq!(frame.width, 700);
    assert_eq!(frame.height, 394);
}
