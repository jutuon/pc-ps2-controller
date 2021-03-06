pub mod debug;
pub mod marker;
pub mod status;

use marker::*;
use status::{DataOwner, ReadStatus};

use super::{io::*, raw::*};

use core::marker::PhantomData;

#[derive(Debug)]
pub struct InitController<T: PortIO>(T);

impl<T: PortIO> InitController<T> {
    /// You should disable interrupts before starting the initialization
    /// process.
    pub fn start_init(port_io: T) -> DevicesDisabled<T> {
        let mut controller = DevicesDisabled(port_io);

        controller.dangerous_disable_auxiliary_device_interface();
        controller.dangerous_disable_keyboard_interface();

        let raw_command_byte = send_controller_command_and_wait_response(
            &mut controller,
            CommandReturnData::READ_CONTROLLER_COMMAND_BYTE,
        );

        let mut command_byte = ControllerCommandByte::from_bits_truncate(raw_command_byte);
        command_byte.set(ControllerCommandByte::ENABLE_AUXILIARY_INTERRUPT, false);
        command_byte.set(ControllerCommandByte::ENABLE_KEYBOARD_INTERRUPT, false);

        write_controller_command_byte(&mut controller, command_byte);

        controller
    }
}

#[derive(Debug)]
pub enum InterfaceError {
    Keyboard(DeviceInterfaceError),
    AuxiliaryDevice(DeviceInterfaceError),
}

#[derive(Debug)]
pub struct DevicesDisabled<T: PortIO>(T);

impl<T: PortIO> DevicesDisabled<T> {
    pub fn scancode_translation(&mut self, enabled: bool) {
        let mut command_byte = self.controller_command_byte();
        command_byte.set(ControllerCommandByte::KEYBOARD_TRANSLATE_MODE, enabled);
        write_controller_command_byte(self, command_byte);
    }

    pub fn enable_devices(
        mut self,
        devices: EnableDevice,
    ) -> Result<EnabledDevices<T, Disabled>, (Self, InterfaceError)> {
        match self.test_devices(devices) {
            Ok(()) => Ok(self.configure(devices, false)),
            Err(e) => Err((self, e)),
        }
    }

    pub fn enable_devices_and_interrupts(
        mut self,
        devices: EnableDevice,
    ) -> Result<EnabledDevices<T, InterruptsEnabled>, (Self, InterfaceError)> {
        match self.test_devices(devices) {
            Ok(()) => Ok(self.configure(devices, true)),
            Err(e) => Err((self, e)),
        }
    }

    fn test_devices(&mut self, devices: EnableDevice) -> Result<(), InterfaceError> {
        match &devices {
            EnableDevice::Keyboard => self.test_keyboard(),
            EnableDevice::AuxiliaryDevice => self.test_auxiliary_device(),
            EnableDevice::KeyboardAndAuxiliaryDevice => self.test_keyboard_and_auxiliary_device(),
        }
    }

    fn test_auxiliary_device(&mut self) -> Result<(), InterfaceError> {
        self.auxiliary_device_interface_test()
            .map_err(InterfaceError::AuxiliaryDevice)
    }

    fn test_keyboard(&mut self) -> Result<(), InterfaceError> {
        self.keyboard_interface_test()
            .map_err(InterfaceError::Keyboard)
    }

    fn test_keyboard_and_auxiliary_device(&mut self) -> Result<(), InterfaceError> {
        self.test_keyboard().and(self.test_auxiliary_device())
    }

