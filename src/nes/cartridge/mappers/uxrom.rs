use crate::{
    bus::InterruptFlags,
    nes::cartridge::{address_converters::AddressConverter, CartridgeCore, Mapper},
};

/**
 * Mapper 2
 */
pub struct UxRom {
    core: CartridgeCore,
}

impl UxRom {
    pub fn new(mut core: CartridgeCore) -> Self {
        core.prg_rom.converter.bank_size = 16;
        core.prg_rom.converter.window_size = 16;
        Self { core }
    }

    fn configure(&mut self, _addr: u16, value: u8) -> u8 {
        let old = self.core.prg_rom.converter.bank;
        self.core.prg_rom.converter.bank = value;
        old
    }
}

impl Mapper for UxRom {
    fn read_cpu(&mut self, addr: u16) -> u8 {
        if 0xC000 <= addr {
            self.core.prg_rom.read_from_bank(-1, addr)
        } else {
            self.core.read_cpu(addr)
        }
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
