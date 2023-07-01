use crate::{
    bus::InterruptFlags,
    nes::cartridge::{CartridgeCore, Mapper},
};

use super::mmc3_irq::MMC3Irq;
/**
 * Mapper 118
 */
pub struct MMC3TxSRom {
    core: CartridgeCore,
    chr_bank_mode: u8,
    prg_bank_mode: u8,
    selected_register: usize,
    registers: [u8; 8],
    write_protection: bool,
    prg_ram_enable: bool,
    mmc3_irq: MMC3Irq,
}

impl MMC3TxSRom {
    pub fn new(mut core: CartridgeCore) -> Self {
        core.chr_ram.set_bank_size_k(1);
        core.prg_rom.set_bank_size_k(8);
        let mut result = Self {
            core,
            chr_bank_mode: 0,
            prg_bank_mode: 0,
            selected_register: 0,
            registers: [0; 8],
            write_protection: false,
            prg_ram_enable: false,
            mmc3_irq: MMC3Irq::new(),
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
            0xA000..=0xBFFE if addr & 1 == 0 => 0,
            0xA001..=0xBFFF if addr & 1 == 1 => {
                let old = if self.write_protection {
                    0b1000_0000
                } else {
                    0
                } | if self.prg_ram_enable { 0b0100_0000 } else { 0 };
                self.write_protection = value & 0b1000_0000 != 0;
                self.prg_ram_enable = value & 0b0100_0000 != 0;
                old
            }
            0xC000..=0xFFFF => self.mmc3_irq.configure(addr, value),
            _ => unreachable!("Invalid register {}", addr),
        }
    }

    fn reconfigure_banks(&mut self) {
        if self.chr_bank_mode == 0 {
            self.core
                .chr_ram
                .set_bank(0, (self.registers[0] & 0b0111_1110) as i16);
            self.core
                .chr_ram
                .set_bank(1, (self.registers[0] & 0b0111_1110) as i16 + 1);
            self.core
                .chr_ram
                .set_bank(2, (self.registers[1] & 0b0111_1110) as i16);
            self.core
                .chr_ram
                .set_bank(3, (self.registers[1] & 0b0111_1110) as i16 + 1);
            self.core
                .chr_ram
                .set_bank(4, (self.registers[2] & 0b0111_1111) as i16);
            self.core
                .chr_ram
                .set_bank(5, (self.registers[3] & 0b0111_1111) as i16);
            self.core
                .chr_ram
                .set_bank(6, (self.registers[4] & 0b0111_1111) as i16);
            self.core
                .chr_ram
                .set_bank(7, (self.registers[5] & 0b0111_1111) as i16);
        } else {
            self.core
                .chr_ram
                .set_bank(0, (self.registers[2] & 0b0111_1111) as i16);
            self.core
                .chr_ram
                .set_bank(1, (self.registers[3] & 0b0111_1111) as i16);
            self.core
                .chr_ram
                .set_bank(2, (self.registers[4] & 0b0111_1111) as i16);
            self.core
                .chr_ram
                .set_bank(3, (self.registers[5] & 0b0111_1111) as i16);
            self.core
                .chr_ram
                .set_bank(4, (self.registers[0] & 0b0111_1110) as i16);
            self.core
                .chr_ram
                .set_bank(5, (self.registers[0] & 0b0111_1110) as i16 + 1);
            self.core
                .chr_ram
                .set_bank(6, (self.registers[1] & 0b0111_1110) as i16);
            self.core
                .chr_ram
                .set_bank(7, (self.registers[1] & 0b0111_1110) as i16 + 1);
        }

        if self.prg_bank_mode == 0 {
            self.core.prg_rom.set_bank(0, self.registers[6] as i16);
            self.core.prg_rom.set_bank(1, self.registers[7] as i16);
            self.core.prg_rom.set_bank(2, -2);
            self.core.prg_rom.set_bank(3, -1);
        } else {
            self.core.prg_rom.set_bank(0, -2);
            self.core.prg_rom.set_bank(1, self.registers[7] as i16);
            self.core.prg_rom.set_bank(2, self.registers[6] as i16);
            self.core.prg_rom.set_bank(3, -1);
        }

        if self.chr_bank_mode == 0 {
            self.core.vram.set_bank(0, (self.registers[0] >> 7) as i16);
            self.core.vram.set_bank(1, (self.registers[0] >> 7) as i16);
            self.core.vram.set_bank(2, (self.registers[1] >> 7) as i16);
            self.core.vram.set_bank(3, (self.registers[1] >> 7) as i16);
            self.core.vram.set_bank(4, (self.registers[0] >> 7) as i16);
            self.core.vram.set_bank(5, (self.registers[0] >> 7) as i16);
            self.core.vram.set_bank(6, (self.registers[1] >> 7) as i16);
            self.core.vram.set_bank(7, (self.registers[1] >> 7) as i16);
        } else {
            self.core.vram.set_bank(0, (self.registers[2] >> 7) as i16);
            self.core.vram.set_bank(1, (self.registers[3] >> 7) as i16);
            self.core.vram.set_bank(2, (self.registers[4] >> 7) as i16);
            self.core.vram.set_bank(3, (self.registers[5] >> 7) as i16);
            self.core.vram.set_bank(4, (self.registers[2] >> 7) as i16);
            self.core.vram.set_bank(5, (self.registers[3] >> 7) as i16);
            self.core.vram.set_bank(6, (self.registers[4] >> 7) as i16);
            self.core.vram.set_bank(7, (self.registers[5] >> 7) as i16);
        }
    }

    fn read_bank_select(&self) -> u8 {
        ((self.chr_bank_mode & 0b1) << 7)
            | ((self.prg_bank_mode & 0b1) << 6)
            | (self.selected_register as u8 & 0b111)
    }
    fn set_bank_select(&mut self, value: u8) -> u8 {
        let old = self.read_bank_select();
        self.chr_bank_mode = (value >> 7) & 0b1;
        self.prg_bank_mode = (value >> 6) & 0b1;
        self.selected_register = (value & 0b111) as usize;
        old
    }
}

impl Mapper for MMC3TxSRom {
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
        self.mmc3_irq.check_a12(addr);
        self.core.read_ppu(addr)
    }
    fn write_ppu(&mut self, addr: u16, value: u8) -> u8 {
        self.mmc3_irq.check_a12(addr);
        self.core.write_ppu(addr, value)
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        self.mmc3_irq.cpu_bus_clock()
    }

    fn ppu_bus_clock(&mut self) {
        self.mmc3_irq.ppu_bus_clock()
    }

    fn core(&self) -> &CartridgeCore {
        &self.core
    }
}
