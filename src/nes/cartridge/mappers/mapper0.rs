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
            let raw_index = addr & 0x0FFF; // mirror 0x2000-0x3FFF down to 0x0000 - 0x0FFF by turning off some bits

            let name_table_requested = (raw_index >> 10) & 0b11;

            let name_table_selected = match (self.mirroring, name_table_requested) {
                (MirrorType::Horizontal, 0b00) => 0b00,
                (MirrorType::Horizontal, 0b01) => 0b00,
                (MirrorType::Horizontal, 0b10) => 0b01,
                (MirrorType::Horizontal, 0b11) => 0b01,
                (MirrorType::Vertical, 0b00) => 0b00,
                (MirrorType::Vertical, 0b01) => 0b01,
                (MirrorType::Vertical, 0b10) => 0b00,
                (MirrorType::Vertical, 0b11) => 0b01,
                _ => name_table_requested,
            };

            let physical = (raw_index & !0b00110000000000) | (name_table_selected << 10);

            CartridgePpuLocation::VRam(physical)
        } else {
            CartridgePpuLocation::None
        }
    }
}
