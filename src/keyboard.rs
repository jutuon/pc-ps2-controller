pub use pc_keyboard;

use pc_keyboard::{Keyboard, DecodedKey, KeyboardLayout, ScancodeSet};

use crate::controller::{
    io::PortIO,
    driver::{
        KeyboardIO,
        ReadData,
        DeviceData,
    },
};

use core::marker::PhantomData;

pub struct KeyboardDriver<T: PortIO, U: KeyboardIO<T>, K: KeyboardLayout, S: ScancodeSet>(U, Keyboard<K, S>, PhantomData<T>);

impl <T: PortIO, U: KeyboardIO<T> + ReadData<T>, K: KeyboardLayout, S: ScancodeSet> KeyboardDriver<T, U, K, S> {
    pub fn new(controller: U, keyboard: Keyboard<K, S>) -> Self {
        KeyboardDriver(controller, keyboard, PhantomData)
    }

    pub fn poll_keyboard(&mut self) -> Option<DecodedKey> {
        if let DeviceData::Keyboard(data) = self.0.read_data()? {
            self.handle_keyboard_data(data)
        } else {
            None
        }
    }

    pub fn exit(self) -> U {
        self.0
    }

    fn handle_keyboard_data(&mut self, data: u8) -> Option<DecodedKey> {
        self.1.add_byte(data)
            .ok()
            .unwrap_or_default()
            .map(|event| self.1.process_keyevent(event))
            .unwrap_or_default()
    }
}

impl <T: PortIO, U: KeyboardIO<T> + ReadData<T>, K: KeyboardLayout, S: ScancodeSet> KeyboardDriver<T, U, K, S> {
    pub fn handle_keyboard_interrupt(&mut self) -> Option<DecodedKey> {
        if let DeviceData::Keyboard(data) = self.0.read_data()? {
            self.handle_keyboard_data(data)
        } else {
            None
        }
    }
}
