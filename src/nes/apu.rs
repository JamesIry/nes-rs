#![allow(clippy::upper_case_acronyms)]

use std::{cell::RefCell, rc::Rc};

use crate::{bus::BusDevice, cpu::CPU};

static RANGE_START: u16 = 0x4000;
static RANGE_END: u16 = 0x401F;
static ADDR_MASK: u16 = 0x001F;

/* Not really a APU yet. Just some read/write registers and oam dma */
pub struct APU {
    registers: [u8; 0x20],
    cpu: Rc<RefCell<CPU>>,
    read_cycle: bool,
    oam_dma_state: OamDmaState,
    oam_dma_data: u8,
    oam_dma_page: u8,
}

impl APU {
    pub fn new(cpu: Rc<RefCell<CPU>>) -> Self {
        Self {
            registers: [0xFF; 0x20],
            cpu,
            read_cycle: true,
            oam_dma_state: OamDmaState::NoDma,
            oam_dma_data: 0,
            oam_dma_page: 0,
        }
    }

    pub fn clock(&mut self) {
        self.read_cycle = !self.read_cycle;

        self.oam_dma();
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.registers = [0xFF; 0x20];
        self.read_cycle = true;
        self.oam_dma_state = OamDmaState::NoDma;
        self.oam_dma_data = 0;
        self.oam_dma_page = 0;
    }

    fn oam_dma(&mut self) {
        match (self.oam_dma_state, self.read_cycle) {
            (OamDmaState::NoDma, _) => (),
            (OamDmaState::Requested, true) => {
                self.cpu.as_ref().borrow_mut().set_rdy(false);
                self.oam_dma_state = OamDmaState::Ready;
            }
            (OamDmaState::Requested, false) => (),
            (OamDmaState::Ready, false) => {
                self.read_oam_dma(0);
                self.oam_dma_state = OamDmaState::Executing(0);
            }
            (OamDmaState::Ready, true) => unreachable!("Oam state ready on even frame"),
            (OamDmaState::Executing(offset), true) => self.read_oam_dma(offset as u16),
            (OamDmaState::Executing(offset), false) => {
                self.write_oam_dma();
                if offset == 255 {
                    self.cpu.as_ref().borrow_mut().set_rdy(true);
                    self.oam_dma_state = OamDmaState::NoDma;
                } else {
                    self.oam_dma_state = OamDmaState::Executing(offset + 1);
                }
            }
        }
    }

    fn read_oam_dma(&mut self, offset: u16) {
        let addr = ((self.oam_dma_page as u16) << 8) | offset;
        self.oam_dma_data = self.cpu.as_ref().borrow_mut().read_bus_byte(addr);
    }

    fn write_oam_dma(&self) {
        self.cpu
            .as_ref()
            .borrow_mut()
            .write_bus_byte(0x2004, self.oam_dma_data);
    }

    fn physical(addr: u16) -> usize {
        ((addr & ADDR_MASK) - (RANGE_START & ADDR_MASK)) as usize
    }
}

impl BusDevice for APU {
    fn read(&mut self, addr: u16) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            Some(self.registers[APU::physical(addr)])
        } else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            let physical = APU::physical(addr);
            if physical == 0x15 {
                self.oam_dma_page = data;
                self.oam_dma_state = OamDmaState::Requested;
            }
            let old = self.registers[physical];
            self.registers[physical] = data;
            Some(old)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OamDmaState {
    NoDma,
    Requested,
    Ready,
    Executing(u8),
}
