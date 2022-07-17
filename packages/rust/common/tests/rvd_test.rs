mod helper;
use crate::helper::test_write;
use byteorder::{LittleEndian, ReadBytesExt};
use common::messages::{rvd::*, MessageComponent};
use std::{io::Cursor, ops::BitAnd};

#[test]
fn test_version() {
    let bytes = include_bytes!("binary/rvd/protocol_version.bin");
    let message: ProtocolVersion = ProtocolVersion::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.version, String::from_utf8(bytes.to_vec()).unwrap());
    test_write(&message, bytes);
}

#[test]
fn test_version_response() {
    let bytes = include_bytes!("binary/rvd/protocol_version_response.bin");
    let message: ProtocolVersionResponse =
        ProtocolVersionResponse::read(&mut Cursor::new(bytes)).unwrap();
    assert!(!message.ok);
    test_write(&message, bytes);
}

#[test]
fn test_permissions_update() {
    let bytes = include_bytes!("binary/rvd/permissions_update.bin");
    let message = PermissionsUpdate::read(&mut Cursor::new(bytes)).unwrap();
    let mut mask = PermissionMask::CLIPBOARD_READ;
    assert_eq!(message.permission_mask.bits(), mask.bits());
    test_write(&message, bytes);
}

#[test]
fn test_display_share() {
    let bytes = include_bytes!("binary/rvd/display_share.bin");
    let message = DisplayShare::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.display_id, 1);
    assert_eq!(message.access.bits(), AccessMask::CONTROLLABLE.bits());
    assert_eq!(message.name, "name");
    test_write(&message, bytes);
}

#[test]
fn test_display_share_ack() {
    let bytes = include_bytes!("binary/rvd/display_share_ack.bin");
    let message = DisplayShareAck::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.display_id, 1);
    test_write(&message, bytes);
}


#[test]
fn test_display_share_unshare() {
    let bytes = include_bytes!("binary/rvd/display_unshare.bin");
    let message = DisplayUnshare::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.display_id, 1);
    test_write(&message, bytes);
}

#[test]
fn test_mouse_location() {
    let bytes = include_bytes!("binary/rvd/mouse_location.bin");
    let message: MouseLocation = MouseLocation::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.display_id, 1);
    assert_eq!(message.x_location, 200);
    assert_eq!(message.y_location, 200);
    test_write(&message, bytes);
}

#[test]
fn test_mouse_hidden() {
    let bytes = include_bytes!("binary/rvd/mouse_hidden.bin");
    let message: MouseHidden = MouseHidden::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.display_id, 1);
    test_write(&message, bytes);
}

#[test]
fn test_mouse_input() {
    let bytes = include_bytes!("binary/rvd/mouse_input.bin");
    let message: MouseInput = MouseInput::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.display_id, 1);
    assert_eq!(message.x_location, 200);
    assert_eq!(message.y_location, 200);
    assert_eq!(message.buttons_delta, ButtonsMask::empty());
    assert_eq!(message.buttons_state, ButtonsMask::empty());
    test_write(&message, bytes);
}

#[test]
fn test_key_input() {
    let bytes = include_bytes!("binary/rvd/key_input.bin");
    let message: KeyInput = KeyInput::read(&mut Cursor::new(bytes)).unwrap();
    assert!(message.down);
    assert_eq!(
        message.key,
        b"1234".as_slice().read_u32::<LittleEndian>().unwrap()
    );
    test_write(&message, bytes);
}

#[test]
fn test_clipboard_request_default() {
    let bytes = include_bytes!("binary/rvd/clipboard_request_default.bin");
    let message: ClipboardRequest = ClipboardRequest::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.info.clipboard_type, ClipboardType::Text);
    assert!(message.info.content_request);
    test_write(&message, bytes);
}

#[test]
fn test_clipboard_request_custom() {
    let bytes = include_bytes!("binary/rvd/clipboard_request_custom.bin");
    let message: ClipboardRequest = ClipboardRequest::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(
        message.info.clipboard_type,
        ClipboardType::Custom("test".to_owned())
    );
    assert!(!message.info.content_request);
    test_write(&message, bytes);
}

#[test]
#[should_panic]
fn test_clipboard_request_bad() {
    let bytes = include_bytes!("binary/rvd/clipboard_request_bad.bin");
    ClipboardRequest::read(&mut Cursor::new(bytes)).unwrap();
}

#[test]
fn clipboard_notification_default_content() {
    let bytes = include_bytes!("binary/rvd/clipboard_notification_default_content.bin");
    let message: ClipboardNotification =
        ClipboardNotification::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.info.clipboard_type, ClipboardType::Text);
    assert!(message.info.content_request);
    assert!(message.content.is_some());
    assert_eq!(message.content.as_ref().unwrap(), b"abcd");
    test_write(&message, bytes);
}

#[test]
fn clipboard_notification_custom_content() {
    let bytes = include_bytes!("binary/rvd/clipboard_notification_custom_content.bin");
    let message: ClipboardNotification =
        ClipboardNotification::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(
        message.info.clipboard_type,
        ClipboardType::Custom("test".to_owned())
    );
    assert!(message.info.content_request);
    assert!(message.content.is_some());
    assert_eq!(message.content.as_ref().unwrap(), b"abcd");
    test_write(&message, bytes);
}

#[test]
fn frame_data() {
    let bytes = include_bytes!("binary/rvd/frame_data.bin");
    let message: FrameData = FrameData::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.display_id, 5);
    assert_eq!(&message.data, b"abc");
    test_write(&message, bytes);
}
