pub fn left_pad(input: &[u8], length: usize) -> Vec<u8> {
    if input.len() > length {
        panic!("input is longer than length");
    }
    let mut output = vec![0; length];
    output[length - input.len() ..].copy_from_slice(input);
    output
}

#[test]
fn left_pad_test() {
    let input = vec![3u8, 97, 98, 99];
    let output = left_pad(&input, 10);
    let expected = vec![0u8, 0, 0, 0, 0, 0, 3, 97, 98, 99];
    assert_eq!(output, expected);
}


#[test]
fn left_pad_test_same_size() {
    let input = vec![3u8, 97, 98, 99];
    let output = left_pad(&input, 4);
    assert_eq!(output, input);
}


#[test]
#[should_panic]
fn left_pad_test_invalid_length() {
    let input = vec![3u8, 97, 98, 99];
    left_pad(&input, 3);
}
