

use core::marker::PhantomData;

use crate::controller::{
    io::{ PortIO, PortIOAvailable },
    driver::*,
    driver::status::ReadStatus,
};

/// Bypass state machine encoded to the types. This should be used
/// only for debugging purposes.
pub struct DebugMode<'a, T: PortIO, U: PortIOAvailable<T>>(PhantomData<T>, &'a mut U);


impl<'a, T: PortIO, U: PortIOAvailable<T>> DebugMode<'a, T, U> {
    pub fn new(controller: &'a mut U) -> Self {
        DebugMode(PhantomData, controller)
    }

    pub fn send_controller_command_and_wait_processing(&mut self, command: u8) {
        send_controller_command_and_wait_processing(self, command);
    }

    pub fn send_controller_command_and_write_data(&mut self, command: u8, data: u8) {
        send_controller_command_and_write_data(self, command, data);
    }

    pub fn write_controller_command_byte(&mut self, data: ControllerCommandByte) {
        write_controller_command_byte(self, data);
    }

    pub fn send_controller_command_and_wait_response(&mut self, command: u8) -> u8 {
        send_controller_command_and_wait_response(self, command)
    }
}

impl_port_io_available!(<T: PortIO, U: PortIOAvailable<T>> DebugMode<'_, T, U>);

impl <T: PortIO, U: PortIOAvailable<T>> ReadStatus<T> for DebugMode<'_, T, U> {}
impl <T: PortIO, U: PortIOAvailable<T>> InterruptsDisabled for DebugMode<'_, T, U> {}
impl <T: PortIO, U: PortIOAvailable<T>> KeyboardDisabled for DebugMode<'_, T, U> {}
impl <T: PortIO, U: PortIOAvailable<T>> AuxiliaryDeviceDisabled for DebugMode<'_, T, U> {}
impl <T: PortIO, U: PortIOAvailable<T>> ReadRAM<T> for DebugMode<'_, T, U> {}
impl <T: PortIO, U: PortIOAvailable<T>> WriteRAM<T> for DebugMode<'_, T, U> {}
impl <T: PortIO, U: PortIOAvailable<T>> Testing<T> for DebugMode<'_, T, U> {}
impl <T: PortIO, U: PortIOAvailable<T>> ResetCPU<T> for DebugMode<'_, T, U> {}

