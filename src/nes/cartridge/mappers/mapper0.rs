use crate::nes::cartridge::{
    mappers::{CartridgeCpuLocation, CartridgePpuLocation},
    Mapper, MirrorType,
};

#[cfg(test)]
mod unit_tests;

pub struct Mapper0 {
    mirroring: MirrorType,
}

impl Mapper0 {
    pub fn new(mirroring: MirrorType) -> Self {
        Self { mirroring }
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
        } else if (0x2000..=0x3FFF).contains(&addr) {
            let raw_index = addr & 0b000111111111111; // mirror 0x2000-0x3FFF down to 0x0000 - 0x1FFF by turning off some bits
            let name_table = raw_index / 0x400;
            debug_assert!((0..=3).contains(&name_table));

            let physical = match (self.mirroring, name_table) {
                (MirrorType::Vertical, 2)
                | (MirrorType::Vertical, 3)
                | (MirrorType::Horizontal, 3) => raw_index - 0x0800,
                (MirrorType::Horizontal, 2) | (MirrorType::Horizontal, 1) => raw_index - 0x0400,
                (MirrorType::Horizontal, 0)
                | (MirrorType::Vertical, 0)
                | (MirrorType::Vertical, 1) => raw_index,
                (MirrorType::FourScreen, _) => raw_index,
                _ => raw_index,
            };

            CartridgePpuLocation::VRam(physical)
        } else {
            CartridgePpuLocation::None
        }
    }
}
