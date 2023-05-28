pub trait BusDevice {
    fn read_from_cpu_bus(&mut self, addr: u16) -> Option<u8>;
    fn write_to_cpu_bus(&mut self, addr: u16, data: u8) -> Option<u8>;
}
