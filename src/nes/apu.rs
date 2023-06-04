#![allow(clippy::upper_case_acronyms)]

use std::{cell::RefCell, rc::Rc};

use crate::{bus::BusDevice, cpu::CPU};

const RANGE_START: u16 = 0x4000;
const RANGE_END: u16 = 0x401F;
const ADDR_MASK: u16 = 0x001F;

const REGISTER_OAM_DMA_PAGE: usize = 0x15;
const REGISTER_INPUT1: usize = 0x16;
const REGISTER_INPUT2: usize = 0x17;

/* Not really a APU yet. Just some read/write registers and oam dma */
pub struct APU {
    registers: [u8; 0x20],
    cpu: Rc<RefCell<CPU>>,
    read_cycle: bool,
    oam_dma_state: OamDmaState,
    oam_dma_data: u8,
    input_port_ctrl: u8,
    input_port1: u8,
    input_port2: u8,
}

impl APU {
    pub fn new(cpu: Rc<RefCell<CPU>>) -> Self {
        Self {
            registers: [0xFF; 0x20],
            cpu,
            read_cycle: true,
            oam_dma_state: OamDmaState::NoDma,
            oam_dma_data: 0,
            input_port_ctrl: 0xFF,
            input_port1: 0,
            input_port2: 0,
        }
    }

    pub fn clock(&mut self) {
        self.read_cycle = !self.read_cycle;

        self.manage_input_ports();

        self.manage_oam_dma();
    }

    pub fn reset(&mut self) {
        self.registers = [0xFF; 0x20];
        self.read_cycle = true;
        self.oam_dma_state = OamDmaState::NoDma;
        self.oam_dma_data = 0;
        self.input_port_ctrl = 0xFF;
    }

    pub fn set_input_port1(&mut self, value: u8) {
        self.input_port1 = value;
    }

    pub fn set_input_port2(&mut self, value: u8) {
        self.input_port2 = value;
    }

    fn manage_oam_dma(&mut self) {
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
        let addr = ((self.registers[REGISTER_OAM_DMA_PAGE] as u16) << 8) | offset;
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

    fn manage_input_ports(&mut self) {
        if self.input_port_ctrl & 0b00000001 != 0 {
            self.registers[REGISTER_INPUT1] = self.input_port1;
            self.registers[REGISTER_INPUT2] = self.input_port2;
        }
    }

    fn bus_read_input_register(&mut self, physical: usize) -> u8 {
        let result = self.registers[physical] & 0b00000001;
        if self.input_port_ctrl & 0b00000001 == 0 {
            self.registers[physical] >>= 1;
            self.registers[physical] |= 0b10000000;
        }
        result
    }
}

#[allow(clippy::manual_range_contains)]
impl BusDevice for APU {
    fn read(&mut self, addr: u16) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            let physical = APU::physical(addr);
            Some(match physical {
                REGISTER_INPUT1 => self.bus_read_input_register(physical),
                REGISTER_INPUT2 => self.bus_read_input_register(physical),
                _ => self.registers[physical],
            })
        } else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            let physical = APU::physical(addr);
            let old = self.registers[physical];
            match physical {
                REGISTER_OAM_DMA_PAGE => {
                    self.oam_dma_state = OamDmaState::Requested;
                    self.registers[physical] = data
                }
                REGISTER_INPUT1 => {
                    self.input_port_ctrl = data;
                }
                _ => self.registers[physical] = data,
            };
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
