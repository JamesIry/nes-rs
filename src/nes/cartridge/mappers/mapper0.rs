use crate::{
    bus::InterruptFlags,
    nes::cartridge::{
        mappers::{CartridgeCpuLocation, CartridgePpuLocation},
        Mapper, MirrorType,
    },
};

#[cfg(test)]
mod unit_tests;

pub struct Mapper0 {
    mirror_type: MirrorType,
}

impl Mapper0 {
    pub fn new(mirroring: MirrorType) -> Self {
        Self {
            mirror_type: mirroring,
        }
    }
}

impl Mapper for Mapper0 {
    fn translate_cpu_addr(&mut self, addr: u16) -> CartridgeCpuLocation {
        if (0x4000..=0x7FFF).contains(&addr) {
            CartridgeCpuLocation::SRam(addr)
        } else if addr >= 0x8000 {
            CartridgeCpuLocation::PrgRom(addr)
        } else {
            CartridgeCpuLocation::None
        }
    }

    fn translate_ppu_addr(&mut self, addr: u16) -> CartridgePpuLocation {
        if (0x0000..=0x1FFF).contains(&addr) {
            CartridgePpuLocation::ChrRom(addr)
        } else if (0x2000..=0x3EFF).contains(&addr) {
            CartridgePpuLocation::VRam(addr)
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
