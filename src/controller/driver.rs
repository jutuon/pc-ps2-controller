

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

pub struct InitController<T: PortIO>(PortIOWrapper<T>);

impl <T: PortIO> PortIOAvailable<T> for InitController<T> {
    fn port_io_mut(&mut self) -> &mut PortIOWrapper<T> { &mut self.0 }
}

impl <T: PortIO> InitController<T> {
    pub fn start_init(port_io: T) -> InitControllerWaitInterrupt<T> {
        let mut controller = InitController(PortIOWrapper(port_io));

        controller.dangerous_disable_auxiliary_device_interface();
        controller.dangerous_disable_keyboard_interface();

        let wait_interrupt = WaitInterrupt::send_controller_command(
            controller,
            RawCommands::READ_CONTROLLER_COMMAND_BYTE,
            Self::convert_function
        );

        InitControllerWaitInterrupt(wait_interrupt)
    }

    fn convert_function(mut controller: InitController<T>, raw_command_byte: u8) -> DevicesDisabled<T> {
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


#[derive(Debug)]
pub enum InterfaceError {
    Keyboard(DeviceInterfaceError),
    AuxiliaryDevice(DeviceInterfaceError),
}

pub struct DevicesDisabled<T: PortIO>(PortIOWrapper<T>);

impl <T: PortIO> DevicesDisabled<T> {
    pub fn scancode_translation(&mut self, enabled: bool) {
        let mut command_byte = self.controller_command_byte();
        command_byte.set(ControllerCommandByte::KEYBOARD_TRANSLATE_MODE, enabled);
        write_controller_command_byte(self, command_byte);
    }

    pub fn enable_keyboard(mut self) -> Result<EnabledDevices<T, KeyboardEnabled, Disabled, Disabled>, (Self, DeviceInterfaceError)> {
        match self.keyboard_interface_test() {
            Ok(()) => Ok(self.configure(EnableDevice::Keyboard, false)),
            Err(e) => Err((self, e)),
        }
    }

    pub fn enable_auxiliary_device(mut self) -> Result<EnabledDevices<T, Disabled, AuxiliaryDeviceEnabled, Disabled>, (Self, DeviceInterfaceError)> {
        match self.auxiliary_device_interface_test() {
            Ok(()) => Ok(self.configure(EnableDevice::AuxiliaryDevice, false)),
            Err(e) => Err((self, e)),
        }
    }

    pub fn enable_keyboard_and_auxiliary_device(mut self) -> Result<EnabledDevices<T, KeyboardEnabled, AuxiliaryDeviceEnabled, Disabled>, (Self, InterfaceError)> {
        match self.test_keyboard_and_auxiliary_device() {
            Ok(()) => Ok(self.configure(EnableDevice::KeyboardAndAuxiliaryDevice, false)),
            Err(e) => Err((self, e)),
        }
    }

    pub fn enable_keyboard_and_interrupts(mut self) -> Result<EnabledDevices<T, KeyboardEnabled, Disabled, InterruptsEnabled>, (Self, DeviceInterfaceError)> {
        match self.keyboard_interface_test() {
            Ok(()) => Ok(self.configure(EnableDevice::Keyboard, true)),
            Err(e) => Err((self, e)),
        }
    }

    pub fn enable_auxiliary_device_and_interrupts(mut self) -> Result<EnabledDevices<T, Disabled, AuxiliaryDeviceEnabled, InterruptsEnabled>, (Self, DeviceInterfaceError)> {
        match self.auxiliary_device_interface_test() {
            Ok(()) => Ok(self.configure(EnableDevice::AuxiliaryDevice, true)),
            Err(e) => Err((self, e)),
        }
    }

    pub fn enable_keyboard_and_auxiliary_device_and_interrupts(mut self) -> Result<EnabledDevices<T, KeyboardEnabled, AuxiliaryDeviceEnabled, InterruptsEnabled>, (Self, InterfaceError)> {
        match self.test_keyboard_and_auxiliary_device() {
            Ok(()) => Ok(self.configure(EnableDevice::KeyboardAndAuxiliaryDevice, true)),
            Err(e) => Err((self, e)),
        }
    }

    fn test_keyboard_and_auxiliary_device(&mut self) -> Result<(), InterfaceError> {
        self.keyboard_interface_test()
            .map_err(|e| InterfaceError::Keyboard(e))
            .and(self.auxiliary_device_interface_test().map_err(|e| InterfaceError::AuxiliaryDevice(e)))
    }

    fn configure<D1, D2, IRQ>(mut self, devices: EnableDevice, interrupts: bool) -> EnabledDevices<T, D1, D2, IRQ> {
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

        EnabledDevices(self.0, PhantomData, PhantomData, PhantomData)
    }
}

enum EnableDevice {
    Keyboard,
    AuxiliaryDevice,
    KeyboardAndAuxiliaryDevice,
}

impl <T: PortIO> PortIOAvailable<T> for DevicesDisabled<T> {
    fn port_io_mut(&mut self) -> &mut PortIOWrapper<T> { &mut self.0 }
}

impl <T: PortIO> ReadStatus<T> for DevicesDisabled<T> {}
impl <T: PortIO> DangerousDeviceCommands<T> for DevicesDisabled<T> {}
impl <T: PortIO> InterruptsDisabled for DevicesDisabled<T> {}
impl <T: PortIO> KeyboardDisabled for DevicesDisabled<T> {}
impl <T: PortIO> AuxiliaryDeviceDisabled for DevicesDisabled<T> {}
impl <T: PortIO> ReadRAM<T> for DevicesDisabled<T> {}
impl <T: PortIO> WriteRAM<T> for DevicesDisabled<T> {}
impl <T: PortIO> Testing<T> for DevicesDisabled<T> {}

pub struct EnabledDevices<T: PortIO, D1, D2, IRQ>(PortIOWrapper<T>, PhantomData<D1>, PhantomData<D2>, PhantomData<IRQ>);

impl <T: PortIO, D1, D2> EnabledDevices<T, D1, D2, InterruptsEnabled> {
    pub fn disable_devices(self) -> InitControllerWaitInterrupt<T> {
        let port_io_wrapper = self.0;
        InitController::start_init(port_io_wrapper.0)
    }
}

impl <T: PortIO, D1, D2> EnabledDevices<T, D1, D2, Disabled> {
    pub fn disable_devices(mut self) -> DevicesDisabled<T> {
        self.dangerous_disable_auxiliary_device_interface();
        self.dangerous_disable_keyboard_interface();

        DevicesDisabled(self.0)
    }
}

impl <T: PortIO, D1, D2, IRQ> PortIOAvailable<T> for EnabledDevices<T, D1, D2, IRQ> {
    fn port_io_mut(&mut self) -> &mut PortIOWrapper<T> { &mut self.0 }
}

impl <T: PortIO, D1, D2, IRQ> ReadStatus<T> for EnabledDevices<T, D1, D2, IRQ> {}

impl <T: PortIO, D2> KeyboardInterruptIO<T> for EnabledDevices<T, KeyboardEnabled, D2, InterruptsEnabled> {}
impl <T: PortIO, D1> AuxiliaryDeviceInterruptIO<T> for EnabledDevices<T, D1, AuxiliaryDeviceEnabled, InterruptsEnabled> {}
impl <T: PortIO, D2, IRQ> KeyboardIO<T> for EnabledDevices<T, KeyboardEnabled, D2, IRQ> {}
impl <T: PortIO, D1, IRQ> AuxiliaryDeviceIO<T> for EnabledDevices<T, D1, AuxiliaryDeviceEnabled, IRQ> {}

impl <T: PortIO, D1, D2> DangerousDeviceCommands<T> for EnabledDevices<T, D1, D2, Disabled> {}

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
    send_controller_command_and_write_data(controller, RawCommands::WRITE_CONTROLLER_COMMAND_BYTE, data.bits())
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


pub struct InitControllerWaitInterrupt<T: PortIO>(WaitInterrupt<T, InitController<T>, DevicesDisabled<T>>);

impl <T: PortIO> InitControllerWaitInterrupt<T> {
    pub fn interrupt_received(self) -> DevicesDisabled<T> {
        self.0.interrupt_received()
    }

    pub fn poll_data(self) -> DevicesDisabled<T> {
        self.0.poll_keyboard_or_controller_command_data()
    }
}

struct WaitInterrupt<
    T: PortIO,
    U: ReadStatus<T> + KeyboardDisabled + AuxiliaryDeviceDisabled,
    V: ReadStatus<T> + KeyboardDisabled + AuxiliaryDeviceDisabled,
>(U, fn(U, u8) -> V, PhantomData<T>);

impl <
    T: PortIO,
    U: ReadStatus<T> + KeyboardDisabled + AuxiliaryDeviceDisabled,
    V: ReadStatus<T> + KeyboardDisabled + AuxiliaryDeviceDisabled
> WaitInterrupt<T, U, V> {
    fn send_controller_command(mut controller: U, command: u8, converter: fn(U, u8) -> V) -> Self {
        if let Some(_) = controller.status().data_availability() {
            controller.port_io_mut().read(T::DATA_PORT);
        }

        send_controller_command(&mut controller, command);

        WaitInterrupt(controller, converter, PhantomData)
    }

    fn interrupt_received(mut self) -> V {
        let data = self.0.port_io_mut().read(T::DATA_PORT);
        (self.1)(self.0, data)
    }

    fn poll_keyboard_or_controller_command_data(mut self) -> V {
        let data = loop {
            if let Some(DataOwner::KeyboardOrCommandController) = self.0.status().data_availability() {
                break self.0.port_io_mut().read(T::DATA_PORT);
            }
        };
        (self.1)(self.0, data)
    }
}


pub trait InterruptsDisabled {}
pub trait KeyboardDisabled {}
pub trait AuxiliaryDeviceDisabled {}

pub trait ReadRAM<T: PortIO>: ReadStatus<T> + InterruptsDisabled + KeyboardDisabled + AuxiliaryDeviceDisabled + Sized {
    fn controller_command_byte(&mut self) -> ControllerCommandByte {
        let raw = send_controller_command_and_wait_response(self, RawCommands::READ_CONTROLLER_COMMAND_BYTE);
        ControllerCommandByte::from_bits_truncate(raw)
    }

    fn ram(&mut self, data: &mut [u8; CONTROLLER_RAM_SIZE]) {
        for (i, byte) in data.iter_mut().enumerate() {
            let data = send_controller_command_and_wait_response(self, RawCommands::READ_RAM_START + i as u8);
            *byte = data;
        }
    }
}

pub trait WriteRAM<T: PortIO>: ReadStatus<T> + Sized {
    fn write_ram(&mut self, data: &mut [u8; CONTROLLER_RAM_SIZE]) {
        for (i, byte) in data.iter().enumerate() {
            send_controller_command_and_write_data(self, RawCommands::WRITE_RAM_START + i as u8, *byte);
        }
    }
}

/// Commands which may break invariants which are encoded
/// to the types.
trait DangerousDeviceCommands<T: PortIO>: ReadStatus<T> + Sized {
    fn dangerous_disable_auxiliary_device_interface(&mut self) {
        send_controller_command(self, RawCommands::DISABLE_AUXILIARY_DEVICE_INTERFACE);
        while self.status().input_buffer_full() {}
    }

    fn dangerous_enable_auxiliary_device(&mut self) {
        send_controller_command(self, RawCommands::ENABLE_AUXILIARY_DEVICE_INTERFACE);
        while self.status().input_buffer_full() {}
    }

    fn dangerous_disable_keyboard_interface(&mut self) {
        send_controller_command(self, RawCommands::DISABLE_KEYBOARD_INTERFACE);
        while self.status().input_buffer_full() {}
    }

    fn dangerous_enable_keyboard_interface(&mut self) {
        send_controller_command(self, RawCommands::ENABLE_KEYBOARD_INTERFACE);
        while self.status().input_buffer_full() {}
    }
}

pub trait Testing<T: PortIO>: ReadStatus<T> + InterruptsDisabled + KeyboardDisabled + AuxiliaryDeviceDisabled + Sized {
    fn auxiliary_device_interface_test(&mut self) -> Result<(), DeviceInterfaceError> {
        let test_result = send_controller_command_and_wait_response(self, RawCommands::AUXILIARY_DEVICE_INTERFACE_TEST);
        DeviceInterfaceError::from_test_result(test_result)
    }

    fn self_test(&mut self) -> Result<(), u8> {
        let result = send_controller_command_and_wait_response(self, RawCommands::SELF_TEST);
        if result == 0x55 {
            Ok(())
        } else {
            Err(result)
        }
    }

    fn keyboard_interface_test(&mut self) -> Result<(), DeviceInterfaceError> {
        let test_result = send_controller_command_and_wait_response(self, RawCommands::KEYBOARD_INTERFACE_TEST);
        DeviceInterfaceError::from_test_result(test_result)
    }
}

pub trait AuxiliaryDeviceIO<T: PortIO>: ReadStatus<T> + Sized {
    fn send_to_auxiliary_device(&mut self, data: u8) {
        send_controller_command_and_write_data(self, RawCommands::WRITE_TO_AUXILIARY_DEVICE, data);
    }

    fn poll_auxiliary_device_data(&mut self) -> Option<u8> {
        if let Some(DataOwner::AuxiliaryDevice) = self.status().data_availability() {
            Some(self.port_io_mut().read(T::DATA_PORT))
        } else {
            None
        }
    }
}

pub trait KeyboardIO<T: PortIO>: ReadStatus<T> + Sized {
    fn send_to_keyboard(&mut self, data: u8) {
        self.port_io_mut().write(T::DATA_PORT, data);
    }

    fn poll_keyboard_data(&mut self) -> Option<u8> {
        if let Some(DataOwner::KeyboardOrCommandController) = self.status().data_availability() {
            Some(self.port_io_mut().read(T::DATA_PORT))
        } else {
            None
        }
    }
}

pub trait KeyboardInterruptIO<T: PortIO>: ReadStatus<T> + Sized {
    fn keyboard_interrupt_read_data_port(&mut self) -> u8 {
        self.port_io_mut().read(T::DATA_PORT)
    }
}

pub trait AuxiliaryDeviceInterruptIO<T: PortIO>: ReadStatus<T> + Sized {
    fn auxiliary_device_interrupt_read_data_port(&mut self) -> u8 {
        self.port_io_mut().read(T::DATA_PORT)
    }
}
