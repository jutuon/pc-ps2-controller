
use super::io::SendToDevice;
use super::keyboard::driver::{SetAllKeys, SetKeyType, DelayMilliseconds, RateValue, KeyboardScancodeSetting};
use super::keyboard::raw::{ FromKeyboard, CommandReturnData };

use arraydeque::{Array, Saturating, ArrayDeque, CapacityError};

#[derive(Debug)]
pub struct CommandQueue<T: Array<Item=Command>> {
    commands: ArrayDeque<T, Saturating>,
    command_checker: CommandChecker,
}

impl <T: Array<Item=Command>> CommandQueue<T> {
    pub fn new() -> Self {
        Self {
            commands: ArrayDeque::new(),
            command_checker: CommandChecker::new(),
        }
    }

    pub fn space_available(&self, count: usize) -> bool {
        (self.commands.capacity() - self.commands.len()) >= count
    }

    pub fn add<U: SendToDevice>(&mut self, command: Command, device: &mut U) -> Result<(), CapacityError<Command>> {
        let result = self.commands.push_back(command);

        if self.command_checker.current_command().is_none() {
            if let Some(command) = self.commands.pop_front() {
                self.command_checker.send_new_command(command, device)
            }
        }

        result
    }

    /// Receive data only if command queue is not empty.
    pub fn receive_data<U: SendToDevice>(&mut self, new_data: u8, device: &mut U) -> Option<Status> {
        let result = self.command_checker.receive_data(new_data, device);

        if let Some(Status::CommandFinished(_)) = &result {
            if let Some(command) = self.commands.pop_front() {
                self.command_checker.send_new_command(command, device);
            }
        }

        result
    }

    pub fn empty(&self) -> bool {
        self.commands.len() == 0 && self.command_checker.current_command().is_none()
    }
}

#[derive(Debug)]
pub struct CommandChecker {
    current_command: Option<Command>,
}

impl CommandChecker {
    pub fn new() -> Self {
        Self {
            current_command: None,
        }
    }

    pub fn current_command(&self) -> &Option<Command> {
        &self.current_command
    }

    pub fn send_new_command<T: SendToDevice>(&mut self, command: Command, device: &mut T) {
        match &command {
            Command::Echo { command } |
            Command::AckResponse { command, ..} |
            Command::AckResponseWithReturnTwoBytes { command, ..} |
            Command::SendCommandAndData {command, .. } |
            Command::SendCommandAndDataSingleAck {command, .. } |
            Command::SendCommandAndDataAndReceiveResponse {command, .. }   => device.send(*command)
        }

        self.current_command = Some(command);
    }

