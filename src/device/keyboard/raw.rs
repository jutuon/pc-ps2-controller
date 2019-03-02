use bitflags::bitflags;

#[derive(Debug)]
pub struct CommandReturnData;

impl CommandReturnData {
    pub const DEFAULT_DISABLE: u8 = 0xF5;
    pub const ECHO: u8 = 0xEE;
    pub const ENABLE: u8 = 0xF4;
    pub const READ_ID: u8 = 0xF2;
    pub const RESEND: u8 = 0xFE;
    pub const RESET: u8 = 0xFF;
    pub const SELECT_ALTERNATE_SCANCODES: u8 = 0xF0;

    pub const SET_DEFAULT: u8 = 0xF6;
    pub const SET_STATUS_INDICATORS: u8 = 0xED;
    pub const SET_TYPEMATIC_RATE: u8 = 0xF3;
}

#[derive(Debug)]
pub struct CommandSetAllKeys;

impl CommandSetAllKeys {
    pub const TYPEMATIC: u8 = 0xF7;
    pub const MAKE_SLASH_BREAK: u8 = 0xF8;
    pub const MAKE: u8 = 0xF9;
    pub const TYPEMATIC_SLASH_MAKE_SLASH_BREAK: u8 = 0xFA;
}

#[derive(Debug)]
pub struct CommandSetKeyType;

impl CommandSetKeyType {
    pub const TYPEMATIC: u8 = 0xFB;
    pub const MAKE_SLASH_BREAK: u8 = 0xFC;
    pub const MAKE: u8 = 0xFD;
}

bitflags! {
    pub struct StatusIndicators: u8 {
        const SCROLL_LOCK = 0b0000_0001;
        const NUM_LOCK = 0b0000_0010;
        const CAPS_LOCK = 0b0000_0100;
    }
}

#[derive(Debug)]
pub struct FromKeyboard;

impl FromKeyboard {
    pub const KEY_DETECTION_OVERRUN_SCANCODE_SET_2_AND_3: u8 = 0;
    pub const ID_FIRST_BYTE: u8 = 0xAB;
    pub const ID_SECOND_BYTE: u8 = 0x83;
    pub const BAT_COMPLETION_CODE: u8 = 0xAA;
    pub const BAT_FAILURE_CODE: u8 = 0xFC;
    pub const ECHO: u8 = 0xEE;
    pub const ACK: u8 = 0xFA;
    pub const RESEND: u8 = 0xFE;
    pub const KEY_DETECTION_OVERRUN_SCANCODE_SET_1: u8 = 0xFF;
}
