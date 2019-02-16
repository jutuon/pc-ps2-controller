
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
    fn port_io_mut(&mut self) -> &mut T;
}


macro_rules! impl_port_io_available {
    (<T: PortIO> $type:ty) => {
        impl <T: PortIO> crate::controller::io::PortIOAvailable<T> for $type {
            fn port_io_mut(&mut self) -> &mut T {
                &mut self.0
            }
        }
    };
    (<T: PortIO, D1, D2, IRQ> $type:ty) => {
        impl <T: PortIO, D1, D2, IRQ> crate::controller::io::PortIOAvailable<T> for $type {
            fn port_io_mut(&mut self) -> &mut T {
                &mut self.0
            }
        }
    };
}
