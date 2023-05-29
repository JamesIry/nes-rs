#![allow(dead_code)]

use std::{cell::RefCell, rc::Rc};

use crate::bus::{Bus, BusDevice};

const CPU_ADDR_START: u16 = 0x2000;
const CPU_ADDR_END: u16 = 0x3FFF;
const CPU_ADDR_MASK: u16 = 0x0007;
const PALETTE_START: u16 = 0x3000;
const PALETTE_END: u16 = 0xFFFF;
const PALETTE_SIZE: usize = 0x1000;
const PALETTE_MASK: u16 = 0x0FFF;

/* Not really a PPU yet. Just some read/write registers */
pub struct PPU {
    ppu_ctrl: u8,
    ppu_mask: u8,
    ppu_status: u8,
    oam_addr: u8,
    oam_data: u8,
    ppu_scroll: u8,
    ppu_addr: u8,
    ppu_data: u8,
    bus: Bus,
    oamdata: [OAMData; 64],
    odd_frame: bool,
    pallettes: [u8; PALETTE_SIZE],
    reset_requested: bool,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            bus: Bus::new(),
            oamdata: [OAMData {
                sprite_y: 0,
                sprite_tile: 0,
                sprite_attribute: 0,
                sprite_x: 0,
            }; 64],
            ppu_ctrl: 0,
            ppu_mask: 0,
            ppu_status: 0,
            oam_addr: 0,
            oam_data: 0,
            ppu_scroll: 0,
            ppu_addr: 0,
            ppu_data: 0,
            odd_frame: false,
            pallettes: [0; PALETTE_SIZE],
            reset_requested: false,
        }
    }

    pub fn clock(&mut self) {
        if self.reset_requested {
            self.do_reset();
        }
    }

    pub fn reset(&mut self) {
        self.reset_requested = true;
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn BusDevice>>) {
        self.bus.add_device(device);
    }

    fn do_reset(&mut self) {
        self.ppu_ctrl = 0;
        self.ppu_mask = 0;
        self.ppu_scroll = 0;
        self.oam_data = 0;
        self.ppu_scroll = 0;
        self.odd_frame = false;
        self.reset_requested = false;
    }

    fn read_ppu_bus(&mut self, addr: u16) -> u8 {
        // TODO, pallette ram could just be another device on the ppu bus?
        if (PALETTE_START..=PALETTE_END).contains(&addr) {
            let physical = (addr & PALETTE_MASK) as usize;
            self.pallettes[physical]
        } else {
            self.bus.read(addr)
        }
    }

    fn write_ppu_bus(&mut self, addr: u16, data: u8) -> u8 {
        if (PALETTE_START..=PALETTE_END).contains(&addr) {
            let physical = (addr & PALETTE_MASK) as usize;
            let old = self.pallettes[physical];
            self.pallettes[physical] = data;
            old
        } else {
            self.bus.write(addr, data)
        }
    }

    fn write_ppu_ctrl(&mut self, data: u8) -> u8 {
        let old = self.ppu_ctrl;
        self.ppu_ctrl = data;
        old
    }

    fn write_ppu_mask(&mut self, data: u8) -> u8 {
        let old = self.ppu_mask;
        self.ppu_mask = data;
        old
    }

    fn read_ppu_status(&mut self) -> u8 {
        self.ppu_status
    }

    fn write_oam_addr(&mut self, data: u8) -> u8 {
        let old = self.oam_addr;
        self.oam_addr = data;
        old
    }

    fn read_oam_data(&mut self) -> u8 {
        self.oam_data
    }

    fn write_oam_data(&mut self, data: u8) -> u8 {
        let old = self.oam_data;
        self.oam_data = data;
        old
    }

    fn write_ppu_scroll(&mut self, data: u8) -> u8 {
        let old = self.ppu_scroll;
        self.ppu_scroll = data;
        old
    }

    fn write_ppu_addr(&mut self, data: u8) -> u8 {
        let old = self.ppu_addr;
        self.ppu_addr = data;
        old
    }

    fn read_ppu_data(&mut self) -> u8 {
        self.ppu_data
    }

    fn write_ppu_data(&mut self, data: u8) -> u8 {
        let old = self.ppu_data;
        self.ppu_data = data;
        old
    }
}

impl BusDevice for PPU {
    fn read(&mut self, addr: u16) -> Option<u8> {
        if (CPU_ADDR_START..=CPU_ADDR_END).contains(&addr) {
            Some(match addr & CPU_ADDR_MASK {
                0 => 0,
                1 => 0,
                2 => self.read_ppu_status(),
                3 => 0,
                4 => self.read_oam_data(),
                5 => 0,
                6 => 0,
                7 => self.read_ppu_data(),
                physical => unreachable!("reading from ppu register {}", physical),
            })
        } else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> Option<u8> {
        if (CPU_ADDR_START..=CPU_ADDR_END).contains(&addr) {
            Some(match addr & CPU_ADDR_MASK {
                0 => self.write_ppu_ctrl(data),
                1 => self.write_ppu_mask(data),
                2 => 0,
                3 => self.write_oam_addr(data),
                4 => self.write_oam_data(data),
                5 => self.write_ppu_scroll(data),
                6 => self.write_ppu_addr(data),
                7 => self.write_ppu_data(data),
                physical => unreachable!("reading from ppu register {}", physical),
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct OAMData {
    sprite_y: u8,
    sprite_tile: u8,
    sprite_attribute: u8,
    sprite_x: u8,
}
