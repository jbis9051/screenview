use std::io::Cursor;
use common::messages::MessageComponent;
use parser::{MessageComponent};


#[derive(MessageComponent)]
struct BadBool {
    val: bool,
}


#[test]
#[should_panic]
fn test_bad_bool() {
    let bytes = include_bytes!("binary/parser/bad_bool.bin");
    BadBool::read(&mut Cursor::new(bytes)).unwrap();
}

#[derive(MessageComponent)]
struct BadLength {
    #[parse(len_prefixed(1))]
    val: String,
}

#[test]
#[should_panic]
fn test_bad_length() {
    let bytes = include_bytes!("binary/parser/bad_length.bin");
    BadLength::read(&mut Cursor::new(bytes)).unwrap();
}