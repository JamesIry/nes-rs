use crate::{
    bus::InterruptFlags,
    nes::cartridge::{
        mappers::{CartridgeCpuLocation, CartridgePpuLocation},
        Mapper, MirrorType,
    },
};

use super::AddressConverter;

/**
 * Mapper 94
 */
pub struct HvcUN1Rom {
    mirror_type: MirrorType,
    prg_rom_converter: AddressConverter,
}

impl HvcUN1Rom {
    pub fn new(mirror_type: MirrorType, prg_rom_size: usize) -> Self {
        Self {
            mirror_type,
            prg_rom_converter: AddressConverter::new(0x8000, 8, 16, Some(prg_rom_size)),
        }
    }
}

impl Mapper for HvcUN1Rom {
    fn translate_cpu_addr(&mut self, addr: usize) -> CartridgeCpuLocation {
        if (0x4000..=0x7FFF).contains(&addr) {
            CartridgeCpuLocation::SRam(addr - 0x4000)
        } else if addr >= 0x8000 {
            let addr = if addr < 0xC000 {
                self.prg_rom_converter.convert(addr)
            } else {
                self.prg_rom_converter.convert_from_bank(-1, addr)
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
        let old = self.prg_rom_converter.bank >> 2;
        self.prg_rom_converter.bank = (value & 0b00011100) << 2;
        old
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }

    fn ppu_bus_clock(&mut self) {}
}
