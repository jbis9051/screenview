use std::io::Cursor;
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