use super::{CartridgeCpuLocation, CartridgePpuLocation, Mapper};

pub struct Mapper0 {}

impl Mapper0 {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mapper for Mapper0 {
    fn translate_cpu_addr(&mut self, addr: u16) -> CartridgeCpuLocation {
        if (0x6000..=0x7FFF).contains(&addr) {
            CartridgeCpuLocation::SRam(addr - 0x6000)
        } else if addr >= 0x8000 {
            CartridgeCpuLocation::PrgRom(addr - 0x8000)
        } else {
            CartridgeCpuLocation::None
        }
    }

    fn translate_ppu_addr(&mut self, addr: u16) -> CartridgePpuLocation {
        if (0x0000..=0x1FFF).contains(&addr) {
            CartridgePpuLocation::ChrRom(addr)
        } else if (0x2000..=0x2FFF).contains(&addr) {
            CartridgePpuLocation::VRam(addr - 0x2000)
        } else if (0x3000..=0x3EFF).contains(&addr) {
            CartridgePpuLocation::VRam(addr - 0x3000)
        } else {
            CartridgePpuLocation::None
        }
    }
}
