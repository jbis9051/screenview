use bytes::Bytes;
use image::{GenericImageView, RgbImage};
use rtp::packet::Packet;

use video_process::{
    convert::{bgra_to_i420, bgra_to_rgb, i420_to_bgra, rgb_to_bgra},
    rtp::{RtpDecoder, RtpEncoder},
    vp9::{VP9Encoder, Vp9Decoder},
};

use webrtc_util::{Marshal, Unmarshal};


#[test]
pub fn vp9_test() {
    let amount = 2;

    let image =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image");
    let image = image.to_rgb8();
    let (width, height) = image.dimensions();
    let data = image.into_raw();
    let mut bgra = rgb_to_bgra(width, height, &data).expect("unable to convert image");
    let i420_frame = bgra_to_i420(width, height, &mut bgra).expect("unable to convert image");

    let mut encoder = VP9Encoder::new(width, height).expect("unable to create encoder");
    let mut decoder = Vp9Decoder::new().expect("unable to create decoder");
    for _ in 0 .. amount {
        let frames = encoder.encode(&i420_frame).expect("encoder failed");
        for frame in frames {
            decoder.decode(&frame.data).expect("decoder failed");
        }
    }
}


#[test]
pub fn convert_test() {
    let image =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image");
    let image = image.to_rgb8();
    let (width, height) = image.dimensions();
    let data = image.into_raw();
    let mut bgra = rgb_to_bgra(width, height, &data).expect("unable to convert image");
    let data = bgra_to_i420(width, height, &mut bgra).expect("unable to convert image");
    assert!(!data.is_empty());
}


#[test]
pub fn encode_test() {
    let img = include_bytes!("img.i420");
    let (width, height) =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image")
            .dimensions();

    let mut encoder = VP9Encoder::new(width, height).expect("could not construct encoder");
    let mut frames = encoder.encode(img).expect("could not encode frame");
    frames.append(&mut encoder.encode(&[]).unwrap());
    assert!(!frames.is_empty());
}

#[test]
pub fn rtp_encode() {
    let frame = include_bytes!("img.vp9");
    let mut rtp = RtpEncoder::new(10000, 1);
    let packets = rtp
        .process_vp9(frame.to_vec())
        .expect("could not encode frame");
    assert!(!packets.is_empty());
}

#[test]
pub fn rtp_decode() {
    let packet = include_bytes!("img.rtp");
    let mut rtp = RtpDecoder::new();
    assert!(rtp.decode_to_vp9(packet.to_vec()).is_none());

    let mut fake = Packet::unmarshal(&mut Bytes::from(packet.to_vec())).unwrap();
    fake.payload = Bytes::from(vec![0x08]);
    fake.header.sequence_number += 1;
    rtp.decode_to_vp9(fake.marshal().expect("unable to marshall").to_vec())
        .expect("rtp didn't produce frame");
}

#[test]
pub fn decode_test() {
    let img = include_bytes!("img.rtp.out");
    let (width, height) =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image")
            .dimensions();

    let bytes = Bytes::from(&img[..]);
    let mut decoder = Vp9Decoder::new().expect("could not construct encoder");
    let mut frames = decoder.decode(&bytes).expect("could not decode frame");
    frames.extend(decoder.decode(&Bytes::new()).unwrap());
    assert_eq!(frames.len(), 1);
    let frame = frames.remove(0);
    assert_eq!(width as u32, frame.meta.width);
    assert_eq!(height as u32, frame.meta.height);
    assert!(!frame.data.is_empty());
}

#[test]
pub fn finalize() {
    let i420 = &mut include_bytes!("img.i420.out").clone();
    let (width, height) =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image")
            .dimensions();

    let bgra = i420_to_bgra(width, height, i420).expect("unable to convert image");
    let rgb = bgra_to_rgb(width, height, &bgra).expect("unable to convert image");
    RgbImage::from_vec(width, height, rgb).expect("unable to load image");
}

pub fn e2e() -> (u32, u32, Vec<u8>) {
    // Start with a png
    let png =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image");

    // Get our RGB data
    let rgb = png.to_rgb8();
    let (width, height) = rgb.dimensions();
    let rgb_data = rgb.into_raw();

    // Convert to BGRA
    let mut bgra = rgb_to_bgra(width, height, &rgb_data).expect("unable to convert image");
    // Convert to I420
    let i420 = bgra_to_i420(width, height, &mut bgra).expect("unable to convert image");

    // Encode to VP9
    let mut encoder = VP9Encoder::new(width, height).expect("could not construct encoder");
    let mut frames = encoder.encode(&i420).expect("could not encode frame");
    frames.extend(encoder.encode(&[]).unwrap());
    assert_eq!(frames.len(), 1);

    let mut frame = frames.remove(0);
    // Packetize to RTP
    let mut rtp = RtpEncoder::new(25000, 1);
    let mut packets = rtp
        .process_vp9(frame.data.clone())
        .expect("could not encode frame");

    assert!(!packets.is_empty());

    let mut rtp = RtpDecoder::new();

    let mut samples = Vec::new();


    // SampleBuilder won't give us a sample until it has the next packet, so lets pretend a new one came in
    let mut fake = packets[1].clone();
    fake.header.sequence_number += 1;
    fake.payload = Bytes::from(vec![0x08]); // indicate the start of a new frame
    packets.push(fake);


    for mut packet in packets {
        // Depacketize to VP9
        //  println!("{:?}", packet.marshal().unwrap().as_ref());
        match rtp.decode_to_vp9(packet.marshal().expect("unable to marshal").to_vec()) {
            None => {}
            Some(sample) => samples.push(sample),
        };
    }

    assert_eq!(samples.len(), 1);

    let sample = samples.remove(0);

    assert_eq!(&frame.data, &sample.data);

    // Decode to i420
    let mut decoder = Vp9Decoder::new().expect("could not construct encoder");
    let mut frames = decoder
        .decode(&sample.data)
        .expect("could not decode frame");
    frames.extend(decoder.decode(&Bytes::new()).unwrap());
    assert_eq!(frames.len(), 1);

    let frame = frames.remove(0);
    assert_eq!(width as u32, frame.meta.width);
    assert_eq!(height as u32, frame.meta.height);
    assert!(!frame.data.is_empty());

    // Convert to BGRA
    let bgra_out = i420_to_bgra(width, height, &frame.data).expect("unable to convert image");

    // Convert to RGB
    let rgb_out = bgra_to_rgb(width, height, &bgra_out).expect("unable to convert image");

    (width, height, rgb_out)
}

#[test]
pub fn e2e_test() {
    e2e();
}

// Below test is used if you want to view the encoding result. It should not be run on CI.
pub fn e2e_demo() {
    let (width, height, rgb) = e2e();
    RgbImage::from_vec(width, height, rgb)
        .expect("unable to load image")
        .save("out.png")
        .unwrap();
}
