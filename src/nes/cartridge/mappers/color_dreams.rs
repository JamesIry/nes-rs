use crate::{
    bus::InterruptFlags,
    nes::cartridge::{CartridgeCore, Mapper},
};

/**
 * Mapper 11
 */
pub struct ColorDreams {
    core: CartridgeCore,
}

impl ColorDreams {
    pub fn new(mut core: CartridgeCore) -> Self {
        core.prg_rom.set_bank_size_k(32);
        core.chr_ram.set_bank_size_k(9);
        Self { core }
    }

    fn configure(&mut self, _addr: u16, value: u8) -> u8 {
        let old =
            ((self.core.chr_ram.get_bank(0) << 4) as u8) | (self.core.prg_rom.get_bank(0) as u8);
        self.core
            .chr_ram
            .set_bank(0, (value & ((0b11110000) >> 4)) as i16);
        self.core.prg_rom.set_bank(0, (value & 0b00000011) as i16);

        old
    }
}

impl Mapper for ColorDreams {
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

    fn core(&self) -> &CartridgeCore {
        &self.core
    }
}
