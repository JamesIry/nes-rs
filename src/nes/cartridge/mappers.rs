use crate::bus::InterruptFlags;

use super::MirrorType;

pub mod mapper0;
pub trait Mapper {
    fn translate_cpu_addr(&mut self, addr: u16) -> CartridgeCpuLocation;
    fn translate_ppu_addr(&mut self, addr: u16) -> CartridgePpuLocation;
    fn mirror_type(&self) -> MirrorType;
    fn configure(&mut self, addr: u16, value: u8) -> u8;
    fn cpu_bus_clock(&mut self) -> InterruptFlags;
    fn ppu_bus_clock(&mut self);
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

pub struct NulMapper {}

impl Mapper for NulMapper {
    fn translate_cpu_addr(&mut self, _addr: u16) -> CartridgeCpuLocation {
        CartridgeCpuLocation::None
    }

    fn translate_ppu_addr(&mut self, _addr: u16) -> CartridgePpuLocation {
        CartridgePpuLocation::None
    }

    fn mirror_type(&self) -> MirrorType {
        MirrorType::FourScreen
    }

    fn configure(&mut self, _addr: u16, _value: u8) -> u8 {
        0
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }

    fn ppu_bus_clock(&mut self) {}
}
