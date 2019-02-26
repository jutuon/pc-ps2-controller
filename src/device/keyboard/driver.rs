


use crate::device::command_queue::{CommandQueue, Command, Status};
use crate::device::io::SendToDevice;

use core::fmt;

use super::raw::{FromKeyboard, StatusIndicators, CommandSetAllKeys, CommandSetKeyType};

use arraydeque::{Array};


pub use pc_keyboard;

use pc_keyboard::{KeyEvent, ScancodeSet2, ScancodeSet1, layouts, Error, HandleControl, Keyboard as KeyboardScancodeDecoder };


pub struct Keyboard<T: Array<Item=Command>> {
    commands: CommandQueue<T>,
    state: State,
    scancode_reader: ScancodeDecoder,
}

impl <T: Array<Item=Command>> fmt::Debug for Keyboard<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Keyboard")
    }
}


impl <T: Array<Item=Command>> Keyboard<T> {
    pub fn new<U: SendToDevice>(device: &mut U) -> Result<Self, NotEnoughSpaceInTheCommandQueue> {
        let mut keyboard = Self {
            commands: CommandQueue::new(),
            state: State::ScancodesDisabled,
            scancode_reader: ScancodeDecoder::new(),
        };

        keyboard.set_defaults_and_disable(device)?;

        Ok(keyboard)
    }

    pub fn set_defaults_and_disable<U: SendToDevice>(&mut self, device: &mut U) -> Result<(), NotEnoughSpaceInTheCommandQueue> {
        if self.commands.space_available(1) {
            self.state = State::ScancodesDisabled;
            self.commands.add(Command::default_disable(), device).unwrap();
            Ok(())
        } else {
            Err(NotEnoughSpaceInTheCommandQueue)
        }
    }

    pub fn set_defaults_and_enable<U: SendToDevice>(&mut self, device: &mut U) -> Result<(), NotEnoughSpaceInTheCommandQueue> {
        if self.commands.space_available(1) {
            self.state = State::ScancodesEnabled;
            self.commands.add(Command::set_default(), device).unwrap();
            Ok(())
        } else {
            Err(NotEnoughSpaceInTheCommandQueue)
        }
    }

    pub fn enable<U: SendToDevice>(&mut self, device: &mut U) -> Result<(), NotEnoughSpaceInTheCommandQueue> {
        if self.commands.space_available(1) {
            self.state = State::ScancodesEnabled;
            self.commands.add(Command::enable(), device).unwrap();
            Ok(())
        } else {
            Err(NotEnoughSpaceInTheCommandQueue)
        }
    }

    pub fn set_status_indicators<U: SendToDevice>(&mut self, device: &mut U, indicators: StatusIndicators) -> Result<(), NotEnoughSpaceInTheCommandQueue> {
        if self.commands.space_available(1) {
            self.commands.add(Command::set_status_indicators(indicators.bits()), device).unwrap();
            Ok(())
        } else {
            Err(NotEnoughSpaceInTheCommandQueue)
        }
    }

    pub fn scancode_set_3_set_all_keys<U: SendToDevice>(&mut self, device: &mut U, set_all_keys: SetAllKeys) -> Result<(), NotEnoughSpaceInTheCommandQueue> {
        if self.commands.space_available(1) {
            self.commands.add(Command::scancode_set_3_set_all_keys(set_all_keys), device).unwrap();
            Ok(())
        } else {
            Err(NotEnoughSpaceInTheCommandQueue)
        }
    }

    pub fn scancode_set_3_set_key_type<U: SendToDevice>(&mut self, device: &mut U, set_key_type: SetKeyType, scancode: u8) -> Result<(), NotEnoughSpaceInTheCommandQueue> {
        if self.commands.space_available(1) {
            self.commands.add(Command::scancode_set_3_set_key_type(set_key_type, scancode), device).unwrap();
            Ok(())
        } else {
            Err(NotEnoughSpaceInTheCommandQueue)
        }
    }

    pub fn set_scancode_decoder(&mut self, setting: ScancodeDecoderSetting) {
        self.scancode_reader.change_decoder(setting)
    }

    pub fn set_typematic_rate<U: SendToDevice>(&mut self, device: &mut U, delay: DelayMilliseconds, rate: RateValue) -> Result<(), NotEnoughSpaceInTheCommandQueue> {
        if self.commands.space_available(1) {
            self.commands.add(Command::set_typematic_rate(delay, rate), device).unwrap();
            Ok(())
        } else {
            Err(NotEnoughSpaceInTheCommandQueue)
        }
    }

