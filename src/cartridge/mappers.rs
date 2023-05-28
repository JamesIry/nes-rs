pub mod mapper0;
pub trait Mapper {
    fn translate_cpu_addr(&mut self, addr: u16) -> CartridgeCpuLocation;
}
pub enum CartridgeCpuLocation {
    None,
    Ram(u16),
    Trainer(u16),
    PrgRom(u16),
}
