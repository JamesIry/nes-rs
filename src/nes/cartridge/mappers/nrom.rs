use crate::{
    bus::InterruptFlags,
    nes::cartridge::{
        mappers::{CartridgeCpuLocation, CartridgePpuLocation},
        Mapper, MirrorType, NesHeader,
    },
};

#[cfg(test)]
mod unit_tests;

/**
 * Mapper 0
 */
pub struct NRom {
    mirror_type: MirrorType,
}

impl NRom {
    pub fn new(nes_header: &NesHeader) -> Self {
        Self {
            mirror_type: nes_header.mirror_type,
        }
    }
}

impl Mapper for NRom {
    fn translate_cpu_addr(&mut self, addr: usize) -> CartridgeCpuLocation {
        if (0x4000..=0x7FFF).contains(&addr) {
            CartridgeCpuLocation::SRam(addr - 0x4000)
        } else if addr >= 0x8000 {
            CartridgeCpuLocation::PrgRom(addr - 0x8000)
        } else {
            CartridgeCpuLocation::None
        }
    }

    fn translate_ppu_addr(&mut self, addr: usize) -> CartridgePpuLocation {
        if (0x0000..=0x1FFF).contains(&addr) {
            CartridgePpuLocation::ChrRom(addr)
        } else if (0x2000..=0x3EFF).contains(&addr) {
            CartridgePpuLocation::VRam(addr - 0x2000)
        } else {
            CartridgePpuLocation::None
        }
    }

    fn mirror_type(&self) -> MirrorType {
        self.mirror_type
    }

    fn configure(&mut self, _addr: u16, _value: u8) -> u8 {
        0
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }

    fn ppu_bus_clock(&mut self) {}
}