    pub fn receive_data<U: SendToDevice>(&mut self, new_data: u8, device: &mut U) -> Option<Status> {
        if let Some(mut command) = self.current_command.take() {
            let mut command_finished = false;
            let mut unexpected_data = None;

            match &mut command {
                Command::Echo { .. } => {
                    if new_data == FromKeyboard::ECHO {
                        command_finished = true;
                    } else if new_data == FromKeyboard::RESEND {
                        self.send_new_command(command, device);
                        return None;
                    } else {
                        unexpected_data = Some(new_data);
                    }
                }
                Command::AckResponse { .. } => {
                    if new_data == FromKeyboard::ACK {
                        command_finished = true;
                    } else if new_data == FromKeyboard::RESEND {
                        self.send_new_command(command, device);
                        return None;
                    } else {
                        unexpected_data = Some(new_data);
                    }
                },
                Command::AckResponseWithReturnTwoBytes { state: s @ AckResponseWithReturnTwoBytesState::WaitAck, .. } => {
                    if new_data == FromKeyboard::ACK {
                        *s = AckResponseWithReturnTwoBytesState::WaitFirstByte;
                    } else if new_data == FromKeyboard::RESEND {
                        self.send_new_command(command, device);
                        return None;
                    } else {
                        unexpected_data = Some(new_data);
                    }
                }
                Command::AckResponseWithReturnTwoBytes { state: s @ AckResponseWithReturnTwoBytesState::WaitFirstByte, byte1, .. } => {
                    *s = AckResponseWithReturnTwoBytesState::WaitSecondByte;
                    *byte1 = new_data;
                }
                Command::AckResponseWithReturnTwoBytes { state: AckResponseWithReturnTwoBytesState::WaitSecondByte, byte2, .. } => {
                    *byte2 = new_data;
                    command_finished = true;
                }
                Command::SendCommandAndData { state: s @ SendCommandAndDataState::WaitAck1, data, .. } => {
                    if new_data == FromKeyboard::ACK {
                        *s = SendCommandAndDataState::WaitAck2;
                        device.send(*data);
                    } else if new_data == FromKeyboard::RESEND {
                        self.send_new_command(command, device);
                        return None;
                    } else {
                        unexpected_data = Some(new_data);
                    }
                }
                Command::SendCommandAndData { state: SendCommandAndDataState::WaitAck2, data, .. } => {
                    if new_data == FromKeyboard::ACK {
                        command_finished = true;
                    } else if new_data == FromKeyboard::RESEND {
                        device.send(*data);
                    } else {
                        unexpected_data = Some(new_data);
                    }
                }
                Command::SendCommandAndDataSingleAck { state: s @ SendCommandAndDataState::WaitAck1, data, .. } => {
                    if new_data == FromKeyboard::ACK {
                        *s = SendCommandAndDataState::WaitAck2;
                        device.send(*data);
                    } else if new_data == FromKeyboard::RESEND {
                        self.send_new_command(command, device);
                        return None;
                    } else {
                        unexpected_data = Some(new_data);
                    }
                }
                Command::SendCommandAndDataSingleAck { state: SendCommandAndDataState::WaitAck2, data, scancode_received_after_this_command, .. } => {
                    if new_data == FromKeyboard::RESEND {
                        device.send(*data);
                    } else {
                        *scancode_received_after_this_command = new_data;
                        command_finished = true;
                    }
                }
                Command::SendCommandAndDataAndReceiveResponse { state: s @ SendCommandAndDataAndReceiveResponseState::WaitAck1, data, .. } => {
                    if new_data == FromKeyboard::ACK {
                        *s = SendCommandAndDataAndReceiveResponseState::WaitAck2;
                        device.send(*data);
                    } else if new_data == FromKeyboard::RESEND {
                        self.send_new_command(command, device);
                        return None;
                    } else {
                        unexpected_data = Some(new_data);
                    }
                }
                Command::SendCommandAndDataAndReceiveResponse { state: s @ SendCommandAndDataAndReceiveResponseState::WaitAck2, data, .. } => {
                    if new_data == FromKeyboard::ACK {
                        *s = SendCommandAndDataAndReceiveResponseState::WaitResponse;
                    } else if new_data == FromKeyboard::RESEND {
                        device.send(*data);
                    }
                }
                Command::SendCommandAndDataAndReceiveResponse { state: SendCommandAndDataAndReceiveResponseState::WaitResponse, response, .. } => {
                    *response = new_data;
                    command_finished = true;
                }
            }

            if command_finished {
                Some(Status::CommandFinished(command))
            } else {
                self.current_command = Some(command);

                if let Some(data) = unexpected_data {
                    Some(Status::UnexpectedData(data))
                } else {
                    Some(Status::CommandInProgress)
                }
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Echo {
        command: u8,
    },
    AckResponse {
        command: u8,
    },
    AckResponseWithReturnTwoBytes { command: u8, byte1: u8, byte2: u8, state: AckResponseWithReturnTwoBytesState },
    SendCommandAndData { command: u8, data: u8, state: SendCommandAndDataState },
    SendCommandAndDataSingleAck { command: u8, data: u8, scancode_received_after_this_command: u8, state: SendCommandAndDataState },
    SendCommandAndDataAndReceiveResponse { command: u8, data: u8, response: u8, state: SendCommandAndDataAndReceiveResponseState },
}

impl Command {
    pub fn default_disable() -> Self {
        Command::AckResponse {
            command: CommandReturnData::DEFAULT_DISABLE,
        }
    }

    pub fn set_default() -> Self {
        Command::AckResponse {
            command: CommandReturnData::SET_DEFAULT,
        }
    }

    pub fn read_id() -> Self {
        Command::AckResponseWithReturnTwoBytes { command: CommandReturnData::READ_ID, byte1: 0, byte2: 0, state: AckResponseWithReturnTwoBytesState::WaitAck }
    }

    pub fn enable() -> Self {
        Command::AckResponse {
            command: CommandReturnData::ENABLE,
        }
    }

    pub fn set_status_indicators(data: u8) -> Self {
        Command::SendCommandAndData {
            command: CommandReturnData::SET_STATUS_INDICATORS,
            data,
            state: SendCommandAndDataState::WaitAck1,
        }
    }

    pub fn scancode_set_3_set_all_keys(command: SetAllKeys) -> Self {
        Command::AckResponse {
            command: command as u8,
        }
    }

    pub fn scancode_set_3_set_key_type(command: SetKeyType, scancode: u8) -> Self {
        Command::SendCommandAndDataSingleAck {
            command: command as u8,
            data: scancode,
            scancode_received_after_this_command: 0,
            state: SendCommandAndDataState::WaitAck1,
        }
    }

    pub fn set_typematic_rate(delay: DelayMilliseconds, rate: RateValue) -> Self {
        Command::SendCommandAndData {
            command: CommandReturnData::SET_TYPEMATIC_RATE,
            data: delay as u8 | rate.value(),
            state: SendCommandAndDataState::WaitAck1,
        }
    }

    pub fn set_alternate_scancodes(scancode_setting: KeyboardScancodeSetting) -> Self {
        Command::SendCommandAndData {
            command: CommandReturnData::SELECT_ALTERNATE_SCANCODES,
            data: scancode_setting as u8,
            state: SendCommandAndDataState::WaitAck1,
        }
    }

    pub fn get_current_scancode_set() -> Self {
        Command::SendCommandAndDataAndReceiveResponse {
            command: CommandReturnData::SELECT_ALTERNATE_SCANCODES,
            data: 0,
            response: 0,
            state: SendCommandAndDataAndReceiveResponseState::WaitAck1,
        }
    }

    pub fn echo() -> Self {
        Command::Echo {
            command: CommandReturnData::ECHO,
        }
    }
}

#[derive(Debug)]
pub enum Status {
    UnexpectedData(u8),
    CommandInProgress,
    CommandFinished(Command),
}

#[derive(Debug)]
pub enum AckResponseWithReturnTwoBytesState {
    WaitAck,
    WaitFirstByte,
    WaitSecondByte,
}

#[derive(Debug)]
pub enum SendCommandAndDataState {
    WaitAck1,
    WaitAck2,
}

#[derive(Debug)]
pub enum SendCommandAndDataAndReceiveResponseState {
    WaitAck1,
    WaitAck2,
    WaitResponse,
}