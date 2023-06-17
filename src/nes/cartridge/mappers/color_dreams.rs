use crate::{
    bus::InterruptFlags,
    nes::cartridge::{
        mappers::{CartridgeCpuLocation, CartridgePpuLocation},
        Mapper, MirrorType, NesHeader,
    },
};

use super::AddressConverter;

/**
 * Mapper 11
 */
pub struct ColorDreams {
    chr_rom_converter: AddressConverter,
    prg_rom_converter: AddressConverter,
    mirror_type: MirrorType,
}

impl ColorDreams {
    pub fn new(nes_header: &NesHeader) -> Self {
        Self {
            chr_rom_converter: AddressConverter::new(0x0000, 8, 8, None),
            prg_rom_converter: AddressConverter::new(0x8000, 32, 32, None),
            mirror_type: nes_header.mirror_type,
        }
    }
}

impl Mapper for ColorDreams {
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
            CartridgePpuLocation::ChrRom(self.chr_rom_converter.convert(addr))
        } else if (0x2000..=0x3EFF).contains(&addr) {
            CartridgePpuLocation::VRam(addr - 0x2000)
        } else {
            CartridgePpuLocation::None
        }
    }

    fn mirror_type(&self) -> MirrorType {
        self.mirror_type
    }

    fn configure(&mut self, _addr: u16, value: u8) -> u8 {
        let old = (self.chr_rom_converter.bank << 4) | self.prg_rom_converter.bank;
        self.chr_rom_converter.bank = value & (0b11110000) >> 4;
        self.prg_rom_converter.bank = value & 0b00000011;

        old
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }

    fn ppu_bus_clock(&mut self) {}
}
