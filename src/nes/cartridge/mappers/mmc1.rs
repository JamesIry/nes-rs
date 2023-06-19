use crate::{
    bus::InterruptFlags,
    nes::cartridge::{address_converters::AddressConverter, CartridgeCore, Mapper, MirrorType},
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
        core.sram.converter.bank_size_k = 8;
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
        result.reconfigure();

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
                self.reconfigure();
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
                    self.reconfigure();
                }
            }
        }
        old
    }

    fn reconfigure(&mut self) {
        self.core.vram.converter.mirror_type = match self.mirror_mode() {
            0 => MirrorType::SingleScreen(0),
            1 => MirrorType::SingleScreen(1),
            2 => MirrorType::Vertical,
            3 => MirrorType::Horizontal,
            _ => unreachable!("Invalid mirror mode {}", self.mirror_mode()),
        };

        let prg_bank_size = match self.prg_bank_mode() {
            0..=1 => 32,
            2..=3 => 16,
            _ => unreachable!("Invalid prg bank mode {}", self.prg_bank_mode()),
        };
        self.core.prg_rom.converter.bank_size_k = prg_bank_size;

        let chr_bank_size = match self.chr_bank_mode() {
            0 => 8,
            1 => 4,
            _ => unreachable!("Invalid chr bank mode {}", self.chr_bank_mode()),
        };
        self.core.chr_ram.converter.bank_size_k = chr_bank_size;
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

    fn prg_bank(&self, addr: u16) -> i16 {
        match (self.prg_bank_mode(), addr) {
            (0..=1, _) => (self.prg_bank_reg >> 1) as i16,
            (2, 0x8000..=0xBFFF) => 0,
            (2, 0xC000..=0xFFFF) => self.prg_bank_reg as i16,
            (3, 0x8000..=0xBFFF) => self.prg_bank_reg as i16,
            (3, 0xC000..=0xFFFF) => -1,
            _ => unreachable!(
                "Invalid prg bank mode or address {} {}",
                self.prg_bank_mode(),
                addr
            ),
        }
    }

    fn chr_bank(&self, addr: u16) -> i16 {
        let bank = match (self.chr_bank_mode(), addr) {
            (0, _) => self.chr_bank_0_reg >> 1,
            (1, 0x0000..=0x0FFF) => self.chr_bank_0_reg,
            (1, 0x1000..=0x1FFF) => self.chr_bank_1_reg,
            _ => unreachable!(
                "Invalid chr bank mode or address {} {}",
                self.chr_bank_mode(),
                addr
            ),
        };
        bank as i16
    }

    fn set_chr_reg(&mut self, reg: u8, value: u8) {
        let chr_ram_size = self.core.chr_ram.converter.max_size / 1024;

        let chr_bank_mask = match chr_ram_size {
            128.. => 0b00011111,
            64..=127 => 0b00001111,
            32..=63 => 0b0000111,
            16..=31 => 0b0000011,
            ..=15 => 0b00000001,
            _ => unreachable!(),
        };

        if reg == 0 {
            self.chr_bank_0_reg = value & chr_bank_mask;
        } else {
            self.chr_bank_1_reg = value & chr_bank_mask;
        }

        let sram_size = self.core.sram.converter.max_size / 1024;
        if sram_size == 16 {
            self.sram_bank_reg = value >> 3 & 0b00000001;
        } else {
            self.sram_bank_reg = value >> 2 & 0b00000011;
        }

        let prg_rom_size = self.core.prg_rom.converter.max_size / 1024;
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
        if self.core.sram.converter.contains_addr(addr) {
            if self.sram_disabled {
                0
            } else {
                self.core
                    .sram
                    .read_from_bank(self.sram_bank_reg as i16, addr)
            }
        } else if self.core.prg_rom.converter.contains_addr(addr) {
            let bank = self.prg_bank(addr);
            self.core.prg_rom.read_from_bank(bank, addr)
        } else {
            self.core.read_cpu(addr)
        }
    }
    fn write_cpu(&mut self, addr: u16, value: u8) -> u8 {
        if self.core.sram.converter.contains_addr(addr) {
            if self.sram_disabled {
                0
            } else {
                self.core
                    .sram
                    .write_to_bank(self.sram_bank_reg as i16, addr, value)
            }
        } else if self.core.prg_rom.converter.contains_addr(addr) {
            self.configure(addr, value)
        } else {
            self.core.write_cpu(addr, value)
        }
    }

    fn read_ppu(&mut self, addr: u16) -> u8 {
        if self.core.chr_ram.converter.contains_addr(addr) {
            let bank = self.chr_bank(addr);
            self.core.chr_ram.read_from_bank(bank, addr)
        } else {
            self.core.read_ppu(addr)
        }
    }

    fn write_ppu(&mut self, addr: u16, value: u8) -> u8 {
        if self.core.chr_ram.converter.contains_addr(addr) {
            let bank = self.chr_bank(addr);
            self.core.chr_ram.write_to_bank(bank, addr, value)
        } else {
            self.core.write_ppu(addr, value)
        }
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }

    fn ppu_bus_clock(&mut self) {
        self.cycle_count += 1;
    }
}
