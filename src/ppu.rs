#![allow(dead_code)]

mod flags;

mod integration_tests;
#[cfg(test)]
mod unit_tests;

use std::{cell::RefCell, rc::Rc};

use crate::bus::{Bus, BusDevice};

use self::flags::{CtrlFlag, MaskFlag, StatusFlag};

const CPU_ADDR_START: u16 = 0x2000;
const CPU_ADDR_END: u16 = 0x3FFF;
const CPU_ADDR_MASK: u16 = 0x0007;
const PALETTE_START: u16 = 0x3000;
const PALETTE_END: u16 = 0xFFFF;
const PALETTE_SIZE: usize = 0x1000;
const PALETTE_MASK: u16 = 0x0FFF;
const OAM_SIZE: usize = 0x0100;

/* Not really a PPU yet. Just some read/write registers */
pub struct PPU {
    ppu_ctrl: u8,
    ppu_mask: u8,
    ppu_status: u8,
    oam_addr: u8,
    ppu_scroll_x: u8,
    ppu_scroll_y: u8,
    ppu_addr_high: u8,
    ppu_addr_low: u8,
    ppu_data: u8,
    bus: Bus,
    oam_table: [u8; OAM_SIZE],
    odd_frame: bool,
    pallettes: [u8; PALETTE_SIZE],
    reset_requested: bool,
    scan_line: i16,
    tick: u16,
    address_load_latch: bool,
    ppu_data_state: PpuDataState,
    last_read_latch: u8,
    nmi_requested: bool,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            bus: Bus::new(),
            oam_table: [0; OAM_SIZE],
            ppu_ctrl: 0,
            ppu_mask: 0,
            ppu_status: 0,
            oam_addr: 0,
            ppu_scroll_x: 0,
            ppu_scroll_y: 0,
            ppu_addr_high: 0,
            ppu_addr_low: 0,
            ppu_data: 0,
            odd_frame: false,
            pallettes: [0; PALETTE_SIZE],
            reset_requested: false,
            scan_line: -1,
            tick: 0,
            address_load_latch: false,
            ppu_data_state: PpuDataState::Idle,
            last_read_latch: 0,
            nmi_requested: false,
        }
    }

    #[must_use]
    pub fn clock(&mut self) -> bool {
        if self.reset_requested {
            self.do_reset();
            false
        } else {
            match (self.scan_line, self.tick) {
                (-1, 1) => {
                    self.ppu_status = 0;
                }
                (241, 1) => self.set_status_flag(StatusFlag::VerticalBlank, true),
                (s, _) if s < 240 => self.oam_addr = 0,
                _ => (),
            }

            self.ppu_data_update();

            self.tick += 1;
            if self.tick == 341 {
                self.tick = 0;
                self.scan_line += 1;
                if self.scan_line == 261 {
                    self.scan_line = -1;
                }
            }

            if self.nmi_requested {
                self.nmi_requested = false;
                true
            } else {
                false
            }
        }
    }

    pub fn reset(&mut self) {
        self.reset_requested = true;
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn BusDevice>>) {
        self.bus.add_device(device);
    }

    fn set_ctrl_flag(&mut self, flag: CtrlFlag, value: bool) {
        if value {
            self.ppu_ctrl |= flag;
        } else {
            self.ppu_ctrl &= !flag;
        }
    }

    fn read_ctrl_flag(&mut self, flag: CtrlFlag) -> bool {
        (self.ppu_ctrl & flag) != 0
    }

    fn set_mask_flag(&mut self, flag: MaskFlag, value: bool) {
        if value {
            self.ppu_mask |= flag;
        } else {
            self.ppu_mask &= !flag;
        }
    }

    fn read_mask_flag(&mut self, flag: MaskFlag) -> bool {
        (self.ppu_mask & flag) != 0
    }

    fn set_status_flag(&mut self, flag: StatusFlag, value: bool) {
        if value {
            self.ppu_status |= flag;
        } else {
            self.ppu_status &= !flag;
        }
    }

    fn read_status_flag(&mut self, flag: StatusFlag) -> bool {
        (self.ppu_status & flag) != 0
    }

    fn do_reset(&mut self) {
        self.ppu_ctrl = 0;
        self.ppu_mask = 0;
        self.ppu_scroll_x = 0;
        self.ppu_scroll_y = 0;
        self.oam_table = [0; 256];
        self.ppu_scroll_x = 0;
        self.ppu_scroll_y = 0;
        self.odd_frame = false;
        self.scan_line = -1;
        self.tick = 0;
        self.address_load_latch = false;
        self.reset_requested = false;
        self.last_read_latch = 0;
        self.ppu_data_state = PpuDataState::Idle;
        self.nmi_requested = false;
    }

    fn read_ppu_bus(&mut self, addr: u16) -> u8 {
        self.bus.read(addr)
    }

    fn write_ppu_bus(&mut self, addr: u16, data: u8) -> u8 {
        self.bus.write(addr, data)
    }

    fn write_ppu_ctrl(&mut self, data: u8) -> u8 {
        let old = self.ppu_ctrl;
        self.ppu_ctrl = data;

        if self.read_status_flag(StatusFlag::VerticalBlank)
            && (old & CtrlFlag::NmiEnabled == 0)
            && (data & CtrlFlag::NmiEnabled != 0)
        {
            self.nmi_requested = true;
        }

        old
    }

    fn write_ppu_mask(&mut self, data: u8) -> u8 {
        let old = self.ppu_mask;
        self.ppu_mask = data;
        old
    }

    fn read_ppu_status(&mut self) -> u8 {
        self.address_load_latch = false;
        let result = self.ppu_status;
        self.set_status_flag(StatusFlag::VerticalBlank, false);
        result
    }

    fn write_oam_addr(&mut self, data: u8) -> u8 {
        let old = self.oam_addr;
        self.oam_addr = data;
        old
    }

    fn read_oam_data(&mut self) -> u8 {
        self.oam_table[self.oam_addr as usize]
    }

    fn write_oam_data(&mut self, data: u8) -> u8 {
        let old = self.read_oam_data();
        if self.scan_line >= 240
            || (!self.read_mask_flag(MaskFlag::ShowSprites)
                && !self.read_mask_flag(MaskFlag::ShowBG))
        {
            self.oam_table[self.oam_addr as usize] = data;
        }

        self.oam_addr = self.oam_addr.wrapping_add(1);
        old
    }

    fn write_ppu_scroll(&mut self, data: u8) -> u8 {
        if !self.address_load_latch {
            let old = self.ppu_scroll_x;
            self.ppu_scroll_x = data;
            self.address_load_latch = true;
            old
        } else {
            let old = self.ppu_scroll_y;
            self.ppu_scroll_y = data;
            old
        }
    }

    fn write_ppu_addr(&mut self, data: u8) -> u8 {
        if !self.address_load_latch {
            let old = self.ppu_addr_high;
            self.ppu_addr_high = data;
            self.address_load_latch = true;
            old
        } else {
            let old = self.ppu_addr_low;
            self.ppu_addr_low = data;
            old
        }
    }

    fn inc_ppu_addr(&mut self, mut addr: u16) {
        addr = addr.wrapping_add(if self.read_ctrl_flag(CtrlFlag::IncrementAcross) {
            32
        } else {
            1
        });

        self.ppu_addr_low = (addr & 0xFF) as u8;
        self.ppu_addr_high = (addr >> 8) as u8;
    }

    fn read_ppu_data(&mut self) -> u8 {
        self.ppu_data_state = PpuDataState::NeedsRead;
        let addr = PPU::make_word(self.ppu_addr_low, self.ppu_addr_high);
        if (PALETTE_START..=PALETTE_END).contains(&addr) {
            let physical = (addr & PALETTE_MASK) as usize;
            self.pallettes[physical]
        } else {
            self.ppu_data
        }
    }

    fn write_ppu_data(&mut self, data: u8) -> u8 {
        let addr = PPU::make_word(self.ppu_addr_low, self.ppu_addr_high);
        if (PALETTE_START..=PALETTE_END).contains(&addr) {
            let physical = (addr & PALETTE_MASK) as usize;
            let old = self.pallettes[physical];
            self.pallettes[physical] = data;
            old
        } else {
            let old = self.ppu_data;
            self.ppu_data = data;
            // TODO does this state also get set when writing palette?
            self.ppu_data_state = PpuDataState::NeedsWrite;
            old
        }
    }

    fn ppu_data_update(&mut self) {
        match self.ppu_data_state {
            PpuDataState::Idle => (),
            PpuDataState::NeedsRead => {
                let addr = PPU::make_word(self.ppu_addr_low, self.ppu_addr_high);
                self.ppu_data = self.read_ppu_bus(addr);
                self.inc_ppu_addr(addr);
                self.ppu_data_state = PpuDataState::Idle;
            }
            PpuDataState::NeedsWrite => {
                let addr = PPU::make_word(self.ppu_addr_low, self.ppu_addr_high);
                self.write_ppu_bus(addr, self.ppu_data);
                self.inc_ppu_addr(addr);
                self.ppu_data_state = PpuDataState::Idle;
            }
        }
    }

    fn make_word(low: u8, high: u8) -> u16 {
        ((high as u16) << 8) | (low as u16)
    }
}