    fn configure<IRQ>(mut self, devices: EnableDevice, interrupts: bool) -> EnabledDevices<T, IRQ> {
        match &devices {
            EnableDevice::Keyboard => self.dangerous_enable_keyboard_interface(),
            EnableDevice::AuxiliaryDevice => self.dangerous_enable_auxiliary_device(),
            EnableDevice::KeyboardAndAuxiliaryDevice => {
                self.dangerous_enable_keyboard_interface();
                self.dangerous_enable_auxiliary_device();
            }
        }

        if interrupts {
            let mut command_byte = self.controller_command_byte();

            match &devices {
                EnableDevice::Keyboard => {
                    command_byte.set(ControllerCommandByte::ENABLE_KEYBOARD_INTERRUPT, true)
                }
                EnableDevice::AuxiliaryDevice => {
                    command_byte.set(ControllerCommandByte::ENABLE_AUXILIARY_INTERRUPT, true)
                }
                EnableDevice::KeyboardAndAuxiliaryDevice => {
                    command_byte.set(ControllerCommandByte::ENABLE_KEYBOARD_INTERRUPT, true);
                    command_byte.set(ControllerCommandByte::ENABLE_AUXILIARY_INTERRUPT, true);
                }
            }

            write_controller_command_byte(&mut self, command_byte);
        }

        EnabledDevices {
            port_io: self.0,
            _marker: PhantomData,
            devices,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum EnableDevice {
    Keyboard,
    AuxiliaryDevice,
    KeyboardAndAuxiliaryDevice,
}

impl_port_io_available!(<T: PortIO> DevicesDisabled<T>);

impl<T: PortIO> ReadStatus<T> for DevicesDisabled<T> {}
impl<T: PortIO> DangerousDeviceCommands<T> for DevicesDisabled<T> {}
impl<T: PortIO> InterruptsDisabled for DevicesDisabled<T> {}
impl<T: PortIO> KeyboardDisabled for DevicesDisabled<T> {}
impl<T: PortIO> AuxiliaryDeviceDisabled for DevicesDisabled<T> {}
impl<T: PortIO> ReadRAM<T> for DevicesDisabled<T> {}
impl<T: PortIO> WriteRAM<T> for DevicesDisabled<T> {}
impl<T: PortIO> Testing<T> for DevicesDisabled<T> {}
impl<T: PortIO> ResetCPU<T> for DevicesDisabled<T> {}

#[derive(Debug)]
pub struct EnabledDevices<T: PortIO, IRQ> {
    port_io: T,
    _marker: PhantomData<IRQ>,
    devices: EnableDevice,
}

impl<T: PortIO, IRQ> EnabledDevices<T, IRQ> {
    pub fn send_to_auxiliary_device(&mut self, data: u8) -> Result<(), ()> {
        match &self.devices {
            EnableDevice::AuxiliaryDevice | EnableDevice::KeyboardAndAuxiliaryDevice => {
                send_controller_command_and_write_data(
                    self,
                    CommandWaitData::WRITE_TO_AUXILIARY_DEVICE,
                    data,
                );
                Ok(())
            }
            EnableDevice::Keyboard => Err(()),
        }
    }

    pub fn send_to_keyboard(&mut self, data: u8) -> Result<(), ()> {
        match &self.devices {
            EnableDevice::Keyboard | EnableDevice::KeyboardAndAuxiliaryDevice => {
                while self.status().input_buffer_full() {}
                self.port_io_mut().write(T::DATA_PORT, data);
                Ok(())
            }
            EnableDevice::AuxiliaryDevice => Err(()),
        }
    }
}

impl<T: PortIO> EnabledDevices<T, InterruptsEnabled> {
    /// You should disable the interrupts before disabling
    /// the devices.
    pub fn disable_devices(self) -> DevicesDisabled<T> {
        InitController::start_init(self.port_io)
    }
}

impl<T: PortIO> EnabledDevices<T, Disabled> {
    pub fn disable_devices(mut self) -> DevicesDisabled<T> {
        self.dangerous_disable_auxiliary_device_interface();
        self.dangerous_disable_keyboard_interface();

        DevicesDisabled(self.port_io)
    }
}

impl_port_io_available!(<T: PortIO, IRQ> EnabledDevices<T, IRQ>);

impl<T: PortIO, IRQ> ReadStatus<T> for EnabledDevices<T, IRQ> {}
impl<T: PortIO, IRQ> ReadData<T> for EnabledDevices<T, IRQ> {}
impl<T: PortIO, IRQ> ResetCPU<T> for EnabledDevices<T, IRQ> {}

impl<T: PortIO> DangerousDeviceCommands<T> for EnabledDevices<T, Disabled> {}

#[derive(Debug)]
pub enum DeviceInterfaceError {
    ClockLineLow,
    ClockLineHigh,
    DataLineLow,
    DataLineHigh,
    UnknownValue(u8),
}

impl DeviceInterfaceError {
    fn from_test_result(value: u8) -> Result<(), DeviceInterfaceError> {
        use DeviceInterfaceError::*;
        let result = match value {
            0 => return Ok(()),
            1 => ClockLineLow,
            2 => ClockLineHigh,
            3 => DataLineLow,
            4 => DataLineHigh,
            _ => UnknownValue(value),
        };

        Err(result)
    }
}

// TODO: The IBM reference (PDF page 344) says that there
//       shouldn't be any writes to ports 0x60 and 0x64 when
//       output buffer bit is set to 1. This is probably unnecessary
//       when controller command doesn't use the output buffer?
//       The current code checks that the buffer is empty only when using
//       commands that return a value.

fn send_controller_command_and_wait_processing<T: PortIO, U: ReadStatus<T>>(
    controller: &mut U,
    command: u8,
) {
    while controller.status().input_buffer_full() {}
    controller.port_io_mut().write(T::COMMAND_REGISTER, command);
    while controller.status().input_buffer_full() {}
}

fn send_controller_command_and_write_data<T: PortIO, U: ReadStatus<T>>(
    controller: &mut U,
    command: u8,
    data: u8,
) {
    send_controller_command_and_wait_processing(controller, command);
    controller.port_io_mut().write(T::DATA_PORT, data);
}

fn write_controller_command_byte<T: PortIO, U: ReadStatus<T>>(
    controller: &mut U,
    data: ControllerCommandByte,
) {
    send_controller_command_and_write_data(
        controller,
        CommandWaitData::WRITE_CONTROLLER_COMMAND_BYTE,
        data.bits(),
    )
}

fn send_controller_command_and_wait_response<
    T: PortIO,
    U: ReadStatus<T> + InterruptsDisabled + KeyboardDisabled + AuxiliaryDeviceDisabled,
>(
    controller: &mut U,
    command: u8,
) -> u8 {
    if controller.status().data_availability().is_some() {
        controller.port_io_mut().read(T::DATA_PORT);
    }

    send_controller_command_and_wait_processing(controller, command);

    loop {
        if let Some(DataOwner::KeyboardOrCommandController) =
            controller.status().data_availability()
        {
            return controller.port_io_mut().read(T::DATA_PORT);
        }
    }
}

pub trait ReadRAM<T: PortIO>:
    ReadStatus<T> + InterruptsDisabled + KeyboardDisabled + AuxiliaryDeviceDisabled + Sized
{
    fn controller_command_byte(&mut self) -> ControllerCommandByte {
        let raw = send_controller_command_and_wait_response(
            self,
            CommandReturnData::READ_CONTROLLER_COMMAND_BYTE,
        );
        ControllerCommandByte::from_bits_truncate(raw)
    }

    fn ram(&mut self, data: &mut [u8; CONTROLLER_RAM_SIZE]) {
        for (i, byte) in data.iter_mut().enumerate() {
            let data = send_controller_command_and_wait_response(
                self,
                CommandReturnData::READ_RAM_START + i as u8,
            );
            *byte = data;
        }
    }
}

pub trait WriteRAM<T: PortIO>: ReadStatus<T> + Sized {
    fn write_ram(&mut self, data: &mut [u8; CONTROLLER_RAM_SIZE]) {
        for (i, byte) in data.iter().enumerate() {
            send_controller_command_and_write_data(
                self,
                CommandWaitData::WRITE_RAM_START + i as u8,
                *byte,
            );
        }
    }
}

/// Commands which may break invariants which are encoded
/// to the types.
trait DangerousDeviceCommands<T: PortIO>: ReadStatus<T> + Sized {
    fn dangerous_disable_auxiliary_device_interface(&mut self) {
        send_controller_command_and_wait_processing(
            self,
            Command::DISABLE_AUXILIARY_DEVICE_INTERFACE,
        );
    }

    fn dangerous_enable_auxiliary_device(&mut self) {
        send_controller_command_and_wait_processing(
            self,
            Command::ENABLE_AUXILIARY_DEVICE_INTERFACE,
        );
    }

    fn dangerous_disable_keyboard_interface(&mut self) {
        send_controller_command_and_wait_processing(self, Command::DISABLE_KEYBOARD_INTERFACE);
    }

    fn dangerous_enable_keyboard_interface(&mut self) {
        send_controller_command_and_wait_processing(self, Command::ENABLE_KEYBOARD_INTERFACE);
    }
}

pub trait Testing<T: PortIO>:
    ReadStatus<T> + ReadRAM<T> + InterruptsDisabled + KeyboardDisabled + AuxiliaryDeviceDisabled + Sized
{
    fn auxiliary_device_interface_test(&mut self) -> Result<(), DeviceInterfaceError> {
        let test_result = send_controller_command_and_wait_response(
            self,
            CommandReturnData::AUXILIARY_DEVICE_INTERFACE_TEST,
        );
        DeviceInterfaceError::from_test_result(test_result)
    }

    fn self_test(&mut self) -> Result<(), u8> {
        // According to the OSDev Wiki the controller self test
        // may reset the controller, so lets save
        // the controller command byte and restore it
        // after the self test.

        let command_byte = self.controller_command_byte();
        let result = send_controller_command_and_wait_response(self, CommandReturnData::SELF_TEST);
        write_controller_command_byte(self, command_byte);

        if result == 0x55 {
            Ok(())
        } else {
            Err(result)
        }
    }

    fn keyboard_interface_test(&mut self) -> Result<(), DeviceInterfaceError> {
        let test_result = send_controller_command_and_wait_response(
            self,
            CommandReturnData::KEYBOARD_INTERFACE_TEST,
        );
        DeviceInterfaceError::from_test_result(test_result)
    }
}

#[derive(Debug)]
pub enum DeviceData {
    Keyboard(u8),
    AuxiliaryDevice(u8),
}

pub trait ReadData<T: PortIO>: ReadStatus<T> + Sized {
    fn read_data(&mut self) -> Option<DeviceData> {
        self.status().data_availability().map(|data_owner| {
            let data = self.port_io_mut().read(T::DATA_PORT);
            match data_owner {
                DataOwner::KeyboardOrCommandController => DeviceData::Keyboard(data),
                DataOwner::AuxiliaryDevice => DeviceData::AuxiliaryDevice(data),
            }
        })
    }
}

pub trait ResetCPU<T: PortIO>: ReadStatus<T> + Sized {
    fn reset_cpu(&mut self) {
        send_controller_command_and_wait_processing(
            self,
            Command::PULSE_OUTPUT_PORT_START | 0b0000_1110,
        );
    }
}