    pub fn receive_data<U: SendToDevice>(&mut self, new_data: u8, device: &mut U) -> Result<Option<KeyboardEvent>, KeyboardError> {
        match new_data {
            FromKeyboard::KEY_DETECTION_OVERRUN_SCANCODE_SET_2_AND_3 => return Err(KeyboardError::KeyDetectionError),
            FromKeyboard::BAT_FAILURE_CODE => return Err(KeyboardError::BATCompletionFailure),
            FromKeyboard::BAT_COMPLETION_CODE => {
                self.state = State::ScancodesEnabled;
                return Ok(Some(KeyboardEvent::BATCompleted));
            },
            _ => (),
        }

        if self.commands.empty() {
            if new_data == FromKeyboard::RESEND {
                return Ok(None);
            }

            self.scancode_reader.decode(new_data).map(|o| o.map(|e| KeyboardEvent::Key(e))).map_err(|e| KeyboardError::ScancodeParsingError(e))
        } else {
            if let Some(Status::UnexpectedData(data)) = self.commands.receive_data(new_data, device) {
                self.scancode_reader.decode(data).map(|o| o.map(|e| KeyboardEvent::Key(e))).map_err(|e| KeyboardError::ScancodeParsingError(e))
            } else {
                Ok(None)
            }
        }
    }
}

#[derive(Debug)]
pub struct ScancodeDecoder {
    current_decoder: Decoder,
}

impl ScancodeDecoder {
    /// Defaults to scancode set 2.
    pub fn new() -> Self {
        Self {
            current_decoder: Decoder::Set2(KeyboardScancodeDecoder::new(layouts::Us104Key, ScancodeSet2, HandleControl::Ignore)),
        }
    }

    pub fn decode(&mut self, scancode: u8) -> Result<Option<KeyEvent>, Error> {
        match &mut self.current_decoder {
            Decoder::Set1(decoder) => decoder.add_byte(scancode),
            Decoder::Set2(decoder) => decoder.add_byte(scancode),
        }
    }

    pub fn change_decoder(&mut self, setting: ScancodeDecoderSetting) {
        match setting {
            ScancodeDecoderSetting::Set1 => self.current_decoder = Decoder::Set1(KeyboardScancodeDecoder::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)),
            ScancodeDecoderSetting::Set2 => self.current_decoder = Decoder::Set2(KeyboardScancodeDecoder::new(layouts::Us104Key, ScancodeSet2, HandleControl::Ignore)),
        }
    }
}

enum Decoder {
    Set1(KeyboardScancodeDecoder<layouts::Us104Key, ScancodeSet1>),
    Set2(KeyboardScancodeDecoder<layouts::Us104Key, ScancodeSet2>),
}

impl fmt::Debug for Decoder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decoder")
    }
}

#[derive(Debug)]
pub enum ScancodeDecoderSetting {
    Set1,
    Set2,
}

#[derive(Debug)]
pub enum KeyboardError {
    KeyDetectionError,
    BATCompletionFailure,
    ScancodeParsingError(Error),
}

#[derive(Debug)]
pub enum KeyboardEvent {
    Key(KeyEvent),
    BATCompleted,
}

#[derive(Debug)]
pub struct NotEnoughSpaceInTheCommandQueue;

#[derive(Debug)]
enum State {
    ScancodesDisabled,
    ScancodesEnabled,
}


#[derive(Debug)]
#[repr(u8)]
pub enum SetAllKeys {
    Typematic = CommandSetAllKeys::TYPEMATIC,
    MakeSlashBreak = CommandSetAllKeys::MAKE_SLASH_BREAK,
    Make = CommandSetAllKeys::MAKE,
    TypematicSlashMakeSlashBreak = CommandSetAllKeys::TYPEMATIC_SLASH_MAKE_SLASH_BREAK,
}

#[derive(Debug)]
#[repr(u8)]
pub enum SetKeyType {
    Typematic = CommandSetKeyType::TYPEMATIC,
    MakeSlashBreak = CommandSetKeyType::MAKE_SLASH_BREAK,
    Make = CommandSetKeyType::MAKE,
}

#[derive(Debug)]
#[repr(u8)]
pub enum DelayMilliseconds {
    Delay250 = 0,
    /// Default value.
    Delay500 = 0b0010_0000,
    Delay750 = 0b0100_0000,
    Delay1000 = 0b0110_0000,
}

#[derive(Debug)]
pub struct RateValue(u8);

impl RateValue {

    /// 30 Hz
    pub const RATE_MAX: RateValue = RateValue(0);

    /// 2 Hz
    pub const RATE_MIN: RateValue = RateValue(0b0001_1111);

    /// 10,9 Hz
    pub const RATE_DEFAULT: RateValue = RateValue(0b0000_1011);

    /// Create new `RateValue`.
    ///
    /// # Panics
    /// If `value & !0b0001_1111 != 0`.
    pub fn new(value: u8) -> Self {
        if value & !0b0001_1111 != 0 {
            panic!("rate value is out of range. '{} & !0b0001_1111 != 0'", value);
        }

        RateValue(value)
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}
