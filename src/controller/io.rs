pub trait PortIO {
    const DATA_PORT: u16 = 0x60;
    const STATUS_REGISTER: u16 = 0x64;
    const COMMAND_REGISTER: u16 = 0x64;

    // Reading is `&mut self`, because it can change controller state.
    fn read(&mut self, port: u16) -> u8;
    fn write(&mut self, port: u16, data: u8);
}

pub trait PortIOAvailable<T: PortIO> {
    fn port_io_mut(&mut self) -> &mut PortIOWrapper<T>;
}

pub struct PortIOWrapper<T: PortIO>(pub(crate) T);

pub(crate) trait PrivatePortIO {
    // Reading is `&mut self`, because it can change controller state.
    fn read(&mut self, port: u16) -> u8;
    fn write(&mut self, port: u16, data: u8);
}

impl <T: PortIO> PrivatePortIO for PortIOWrapper<T> {
    fn read(&mut self, port: u16) -> u8 {
        self.0.read(port)
    }
    fn write(&mut self, port: u16, data: u8) {
        self.0.write(port, data)
    }
}
