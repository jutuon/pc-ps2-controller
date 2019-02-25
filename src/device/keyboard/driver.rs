


use crate::device::command_queue::{CommandQueue, Command, Status};
use crate::device::io::SendToDevice;


use super::raw::FromKeyboard;

use arraydeque::{Array};


pub use pc_keyboard;

use pc_keyboard::{KeyEvent, ScancodeSet2, layouts, Error };


pub struct Keyboard<T: Array<Item=Command>> {
    commands: CommandQueue<T>,
    state: State,
    scancode_reader: pc_keyboard::Keyboard<layouts::Us104Key, ScancodeSet2>,
}


impl <T: Array<Item=Command>> Keyboard<T> {
    pub fn new<U: SendToDevice>(device: &mut U) -> Result<Self, NotEnoughSpaceInTheCommandQueue> {
        let mut keyboard = Self {
            commands: CommandQueue::new(),
            state: State::ScancodesDisabled,
            scancode_reader: pc_keyboard::Keyboard::new(layouts::Us104Key, ScancodeSet2),
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

            self.scancode_reader.add_byte(new_data).map(|o| o.map(|e| KeyboardEvent::Key(e))).map_err(|e| KeyboardError::ScancodeParsingError(e))
        } else {
            if let Some(Status::UnexpectedData(data)) = self.commands.receive_data(new_data, device) {
                self.scancode_reader.add_byte(data).map(|o| o.map(|e| KeyboardEvent::Key(e))).map_err(|e| KeyboardError::ScancodeParsingError(e))
            } else {
                Ok(None)
            }
        }
    }
}

pub enum KeyboardError {
    KeyDetectionError,
    BATCompletionFailure,
    ScancodeParsingError(Error),
}

pub enum KeyboardEvent {
    Key(KeyEvent),
    BATCompleted,
}

pub struct NotEnoughSpaceInTheCommandQueue;

enum State {
    ScancodesDisabled,
    ScancodesEnabled,
}
