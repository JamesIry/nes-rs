use crate::{
    bus::InterruptFlags,
    nes::cartridge::{
        mappers::{CartridgeCpuLocation, CartridgePpuLocation},
        Mapper, MirrorType, NesHeader,
    },
};

use super::AddressConverter;

/**
 * Mapper 180
 */
pub struct UxRomInvert {
    mirror_type: MirrorType,
    prg_rom_converter: AddressConverter,
}

impl UxRomInvert {
    pub fn new(nes_header: &NesHeader) -> Self {
        Self {
            mirror_type: nes_header.mirror_type,
            prg_rom_converter: AddressConverter::new(8000, 16, 16, None),
        }
    }
}

impl Mapper for UxRomInvert {
    fn translate_cpu_addr(&mut self, addr: usize) -> CartridgeCpuLocation {
        if (0x4000..=0x7FFF).contains(&addr) {
            CartridgeCpuLocation::SRam(addr - 0x4000)
        } else if addr >= 0x8000 {
            let addr = if addr < 0x4000 {
                self.prg_rom_converter.convert_from_bank(0, addr)
            } else {
                self.prg_rom_converter.convert(addr)
            };
            CartridgeCpuLocation::PrgRom(addr)
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

    fn configure(&mut self, _addr: u16, value: u8) -> u8 {
        let old = self.prg_rom_converter.bank;
        self.prg_rom_converter.bank = value;
        old
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }

    fn ppu_bus_clock(&mut self) {}
}
