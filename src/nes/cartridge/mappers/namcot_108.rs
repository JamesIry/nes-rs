use crate::{
    bus::InterruptFlags,
    nes::cartridge::{CartridgeCore, Mapper},
};

/**
 * Mapper 206
 */
pub struct Namcot108 {
    core: CartridgeCore,
    selected_register: usize,
    registers: [u8; 8],
}

impl Namcot108 {
    pub fn new(mut core: CartridgeCore) -> Self {
        core.chr_ram.set_bank_size_k(1);
        core.prg_rom.set_bank_size_k(8);
        let mut result = Self {
            core,
            selected_register: 0,
            registers: [0; 8],
        };
        result.registers[0] = 0;
        result.registers[1] = 2;
        result.registers[2] = 4;
        result.registers[3] = 5;
        result.registers[4] = 6;
        result.registers[5] = 7;

        result.registers[6] = 0;
        result.registers[7] = 1;

        result.reconfigure_banks();
        result
    }

    fn configure(&mut self, addr: u16, value: u8) -> u8 {
        match addr {
            0x8000..=0x9FFE if addr & 1 == 0 => {
                let old = self.set_bank_select(value);
                self.reconfigure_banks();
                old
            }
            0x8001..=0x9FFF if addr & 1 == 1 => {
                let old = self.registers[self.selected_register];
                self.registers[self.selected_register] = value;
                self.reconfigure_banks();
                old
            }
            0xA000..=0xFFFF => 0,
            _ => unreachable!("Invalid register {}", addr),
        }
    }

    fn reconfigure_banks(&mut self) {
        self.core
            .chr_ram
            .set_bank(0, (self.registers[0] & 0b0011_1110) as i16);
        self.core
            .chr_ram
            .set_bank(1, (self.registers[0] & 0b0011_1110) as i16 + 1);
        self.core
            .chr_ram
            .set_bank(2, (self.registers[1] & 0b0011_1110) as i16);
        self.core
            .chr_ram
            .set_bank(3, (self.registers[1] & 0b0011_1110) as i16 + 1);
        self.core
            .chr_ram
            .set_bank(4, (self.registers[2] & 0b0011_1111) as i16);
        self.core
            .chr_ram
            .set_bank(5, (self.registers[3] & 0b0011_1111) as i16);
        self.core
            .chr_ram
            .set_bank(6, (self.registers[4] & 0b0011_1111) as i16);
        self.core
            .chr_ram
            .set_bank(7, (self.registers[5] & 0b0011_1111) as i16);

        self.core
            .prg_rom
            .set_bank(0, (self.registers[6] & 0b0000_1111) as i16);
        self.core
            .prg_rom
            .set_bank(1, (self.registers[7] & 0b0000_1111) as i16);
        self.core.prg_rom.set_bank(2, -2);
        self.core.prg_rom.set_bank(3, -1);
    }

    fn read_bank_select(&self) -> u8 {
        self.selected_register as u8 & 0b111
    }
    fn set_bank_select(&mut self, value: u8) -> u8 {
        let old = self.read_bank_select();
        self.selected_register = (value & 0b111) as usize;
        old
    }
}

impl Mapper for Namcot108 {
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
