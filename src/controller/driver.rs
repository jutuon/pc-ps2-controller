

use super::{
    raw::*,
    io::*,
};

use core::marker::PhantomData;

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

pub struct InitController<T: PortIO>(T);

impl_port_io_available!(<T: PortIO> InitController<T>);

impl <T: PortIO> InitController<T> {
    /// You should disable interrupts before starting the initialization
    /// process.
    pub fn start_init(port_io: T) -> DevicesDisabled<T> {
        let mut controller = InitController(port_io);

        controller.dangerous_disable_auxiliary_device_interface();
        controller.dangerous_disable_keyboard_interface();

        let raw_command_byte = send_controller_command_and_wait_response(&mut controller, CommandReturnData::READ_CONTROLLER_COMMAND_BYTE);

        let mut command_byte = ControllerCommandByte::from_bits_truncate(raw_command_byte);
        command_byte.set(ControllerCommandByte::ENABLE_AUXILIARY_INTERRUPT, false);
        command_byte.set(ControllerCommandByte::ENABLE_KEYBOARD_INTERRUPT, false);

        write_controller_command_byte(&mut controller, command_byte);

        DevicesDisabled(controller.0)
    }
}

impl <T: PortIO> ReadStatus<T> for InitController<T> {}
impl <T: PortIO> DangerousDeviceCommands<T> for InitController<T> {}
impl <T: PortIO> KeyboardDisabled for InitController<T> {}
impl <T: PortIO> AuxiliaryDeviceDisabled for InitController<T> {}
impl <T: PortIO> InterruptsDisabled for InitController<T> {}


#[derive(Debug)]
pub enum InterfaceError {
    Keyboard(DeviceInterfaceError),
    AuxiliaryDevice(DeviceInterfaceError),
}

pub struct DevicesDisabled<T: PortIO>(T);

impl <T: PortIO> DevicesDisabled<T> {
    pub fn scancode_translation(&mut self, enabled: bool) {
        let mut command_byte = self.controller_command_byte();
        command_byte.set(ControllerCommandByte::KEYBOARD_TRANSLATE_MODE, enabled);
        write_controller_command_byte(self, command_byte);
    }

    pub fn enable_devices(mut self, devices: EnableDevice) -> Result<EnabledDevices<T, Disabled>, (Self, InterfaceError)> {
        match self.test_devices(devices) {
            Ok(()) => Ok(self.configure(devices, false)),
            Err(e) => Err((self, e)),
        }
    }

    pub fn enable_devices_and_interrupts(mut self, devices: EnableDevice) -> Result<EnabledDevices<T, InterruptsEnabled>, (Self, InterfaceError)> {
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
        self.auxiliary_device_interface_test().map_err(|e| InterfaceError::AuxiliaryDevice(e))
    }

