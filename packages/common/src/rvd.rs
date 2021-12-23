pub struct ProtocolVersion {
    pub version: String // fixed 11 bytes
}

pub struct ProtocolVersionResponse {
    pub ok: bool
}

// type = 1
pub struct DisplayChange {
    pub clipboard_readable: bool,
    pub display_information: Vec<DisplayInformation>
}

type DisplayId = u8;

pub struct DisplayInformation {
    pub display_id: DisplayId,
    pub width: u16,
    pub height: u16,
    pub cell_width: u16,
    pub cell_height: u16,
    pub access: AccessMask,
    pub name: String,
}

pub struct AccessMask {
    pub flush: bool,
    pub controllable: bool,
}

// type = 2
pub struct DisplayChangeReceived {}

// type = 3
pub struct MouseLocation {
    pub display_id: DisplayId,
    pub x_location: u16,
    pub y_location: u16,
}

// type = 4
pub struct MouseInput {
    pub display_id: DisplayId,
    pub x_location: u16,
    pub y_location: u16,
    pub buttons: ButtonsMask
}

pub struct ButtonsMask {
    // TODO
}

// type = 5
pub struct KeyInput {
    pub down: bool,
    pub key: u16, // keysym
}

// TODO Clipboard

// type = 10
pub struct FrameData {
    pub frame_number: u16,
    pub display_id: u8,
    pub cell_number: u16,
    pub data: Vec<u8>
}

