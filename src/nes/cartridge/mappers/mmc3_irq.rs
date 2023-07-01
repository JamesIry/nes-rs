use crate::bus::InterruptFlags;

const A12_SKIP_COUNT: u8 = 9;

/**
 * Core IRQ logic for variouis MMC3 based mappers
 */
pub struct MMC3Irq {
    irq_latch: u8,
    irq_count: u8,
    irq_reload: bool,
    irq_enabled: bool,
    irq_occurred: bool,
    a12_state: A12State,
}

impl MMC3Irq {
    pub fn new() -> Self {
        Self {
            irq_latch: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_occurred: false,
            irq_count: 0,
            a12_state: A12State::WasLow(0),
        }
    }

    pub fn configure(&mut self, addr: u16, value: u8) -> u8 {
        match addr {
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

    pub fn check_a12(&mut self, addr: u16) {
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

    pub fn cpu_bus_clock(&mut self) -> InterruptFlags {
        if self.irq_enabled && self.irq_occurred {
            InterruptFlags::IRQ
        } else {
            InterruptFlags::empty()
        }
    }

    pub fn ppu_bus_clock(&mut self) {
        match self.a12_state {
            A12State::WasLow(0) => (),
            A12State::WasLow(n) => self.a12_state = A12State::WasLow(n.wrapping_sub(1)),
            A12State::WasHigh => (),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum A12State {
    WasLow(u8),
    WasHigh,
}
