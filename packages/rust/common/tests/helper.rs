use common::messages::MessageComponent;
use std::io::Cursor;

pub fn test_write<'a, T: MessageComponent<'a>>(message: &T, bytes: &[u8]) {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    message.write(&mut cursor).unwrap();
    let inner = cursor.into_inner();
    assert_eq!(inner, bytes, "write failed");

    let data = message.to_bytes(None).unwrap();
    assert_eq!(data, bytes, "to_bytes failed");

    let data = message.to_bytes(Some(2)).unwrap();
    let length = u16::from_le_bytes(data[0 .. 2].try_into().unwrap());
    assert_eq!(bytes.len(), length as usize, "to_bytes length failed");
    assert_eq!(&data[2 ..], bytes, "to_bytes failed");
}

#[macro_export]
macro_rules! b_to_u64 {
    ($b:literal) => {
        &u64::from_le_bytes(*$b)
    };
}
