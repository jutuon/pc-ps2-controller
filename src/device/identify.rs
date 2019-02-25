
use super::io::SendToDevice;
use super::keyboard::raw::{ CommandReturnData, FromKeyboard};


pub struct DeviceIdentifier<T: SendToDevice> {
    state: fn(&mut DeviceIdentifier<T>, new_data: u8, device: &mut T) -> Option<Device>,
    byte1: u8,
}

impl <T: SendToDevice> DeviceIdentifier<T> {
    pub fn new() -> Self {
        Self {
            state: Self::start_state,
            byte1: 0,
        }
    }

    pub fn start_identification(&mut self, device: &mut T) {
        self.state = Self::start_state;
        (self.state)(self, 0, device);
    }

    pub fn byte_received(&mut self, device: &mut T, data: u8) -> Option<Device> {
        (self.state)(self, data, device)
    }

    fn start_state(state: &mut DeviceIdentifier<T>, new_data: u8, device: &mut T) -> Option<Device> {
        device.send(CommandReturnData::DEFAULT_DISABLE);
        state.state = Self::wait_ack_1;
        None
    }

    fn wait_ack_1(state: &mut DeviceIdentifier<T>, new_data: u8, device: &mut T) -> Option<Device> {
        if new_data == FromKeyboard::ACK {
            device.send(CommandReturnData::READ_ID);
            state.state = Self::wait_ack_2;
            None
        } else {
            None
        }
    }

    fn wait_ack_2(state: &mut DeviceIdentifier<T>, new_data: u8, device: &mut T) -> Option<Device> {
        if new_data == FromKeyboard::ACK {
            state.state = Self::wait_id_byte_1;
            None
        } else {
            None
        }
    }

    fn wait_id_byte_1(state: &mut DeviceIdentifier<T>, new_data: u8, device: &mut T) -> Option<Device> {
        state.state = Self::wait_id_byte_2;
        state.byte1 = new_data;
        None
    }

    fn wait_id_byte_2(state: &mut DeviceIdentifier<T>, new_data: u8, device: &mut T) -> Option<Device> {
        state.state = Self::end;

        let device = match (state.byte1, new_data) {
            (FromKeyboard::ID_FIRST_BYTE, FromKeyboard::ID_SECOND_BYTE) => Device::Keyboard,
            (first_byte, second_byte) => Device::UnknownID { first_byte, second_byte },
        };

        Some(device)
    }

    fn end(state: &mut DeviceIdentifier<T>, new_data: u8, device: &mut T) -> Option<Device> {
        None
    }
}

pub enum Device {
    Keyboard,
    Mouse,
    UnknownID { first_byte: u8, second_byte: u8 },
}
