use super::{CartridgeCpuLocation, Mapper};

pub struct Mapper0 {}

impl Mapper0 {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mapper for Mapper0 {
    fn translate_cpu_addr(&mut self, addr: u16) -> CartridgeCpuLocation {
        if (0x6000..=0x7FFF).contains(&addr) {
            CartridgeCpuLocation::Ram(addr - 0x6000)
        } else if addr >= 0x8000 {
            CartridgeCpuLocation::PrgRom(addr - 0x8000)
        } else {
            CartridgeCpuLocation::None
        }
    }
}
