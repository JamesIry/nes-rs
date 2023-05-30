pub mod mapper0;
pub trait Mapper {
    fn translate_cpu_addr(&mut self, addr: u16) -> CartridgeCpuLocation;
    fn translate_ppu_addr(&mut self, addr: u16) -> CartridgePpuLocation;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CartridgeCpuLocation {
    None,
    SRam(u16),
    Trainer(u16),
    PrgRom(u16),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CartridgePpuLocation {
    None,
    ChrRom(u16),
    VRam(u16),
}