    fn test_keyboard(&mut self) -> Result<(), InterfaceError> {
        self.keyboard_interface_test().map_err(|e| InterfaceError::Keyboard(e))
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
            },
        }

        if interrupts {
            let mut command_byte = self.controller_command_byte();

            match &devices {
                EnableDevice::Keyboard => command_byte.set(ControllerCommandByte::ENABLE_KEYBOARD_INTERRUPT, true),
                EnableDevice::AuxiliaryDevice => command_byte.set(ControllerCommandByte::ENABLE_AUXILIARY_INTERRUPT, true),
                EnableDevice::KeyboardAndAuxiliaryDevice => {
                    command_byte.set(ControllerCommandByte::ENABLE_KEYBOARD_INTERRUPT, true);
                    command_byte.set(ControllerCommandByte::ENABLE_AUXILIARY_INTERRUPT, true);
                },
            }

            write_controller_command_byte(&mut self, command_byte);
        }

        EnabledDevices {
            port_io: self.0,
            _marker: PhantomData,
            devices
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

impl <T: PortIO> ReadStatus<T> for DevicesDisabled<T> {}
impl <T: PortIO> DangerousDeviceCommands<T> for DevicesDisabled<T> {}
impl <T: PortIO> InterruptsDisabled for DevicesDisabled<T> {}
impl <T: PortIO> KeyboardDisabled for DevicesDisabled<T> {}
impl <T: PortIO> AuxiliaryDeviceDisabled for DevicesDisabled<T> {}
impl <T: PortIO> ReadRAM<T> for DevicesDisabled<T> {}
impl <T: PortIO> WriteRAM<T> for DevicesDisabled<T> {}
impl <T: PortIO> Testing<T> for DevicesDisabled<T> {}

pub struct EnabledDevices<T: PortIO, IRQ> {
    port_io: T,
    _marker: PhantomData<IRQ>,
    devices: EnableDevice,
}

impl <T: PortIO, IRQ> EnabledDevices<T, IRQ> {
    pub fn send_to_auxiliary_device(&mut self, data: u8) -> Result<(),()> {
        match &self.devices {
            EnableDevice::AuxiliaryDevice | EnableDevice::KeyboardAndAuxiliaryDevice => {
                send_controller_command_and_write_data(self, CommandWaitData::WRITE_TO_AUXILIARY_DEVICE, data);
                Ok(())
            },
            EnableDevice::Keyboard => Err(())
        }
    }

    pub fn send_to_keyboard(&mut self, data: u8) -> Result<(), ()> {
        match &self.devices {
            EnableDevice::Keyboard | EnableDevice::KeyboardAndAuxiliaryDevice => {
                self.port_io_mut().write(T::DATA_PORT, data);
                Ok(())
            },
            EnableDevice::AuxiliaryDevice => Err(())
        }
    }
}

impl <T: PortIO> EnabledDevices<T, InterruptsEnabled> {
    /// You should disable the interrupts before disabling
    /// the devices.
    pub fn disable_devices(self) -> DevicesDisabled<T> {
        InitController::start_init(self.port_io)
    }
}

impl <T: PortIO> EnabledDevices<T, Disabled> {
    pub fn disable_devices(mut self) -> DevicesDisabled<T> {
        self.dangerous_disable_auxiliary_device_interface();
        self.dangerous_disable_keyboard_interface();

        DevicesDisabled(self.port_io)
    }
}

impl_port_io_available!(<T: PortIO, IRQ> EnabledDevices<T, IRQ>);

impl <T: PortIO, IRQ> ReadStatus<T> for EnabledDevices<T, IRQ> {}
impl <T: PortIO, IRQ> ReadData<T> for EnabledDevices<T, IRQ> {}

impl <T: PortIO> DangerousDeviceCommands<T> for EnabledDevices<T, Disabled> {}

pub struct InterruptsEnabled;
pub struct KeyboardEnabled;
pub struct AuxiliaryDeviceEnabled;
pub struct Disabled;

#[derive(Debug)]
pub enum DeviceInterfaceError {
    ClockLineLow,
    ClockLineHigh,
    DataLineLow,
    DataLineHigh,
    UnknownValue(u8)
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

fn send_controller_command<T: PortIO, U: ReadStatus<T>>(controller: &mut U, command: u8) {
    while controller.status().input_buffer_full() {}
    controller.port_io_mut().write(T::COMMAND_REGISTER, command);
}

fn send_controller_command_and_write_data<T: PortIO, U: ReadStatus<T>>(controller: &mut U, command: u8, data: u8) {
    send_controller_command(controller, command);
    while controller.status().input_buffer_full() {}
    controller.port_io_mut().write(T::DATA_PORT, data);
}

fn write_controller_command_byte<T: PortIO, U: ReadStatus<T>>(controller: &mut U, data: ControllerCommandByte) {
    send_controller_command_and_write_data(controller, CommandWaitData::WRITE_CONTROLLER_COMMAND_BYTE, data.bits())
}

fn send_controller_command_and_wait_response<
    T: PortIO,
    U: ReadStatus<T> + InterruptsDisabled +
        KeyboardDisabled + AuxiliaryDeviceDisabled
    >(controller: &mut U, command: u8) -> u8 {
    if let Some(_) = controller.status().data_availability() {
        controller.port_io_mut().read(T::DATA_PORT);
    }

    send_controller_command(controller, command);

    loop {
        if let Some(DataOwner::KeyboardOrCommandController) = controller.status().data_availability() {
            return controller.port_io_mut().read(T::DATA_PORT)
        }
    }
}

/// Marker trait to notify user about when interrupts
/// should be disabled or not handled.
pub trait InterruptsDisabled {}
pub trait KeyboardDisabled {}
pub trait AuxiliaryDeviceDisabled {}

pub trait ReadRAM<T: PortIO>: ReadStatus<T> + InterruptsDisabled + KeyboardDisabled + AuxiliaryDeviceDisabled + Sized {
    fn controller_command_byte(&mut self) -> ControllerCommandByte {
        let raw = send_controller_command_and_wait_response(self, CommandReturnData::READ_CONTROLLER_COMMAND_BYTE);
        ControllerCommandByte::from_bits_truncate(raw)
    }

    fn ram(&mut self, data: &mut [u8; CONTROLLER_RAM_SIZE]) {
        for (i, byte) in data.iter_mut().enumerate() {
            let data = send_controller_command_and_wait_response(self, CommandReturnData::READ_RAM_START + i as u8);
            *byte = data;
        }
    }
}

pub trait WriteRAM<T: PortIO>: ReadStatus<T> + Sized {
    fn write_ram(&mut self, data: &mut [u8; CONTROLLER_RAM_SIZE]) {
        for (i, byte) in data.iter().enumerate() {
            send_controller_command_and_write_data(self, CommandWaitData::WRITE_RAM_START + i as u8, *byte);
        }
    }
}

/// Commands which may break invariants which are encoded
/// to the types.
trait DangerousDeviceCommands<T: PortIO>: ReadStatus<T> + Sized {
    fn dangerous_disable_auxiliary_device_interface(&mut self) {
        send_controller_command(self, Command::DISABLE_AUXILIARY_DEVICE_INTERFACE);
        while self.status().input_buffer_full() {}
    }

    fn dangerous_enable_auxiliary_device(&mut self) {
        send_controller_command(self, Command::ENABLE_AUXILIARY_DEVICE_INTERFACE);
        while self.status().input_buffer_full() {}
    }

    fn dangerous_disable_keyboard_interface(&mut self) {
        send_controller_command(self, Command::DISABLE_KEYBOARD_INTERFACE);
        while self.status().input_buffer_full() {}
    }

    fn dangerous_enable_keyboard_interface(&mut self) {
        send_controller_command(self, Command::ENABLE_KEYBOARD_INTERFACE);
        while self.status().input_buffer_full() {}
    }
}

pub trait Testing<T: PortIO>: ReadStatus<T> + InterruptsDisabled + KeyboardDisabled + AuxiliaryDeviceDisabled + Sized {
    fn auxiliary_device_interface_test(&mut self) -> Result<(), DeviceInterfaceError> {
        let test_result = send_controller_command_and_wait_response(self, CommandReturnData::AUXILIARY_DEVICE_INTERFACE_TEST);
        DeviceInterfaceError::from_test_result(test_result)
    }

    fn self_test(&mut self) -> Result<(), u8> {
        let result = send_controller_command_and_wait_response(self, CommandReturnData::SELF_TEST);
        if result == 0x55 {
            Ok(())
        } else {
            Err(result)
        }
    }

    fn keyboard_interface_test(&mut self) -> Result<(), DeviceInterfaceError> {
        let test_result = send_controller_command_and_wait_response(self, CommandReturnData::KEYBOARD_INTERFACE_TEST);
        DeviceInterfaceError::from_test_result(test_result)
    }
}


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
