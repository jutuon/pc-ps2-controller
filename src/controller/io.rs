
pub const DATA_PORT_RAW: u16 = 0x60;
pub const STATUS_REGISTER_RAW: u16 = 0x64;
pub const COMMAND_REGISTER_RAW: u16 = 0x64;


pub trait PortIO {
    type PortID: Copy;

    const DATA_PORT: Self::PortID;
    const STATUS_REGISTER: Self::PortID;
    const COMMAND_REGISTER: Self::PortID;

    // Reading is `&mut self`, because it can change controller state.
    fn read(&mut self, port: Self::PortID) -> u8;
    fn write(&mut self, port: Self::PortID, data: u8);
}

pub trait PortIOAvailable<T: PortIO> {
    fn port_io_mut(&mut self) -> &mut PortIOWrapper<T>;
}

pub struct PortIOWrapper<T: PortIO>(pub(crate) T);

pub(crate) trait PrivatePortIO {
    type PortID: Copy;

    // Reading is `&mut self`, because it can change controller state.
    fn read(&mut self, port: Self::PortID) -> u8;
    fn write(&mut self, port: Self::PortID, data: u8);
}

impl <T: PortIO> PrivatePortIO for PortIOWrapper<T> {
    type PortID = T::PortID;

    fn read(&mut self, port: Self::PortID) -> u8 {
        self.0.read(port)
    }
    fn write(&mut self, port: Self::PortID, data: u8) {
        self.0.write(port, data)
    }
}
