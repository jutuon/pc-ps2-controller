//! Read controller status register.

use crate::controller::{
    raw::StatusRegister,
    io::{ PortIO, PortIOAvailable },
};

pub struct StatusInfo {
    register: StatusRegister,
}

pub struct OddParity;
pub struct EvenParity;

pub enum DataOwner {
    KeyboardOrCommandController,
    AuxiliaryDevice,
}

pub enum PasswordState {
    Active,
    Inactive,
}

impl StatusInfo {
    pub fn keyboard_data_parity(&self) -> Result<OddParity, EvenParity> {
        if self.register.contains(StatusRegister::KEYBOARD_PARITY_ERROR) {
            Err(EvenParity)
        } else {
            Ok(OddParity)
        }
    }

    /// If `true` then there is a general timeout error.
    pub fn general_timeout_error(&self) -> bool {
        self.register.contains(StatusRegister::GENERAL_TIMEOUT)
    }

    /// If `Some(_)` there is new data available to read from the controller.
    pub fn data_availability(&self) -> Option<DataOwner> {
        if self.register.contains(
            StatusRegister::AUXILIARY_DEVICE_OUTPUT_BUFFER_FULL |
            StatusRegister::OUTPUT_BUFFER_FULL
        ) {
            Some(DataOwner::AuxiliaryDevice)
        } else if self.register.contains(StatusRegister::OUTPUT_BUFFER_FULL) {
            Some(DataOwner::KeyboardOrCommandController)
        } else {
            None
        }
    }

    pub fn password_state(&self) -> PasswordState {
        if self.register.contains(StatusRegister::INHIBIT_SWITCH) {
            PasswordState::Active
        } else {
            PasswordState::Inactive
        }
    }

    pub fn system_flag(&self) -> bool {
        self.register.contains(StatusRegister::SYSTEM_FLAG)
    }

    /// There is data that controller has not handled yet.
    pub fn input_buffer_full(&self) -> bool {
        self.register.contains(StatusRegister::INPUT_BUFFER_FULL)
    }

    pub fn raw(&self) -> StatusRegister {
        self.register
    }
}


pub trait ReadStatus<T: PortIO>: PortIOAvailable<T> {
    fn status(&mut self) -> StatusInfo {
        let raw = self.port_io_mut().read(T::STATUS_REGISTER);

        StatusInfo {
            register: StatusRegister::from_bits_truncate(raw),
        }
    }
}
