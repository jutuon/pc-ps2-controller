
use bitflags::bitflags;

bitflags! {
    pub struct StatusRegister: u8 {
        const KEYBOARD_PARITY_ERROR = 0b1000_0000;
        const GENERAL_TIMEOUT = 0b0100_0000;
        const AUXILIARY_DEVICE_OUTPUT_BUFFER_FULL = 0b0010_0000;
        const INHIBIT_SWITCH = 0b0001_0000;
        const COMMAND_OR_DATA = 0b0000_1000;
        const SYSTEM_FLAG = 0b0000_0100;
        const INPUT_BUFFER_FULL = 0b0000_0010;
        const OUTPUT_BUFFER_FULL = 0b0000_0001;
    }
}


pub const CONTROLLER_RAM_SIZE: usize = (RawCommands::READ_RAM_END - RawCommands::READ_RAM_START + 1) as usize;

pub struct RawCommands;

impl RawCommands {
    pub const READ_CONTROLLER_COMMAND_BYTE: u8 = 0x20;
    pub const READ_RAM_START: u8 = 0x21;
    pub const READ_RAM_END: u8 = 0x3F;

    pub const WRITE_CONTROLLER_COMMAND_BYTE: u8 = 0x60;
    pub const WRITE_RAM_START: u8 = 0x61;
    //const WRITE_RAM_END: u8 = 0x7F;

    //const TEST_PASSWORD_INSTALLED: u8 = 0xA4;
    //const LOAD_PASSWORD: u8 = 0xA5;
    //const ENABLE_PASSWORD: u8 = 0xA6;
    pub const DISABLE_AUXILIARY_DEVICE_INTERFACE: u8 = 0xA7;
    pub const ENABLE_AUXILIARY_DEVICE_INTERFACE: u8 = 0xA8;
    pub const AUXILIARY_DEVICE_INTERFACE_TEST: u8 = 0xA9;
    pub const SELF_TEST: u8 = 0xAA;
    pub const KEYBOARD_INTERFACE_TEST: u8 = 0xAB;
    pub const DISABLE_KEYBOARD_INTERFACE: u8 = 0xAD;
    pub const ENABLE_KEYBOARD_INTERFACE: u8 = 0xAE;

    //const READ_INPUT_PORT: u8 = 0xC0;
    //const POLL_INPUT_PORT_LOW: u8 = 0xC1;
    //const READ_OUTPUT_PORT: u8 = 0xD0;
    //const WRITE_OUTPUT_PORT: u8 = 0xD1;
    //const WRITE_KEYBOARD_OUTPUT_BUFFER: u8 = 0xD2;
    //const WRITE_AUXILIARY_DEVICE_OUTPUT_BUFFER: u8 = 0xD3;
    pub const WRITE_TO_AUXILIARY_DEVICE: u8 = 0xD4;
    //const READ_TEST_INPUTS: u8 = 0xE0;
    //const PULSE_OUTPUT_PORT: u8 = 0xF0;
}

bitflags! {
    pub struct ControllerCommandByte: u8 {
        const KEYBOARD_TRANSLATE_MODE = 0b0100_0000;
        const DISABLE_AUXILIARY_DEVICE = 0b0010_0000;
        const DISABLE_KEYBOARD = 0b0001_0000;
        const SYSTEM_FLAG = 0b0000_0100;
        const ENABLE_AUXILIARY_INTERRUPT = 0b0000_0010;
        const ENABLE_KEYBOARD_INTERRUPT = 0b0000_0001;
    }
}

bitflags! {
    pub struct InputPortBits: u8 {
        const AUXILIARY_DATA_IN = 0b0000_0010;
        const KEYBOARD_DATA_IN = 0b0000_0001;
    }
}

bitflags! {
    pub struct OutputPortBits: u8 {
        const KEYBOARD_DATA_OUT = 0b1000_0000;
        const KEYBOARD_CLOCK_OUT = 0b0100_0000;
        const IRQ12 = 0b0010_0000;
        const IRQ1 = 0b0001_0000;
        const AUXILIARY_CLOCK_OUT = 0b0000_1000;
        const AUXILIARY_DATA_OUT = 0b0000_0100;
        const GATE_ADDRESS_LINE_20 = 0b0000_0010;
        const RESET_MICROPROCESSOR = 0b0000_0001;
    }
}
