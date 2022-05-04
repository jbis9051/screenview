use common::messages::MessageComponent;
use std::io::Cursor;

pub fn test_write<'a, T: MessageComponent<'a>>(message: &T, bytes: &[u8]) {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    message.write(&mut cursor).unwrap();
    let inner = cursor.into_inner();
    assert_eq!(inner, bytes, "write failed");
}

#[macro_export]
macro_rules! b_to_u64 {
    ($b:literal) => {
        &u64::from_le_bytes(*$b)
    };
}
