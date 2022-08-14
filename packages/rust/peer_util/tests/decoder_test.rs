use peer_util::decoder::Decoder;

#[test]
fn decoder_test() {
    let mut decoder = Decoder::new().expect("could not construct decoder");
    let mut frames = decoder
        .process(include_bytes!("img.rtp").to_vec())
        .expect("could not decode frame");
    assert_eq!(frames.len(), 1);
    let frame = frames.remove(0);
    assert!(!frame.data.is_empty());
    assert_eq!(frame.width, 700);
    assert_eq!(frame.height, 394);
}
