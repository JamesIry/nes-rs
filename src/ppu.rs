use std::{cell::RefCell, rc::Weak};

use crate::bus::{Bus, BusDevice, Processor};

static RANGE_START: u16 = 0x2000;
static RANGE_END: u16 = 0x3FFF;
static ADDR_MASK: u16 = 0x0007;

/* Not really a PPU yet. Just some read/write registers */
pub struct PPU {
    registers: [u8; 8],
    bus: Weak<RefCell<Bus>>,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            registers: [0; 8],
            bus: Weak::new(),
        }
    }

    fn physical(addr: u16) -> usize {
        ((addr & ADDR_MASK) - (RANGE_START & ADDR_MASK)) as usize
    }
}

impl BusDevice for PPU {
    fn read(&mut self, addr: u16) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            Some(self.registers[PPU::physical(addr)])
        } else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            let old = self.registers[PPU::physical(addr)];
            self.registers[PPU::physical(addr)] = data;
            Some(old)
        } else {
            None
        }
    }
}

impl Processor for PPU {
    fn clock(&mut self) {}

    fn reset(&mut self) {}

    fn nmi(&mut self) {}

    fn irq(&mut self) {}

    fn stuck(&self) -> bool {
        false
    }

    fn set_bus(&mut self, bus: std::rc::Weak<std::cell::RefCell<crate::bus::Bus>>) {
        self.bus = bus;
    }
}
