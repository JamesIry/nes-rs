use crate::{
    bus::InterruptFlags,
    nes::cartridge::{CartridgeCore, Mapper, MirrorType},
};

const A12_SKIP_COUNT: u8 = 9;

/**
 * Mapper 4
 */
pub struct MMC3 {
    core: CartridgeCore,
    chr_bank_mode: u8,
    prg_bank_mode: u8,
    selected_register: usize,
    registers: [u8; 8],
    mirror_mode: u8,
    write_protection: bool,
    prg_ram_enable: bool,
    irq_latch: u8,
    irq_count: u8,
    irq_reload: bool,
    irq_enabled: bool,
    irq_occurred: bool,
    a12_state: A12State,
}

impl MMC3 {
    pub fn new(mut core: CartridgeCore) -> Self {
        core.chr_ram.set_bank_size_k(1);
        core.prg_rom.set_bank_size_k(8);
        let mut result = Self {
            core,
            chr_bank_mode: 0,
            prg_bank_mode: 0,
            selected_register: 0,
            registers: [0; 8],
            mirror_mode: 0,
            write_protection: false,
            prg_ram_enable: false,
            irq_latch: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_occurred: false,
            irq_count: 0,
            a12_state: A12State::WasLow(0),
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
            0xC000..=0xDFFE if addr & 1 == 0 => {
                let old = self.irq_latch;
                self.irq_latch = value;
                old
            }
            0xC001..=0xDFFF if addr & 1 == 1 => {
                let old = if self.irq_reload { 0xFF } else { 0 };
                self.irq_count = 0;
                self.irq_reload = true;
                old
            }
            0xE000..=0xFFFE if addr & 1 == 0 => {
                let old = if !self.irq_enabled { 0xFF } else { 0 };
                self.irq_enabled = false;
                self.irq_occurred = false;
                old
            }
            0xE001..=0xFFFF if addr & 1 == 1 => {
                let old = if self.irq_enabled { 0xFF } else { 0 };
                self.irq_enabled = true;
                old
            }
            _ => unreachable!("Invalid register {}", addr),
        }
    }

    fn reconfigure_banks(&mut self) {
        if self.chr_bank_mode == 0 {
            self.core
                .chr_ram
                .set_bank(0, (self.registers[0] & 0b1111_1110) as i16);
            self.core
                .chr_ram
                .set_bank(1, (self.registers[0] & 0b1111_1110) as i16 + 1);
            self.core
                .chr_ram
                .set_bank(2, (self.registers[1] & 0b1111_1110) as i16);
            self.core
                .chr_ram
                .set_bank(3, (self.registers[1] & 0b1111_1110) as i16 + 1);
            self.core.chr_ram.set_bank(4, self.registers[2] as i16);
            self.core.chr_ram.set_bank(5, self.registers[3] as i16);
            self.core.chr_ram.set_bank(6, self.registers[4] as i16);
            self.core.chr_ram.set_bank(7, self.registers[5] as i16);
        } else {
            self.core.chr_ram.set_bank(0, self.registers[2] as i16);
            self.core.chr_ram.set_bank(1, self.registers[3] as i16);
            self.core.chr_ram.set_bank(2, self.registers[4] as i16);
            self.core.chr_ram.set_bank(3, self.registers[5] as i16);
            self.core
                .chr_ram
                .set_bank(4, (self.registers[0] & 0b1111_1110) as i16);
            self.core
                .chr_ram
                .set_bank(5, (self.registers[0] & 0b1111_1110) as i16 + 1);
            self.core
                .chr_ram
                .set_bank(6, (self.registers[1] & 0b1111_1110) as i16);
            self.core
                .chr_ram
                .set_bank(7, (self.registers[1] & 0b1111_1110) as i16 + 1);
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

    fn check_a12(&mut self, addr: u16) {
        let a12_high = addr & 0b0001_0000_0000_0000 != 0;
        match (a12_high, self.a12_state) {
            (true, A12State::WasLow(n)) => {
                self.a12_state = A12State::WasHigh;
                if n == 0 {
                    self.clock_scanline();
                }
            }
            (true, A12State::WasHigh) => (),
            (false, A12State::WasHigh) => self.a12_state = A12State::WasLow(A12_SKIP_COUNT - 1),
            (false, A12State::WasLow(_)) => (),
        }
    }

    fn clock_scanline(&mut self) {
        if self.irq_reload || self.irq_count == 0 {
            self.irq_count = self.irq_latch;
            self.irq_reload = false;
        } else {
            self.irq_count = self.irq_count.wrapping_sub(1);
        }

        if self.irq_count == 0 {
            self.irq_occurred = self.irq_enabled;
        }
    }
}

impl Mapper for MMC3 {
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
        self.check_a12(addr);
        self.core.read_ppu(addr)
    }
    fn write_ppu(&mut self, addr: u16, value: u8) -> u8 {
        self.check_a12(addr);
        self.core.write_ppu(addr, value)
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        if self.irq_enabled && self.irq_occurred {
            InterruptFlags::IRQ
        } else {
            InterruptFlags::empty()
        }
    }

    fn ppu_bus_clock(&mut self) {
        match self.a12_state {
            A12State::WasLow(0) => (),
            A12State::WasLow(n) => self.a12_state = A12State::WasLow(n.wrapping_sub(1)),
            A12State::WasHigh => (),
        }
    }

    fn core(&self) -> &CartridgeCore {
        &self.core
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum A12State {
    WasLow(u8),
    WasHigh,
}
