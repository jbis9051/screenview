use bytes::Bytes;
use native::api::BGRAFrame;
use peer_util::{decoder::Decoder, encoder::Encoder};
use rtp::packet::Packet;
use video_process::convert::rgb_to_bgra;
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

fn encode(num: usize) -> Vec<Packet> {
    let image =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image");
    let image = image.to_rgb8();
    let (width, height) = image.dimensions();
    let data = image.into_raw();
    let bgra = rgb_to_bgra(width, height, &data).expect("unable to convert image");

    let mut frame = BGRAFrame {
        width,
        height,
        data: bgra,
    };

    let mut packets = Vec::new();

    let mut encoder = Encoder::new(1500);

    for _ in 0 .. num {
        packets.extend(encoder.process(&mut frame).expect("processing failed"));
    }

    packets
}

#[test]
fn encoder_test() {
    let packets = encode(3);
    assert!(!packets.is_empty());
}


#[test]
fn process_test() {
    let amount = 3;
    let packets = encode(amount);
    let mut decoder = Decoder::new().expect("could not construct decoder");

    let mut frames = Vec::new();
    for packet in packets {
        frames.extend(
            decoder
                .process(packet.marshal().expect("unable to marshal").to_vec())
                .expect("could not decode frame"),
        );
    }

    assert_eq!(frames.len(), amount - 1); // - 1 cause reasons

    for frame in frames {
        assert!(!frame.data.is_empty());
        assert_eq!(frame.width, 700);
        assert_eq!(frame.height, 394);
    }
}
