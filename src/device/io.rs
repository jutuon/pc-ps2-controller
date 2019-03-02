pub trait SendToDevice {
    fn send(&mut self, data: u8);
}
