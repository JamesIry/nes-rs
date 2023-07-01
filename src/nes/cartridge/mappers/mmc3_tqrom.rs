use crate::{
    bus::InterruptFlags,
    nes::cartridge::{CartridgeCore, Mapper, MirrorType},
};

use super::mmc3_irq::MMC3Irq;
/**
 * Mapper 119
 */
pub struct MMC3TQRom {
    core: CartridgeCore,
    chr_bank_mode: u8,
    prg_bank_mode: u8,
    selected_register: usize,
    registers: [u8; 8],
    mirror_mode: u8,
    write_protection: bool,
    prg_ram_enable: bool,
    mmc3_irq: MMC3Irq,
}

impl MMC3TQRom {
    pub fn new(mut core: CartridgeCore) -> Self {
        core.chr_ram.set_bank_size_k(1);
        core.prg_rom.set_bank_size_k(8);
        core.chr_ram.set_alternate_memory(vec![0; 8 * 1024], false);

        let mut result = Self {
            core,
            chr_bank_mode: 0,
            prg_bank_mode: 0,
            selected_register: 0,
            registers: [0; 8],
            mirror_mode: 0,
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
            0xA000..=0xBFFE if addr & 1 == 0 => {
                let old = self.mirror_mode;
                self.mirror_mode = value & 0b1;
                self.reconfigure_banks();
                old
            }
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

    fn set_chr_bank(&mut self, page: usize, bank: u8) {
        let chr_ram = bank & 0b0100_0000 != 0;
        let bank = (bank & 0b0011_1111) as i16;

        self.core.chr_ram.set_bank(page, bank);
        self.core.chr_ram.select_alternate_memory(page, chr_ram);
    }

    fn reconfigure_banks(&mut self) {
        if self.chr_bank_mode == 0 {
            self.set_chr_bank(0, self.registers[0] & 0b1111_1110);
            self.set_chr_bank(1, (self.registers[0] & 0b1111_1110) + 1);
            self.set_chr_bank(2, self.registers[1] & 0b1111_1110);
            self.set_chr_bank(3, (self.registers[1] & 0b1111_1110) + 1);
            self.set_chr_bank(4, self.registers[2]);
            self.set_chr_bank(5, self.registers[3]);
            self.set_chr_bank(6, self.registers[4]);
            self.set_chr_bank(7, self.registers[5]);
        } else {
            self.set_chr_bank(0, self.registers[2]);
            self.set_chr_bank(1, self.registers[3]);
            self.set_chr_bank(2, self.registers[4]);
            self.set_chr_bank(3, self.registers[5]);
            self.set_chr_bank(4, self.registers[0] & 0b1111_1110);
            self.set_chr_bank(5, (self.registers[0] & 0b1111_1110) + 1);
            self.set_chr_bank(6, self.registers[1] & 0b1111_1110);
            self.set_chr_bank(7, (self.registers[1] & 0b1111_1110) + 1);
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

        if self.core.nes_header.mirror_type != MirrorType::FourScreen {
            let mirror_type = if self.mirror_mode == 0 {
                MirrorType::Vertical
            } else {
                MirrorType::Horizontal
            };
            self.core.vram.set_mirror_type(mirror_type);
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

impl Mapper for MMC3TQRom {
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
