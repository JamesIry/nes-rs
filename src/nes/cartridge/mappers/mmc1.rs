use crate::{
    bus::InterruptFlags,
    nes::cartridge::{CartridgeCore, Mapper, MirrorType},
};

/**
 * Mapper 1 (with force_sram_enable == false)
 * Mapper 155 (with force_sram_enable == true)
 */
pub struct MMC1 {
    core: CartridgeCore,

    sram_disabled: bool,
    force_sram_enable: bool,
    control_reg: u8,
    sram_bank_reg: u8,
    chr_bank_0_reg: u8,
    chr_bank_1_reg: u8,
    prg_bank_reg: u8,
    shift_register: u8,
    shift_count: u8,
    cycle_count: usize,
    last_write_cycle: usize,
}

impl MMC1 {
    pub fn new(mut core: CartridgeCore, force_sram_enable: bool) -> Self {
        core.sram.set_bank_size_k(8);
        let mut result = Self {
            core,

            sram_disabled: false,
            force_sram_enable,
            sram_bank_reg: 0,
            control_reg: 0b00001100,
            chr_bank_0_reg: 0,
            chr_bank_1_reg: 0,
            prg_bank_reg: 0,
            shift_register: 0,
            shift_count: 0,
            cycle_count: 0,
            last_write_cycle: 0xFFFF,
        };
        result.reconfigure_banks();

        result
    }

    fn configure(&mut self, addr: u16, value: u8) -> u8 {
        let old = self.shift_register;

        if self.cycle_count != self.last_write_cycle + 1 {
            self.last_write_cycle = self.cycle_count;
            if value & 0b00010000 != 0 {
                self.control_reg |= 0b00001100;
                self.shift_count = 0;
                self.shift_register = 0;
                self.reconfigure_banks();
            } else {
                self.shift_register >>= 1;
                self.shift_register |= (value & 0b00000001) << 4;
                self.shift_count += 1;
                if self.shift_count == 5 {
                    let value = self.shift_register;
                    self.shift_register = 0;
                    self.shift_count = 0;
                    match addr {
                        0x8000..=0x9FFF => self.control_reg = value,
                        0xA000..=0xBFFF => self.set_chr_reg(0, value),
                        0xC000..=0xDFFF => self.set_chr_reg(1, value),
                        0xE000..=0xFFFF => self.set_prg_reg(value),
                        _ => unreachable!("Couldn't find register for {}", addr),
                    }
                    self.reconfigure_banks();
                }
            }
        }
        old
    }

    fn reconfigure_banks(&mut self) {
        let mirror_type = match self.mirror_mode() {
            0 => MirrorType::SingleScreen(0),
            1 => MirrorType::SingleScreen(1),
            2 => MirrorType::Vertical,
            3 => MirrorType::Horizontal,
            _ => unreachable!("Invalid mirror mode {}", self.mirror_mode()),
        };
        self.core.vram.set_mirror_type(mirror_type);

        match self.prg_bank_mode() {
            0..=1 => {
                self.core.prg_rom.set_bank_size_k(32);
                self.core
                    .prg_rom
                    .set_bank(0, (self.prg_bank_reg >> 1) as i16);
            }
            2 => {
                self.core.prg_rom.set_bank_size_k(16);
                self.core.prg_rom.set_bank(0, 0);
                self.core.prg_rom.set_bank(1, self.prg_bank_reg as i16);
            }
            3 => {
                self.core.prg_rom.set_bank_size_k(16);
                self.core.prg_rom.set_bank(0, self.prg_bank_reg as i16);
                self.core.prg_rom.set_bank(1, -1);
            }
            _ => unreachable!("Invalid prg bank mode {}", self.prg_bank_mode()),
        };

        match self.chr_bank_mode() {
            0 => {
                self.core.chr_ram.set_bank_size_k(8);
                self.core
                    .chr_ram
                    .set_bank(0, (self.chr_bank_0_reg >> 1) as i16);
            }
            1 => {
                self.core.chr_ram.set_bank_size_k(4);
                self.core.chr_ram.set_bank(0, self.chr_bank_0_reg as i16);
                self.core.chr_ram.set_bank(1, self.chr_bank_1_reg as i16);
            }
            _ => unreachable!("Invalid chr bank mode {}", self.chr_bank_mode()),
        };

        self.core.sram.set_bank(0, self.sram_bank_reg as i16);
    }

    fn mirror_mode(&self) -> u8 {
        self.control_reg & 0b00000011
    }

    fn prg_bank_mode(&self) -> u8 {
        self.control_reg & 0b00001100 >> 2
    }

    fn chr_bank_mode(&self) -> u8 {
        self.control_reg & 0b00010000 >> 4
    }

    fn set_chr_reg(&mut self, reg: u8, value: u8) {
        let chr_ram_size = self.core.chr_ram.get_memory_size_k();

        let chr_bank_mask = match chr_ram_size {
            128.. => 0b00011111,
            64..=127 => 0b00001111,
            32..=63 => 0b0000111,
            16..=31 => 0b0000011,
            ..=15 => 0b00000001,
        };

        if reg == 0 {
            self.chr_bank_0_reg = value & chr_bank_mask;
        } else {
            self.chr_bank_1_reg = value & chr_bank_mask;
        }

        let sram_size = self.core.sram.get_memory_size_k();
        if sram_size == 16 {
            self.sram_bank_reg = value >> 3 & 0b00000001;
        } else {
            self.sram_bank_reg = value >> 2 & 0b00000011;
        }

        let prg_rom_size = self.core.prg_rom.get_memory_size_k();
        if prg_rom_size == 512 {
            self.prg_bank_reg = (self.prg_bank_reg & 0b00001111) | (value & 0b00010000)
        }
    }

    fn set_prg_reg(&mut self, value: u8) {
        self.prg_bank_reg = (self.prg_bank_reg & 0b11110000) | (value & 0b00001111);
        if !self.force_sram_enable {
            self.sram_disabled = value & 0b00010000 != 0;
        }
    }
}

impl Mapper for MMC1 {
    fn read_cpu(&mut self, addr: u16) -> u8 {
        if self.core.sram.contains_addr(addr) {
            if self.sram_disabled {
                0
            } else {
                self.core.sram.read(addr)
            }
        } else {
            self.core.read_cpu(addr)
        }
    }
    fn write_cpu(&mut self, addr: u16, value: u8) -> u8 {
        if self.core.sram.contains_addr(addr) {
            if self.sram_disabled {
                0
            } else {
                self.core.sram.write(addr, value)
            }
        } else if self.core.prg_rom.contains_addr(addr) {
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

    fn ppu_bus_clock(&mut self) {
        self.cycle_count += 1;
    }
}
