use crate::{
    bus::InterruptFlags,
    nes::cartridge::{
        mappers::{CartridgeCpuLocation, CartridgePpuLocation},
        Mapper, MirrorType, NesHeader,
    },
};

use super::AddressConverter;

/**
 * Mapper 7
 */
pub struct AxRom {
    mirror_mode: u8,
    prg_rom_converter: AddressConverter,
}

impl AxRom {
    pub fn new(_nes_header: &NesHeader) -> Self {
        Self {
            mirror_mode: 0,
            prg_rom_converter: AddressConverter::new(0x8000, 32, 32, None),
        }
    }
}

impl Mapper for AxRom {
    fn translate_cpu_addr(&mut self, addr: usize) -> CartridgeCpuLocation {
        if (0x4000..=0x7FFF).contains(&addr) {
            CartridgeCpuLocation::SRam(addr - 0x4000)
        } else if addr >= 0x8000 {
            CartridgeCpuLocation::PrgRom(self.prg_rom_converter.convert(addr))
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
        MirrorType::SingleScreen(self.mirror_mode)
    }

    fn configure(&mut self, _addr: u16, value: u8) -> u8 {
        let old = self.prg_rom_converter.bank | (self.mirror_mode << 4);
        self.prg_rom_converter.bank = value & 0b00000111;
        self.mirror_mode = (value & 0b00010000) >> 4;

        old
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }

    fn ppu_bus_clock(&mut self) {}
}
