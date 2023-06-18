use crate::{
    bus::InterruptFlags,
    nes::cartridge::{address_converters::AddressConverter, CartridgeCore, Mapper},
};

/**
 * Mapper 94
 */
pub struct HvcUN1Rom {
    core: CartridgeCore,
}

impl HvcUN1Rom {
    pub fn new(mut core: CartridgeCore) -> Self {
        core.prg_rom.converter.bank_size = 16;
        core.prg_rom.converter.window_size = 16;
        Self { core }
    }

    fn configure(&mut self, _addr: u16, value: u8) -> u8 {
        let old = self.core.prg_rom.converter.bank >> 2;
        self.core.prg_rom.converter.bank = (value & 0b00011100) << 2;
        old
    }
}

impl Mapper for HvcUN1Rom {
    fn read_cpu(&mut self, addr: u16) -> u8 {
        self.core.read_cpu(addr)
    }
    fn write_cpu(&mut self, addr: u16, value: u8) -> u8 {
        if self.core.prg_rom.converter.contains_addr(addr) {
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
