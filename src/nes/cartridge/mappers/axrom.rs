use crate::{
    bus::InterruptFlags,
    nes::cartridge::{CartridgeCore, MirrorType},
};

use super::Mapper;

/**
 * Mapper 7
 */
pub struct AxRom {
    mirror_mode: u8,
    core: CartridgeCore,
}

impl AxRom {
    pub fn new(mut core: CartridgeCore) -> Self {
        core.vram.set_mirror_type(MirrorType::SingleScreen(0));
        core.prg_rom.set_bank_size_k(32);
        Self {
            mirror_mode: 0,
            core,
        }
    }

    fn configure(&mut self, _addr: u16, value: u8) -> u8 {
        let old = self.core.prg_rom.get_bank(0) as u8 | (self.mirror_mode << 4);
        self.core.prg_rom.set_bank(0, (value & 0b00000111) as i16);
        self.mirror_mode = (value & 0b00010000) >> 4;
        self.core
            .vram
            .set_mirror_type(MirrorType::SingleScreen(self.mirror_mode));
        old
    }
}

impl Mapper for AxRom {
    fn read_cpu(&mut self, addr: u16) -> u8 {
        self.core.read_cpu(addr)
    }
    fn write_cpu(&mut self, addr: u16, value: u8) -> u8 {
        if self.core.prg_rom.contains_addr(addr) {
            self.configure(addr, value)
        } else {
            self.core.write_cpu(addr, value)
        }
    }

    fn read_ppu(&mut self, addr: u16) -> u8 {
        self.core.read_ppu(addr)
    }

    fn write_ppu(&mut self, addr: u16, value: u8) -> u8 {
        self.core.write_ppu(addr, value)
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }

    fn ppu_bus_clock(&mut self) {}
}