impl BusDevice for PPU {
    fn read(&mut self, addr: u16) -> Option<u8> {
        if (CPU_ADDR_START..=CPU_ADDR_END).contains(&addr) {
            let result = match addr & CPU_ADDR_MASK {
                0 => self.last_read_latch,
                1 => self.last_read_latch,
                2 => self.read_ppu_status(),
                3 => self.last_read_latch,
                4 => self.read_oam_data(),
                5 => self.last_read_latch,
                6 => self.last_read_latch,
                7 => self.read_ppu_data(),
                physical => unreachable!("reading from ppu register {}", physical),
            };
            self.last_read_latch = result;
            Some(result)
        } else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> Option<u8> {
        if (CPU_ADDR_START..=CPU_ADDR_END).contains(&addr) {
            self.last_read_latch = data;
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PpuDataState {
    Idle,
    NeedsRead,
    NeedsWrite,
}

#[cfg(test)]
pub fn create_test_configuration() -> (PPU, Rc<RefCell<crate::ram::RAM>>) {
    use crate::ram::RAM;

    let mut ppu = PPU::new();
    let mem = Rc::new(RefCell::new(RAM::new(0x0000, 0xFFFF, 0xFFFF)));
    ppu.add_device(mem.clone());
    (ppu, mem)
}
