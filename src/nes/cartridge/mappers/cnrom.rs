use crate::{
    bus::InterruptFlags,
    nes::cartridge::{CartridgeCore, Mapper},
};

/**
 * Mapper 3 (without copy protection)
 * Mapper 185 (with copy protection)
 */
pub struct CNRom {
    core: CartridgeCore,
    remaining_junk_reads: u8,
}

impl CNRom {
    pub fn new(mut core: CartridgeCore, copy_protection: bool) -> Self {
        core.chr_ram.set_bank_size_k(8);
        let remaining_junk_reads = if copy_protection { 2 } else { 0 };
        Self {
            core,
            remaining_junk_reads,
        }
    }

    fn configure(&mut self, _addr: u16, value: u8) -> u8 {
        let old = self.core.chr_ram.get_bank(0) as u8;
        self.core.chr_ram.set_bank(0, value as i16);
        old
    }
}

impl Mapper for CNRom {
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
        if addr == 0x2007 && self.remaining_junk_reads > 0 {
            self.remaining_junk_reads -= 1;
            0xFF
        } else {
            self.core.read_ppu(addr)
        }
    }

    fn write_ppu(&mut self, addr: u16, value: u8) -> u8 {
        self.core.write_ppu(addr, value)
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }

    fn ppu_bus_clock(&mut self) {}
}
