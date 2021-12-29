use common::messages::MessageComponent;
use std::io::Cursor;

pub fn test_write<T: MessageComponent>(message: &T, bytes: &[u8]) {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    message.write(&mut cursor).unwrap();
    let inner = cursor.into_inner();
    assert_eq!(inner, bytes, "write failed");
}
