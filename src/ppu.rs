#![allow(dead_code)]

mod flags;
mod rgb;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod unit_tests;

use std::{cell::RefCell, rc::Rc};

use crate::bus::{Bus, BusDevice};

use self::flags::{CtrlFlag, MaskFlag, StatusFlag};

const CPU_ADDR_START: u16 = 0x2000;
const CPU_ADDR_END: u16 = 0x3FFF;
const PALETTE_START: u16 = 0x3000;
const PALETTE_END: u16 = 0xFFFF;
const PALETTE_SIZE: usize = 0x1000;
const PALETTE_MASK: u16 = 0x0FFF;
const OAM_SIZE: usize = 0x0100;

/* Not really a PPU yet. Just some read/write registers */
pub struct PPU {
    renderer: fn(u16, u16, (u8, u8, u8)) -> (),
    ppu_ctrl_high: u8,
    ppu_mask: u8,
    ppu_status: u8,
    oam_addr: u8,
    ppu_data_buffer: u8,
    bus: Bus,
    oam_table: [u8; OAM_SIZE],
    pallettes: [u8; PALETTE_SIZE],
    scan_line: i16,
    tick: u16,

    nmi_requested: bool,
    even_frame: bool,

    write_toggle: bool,

    /**
     * 0 yyy NN YYYYY XXXXX
     * | ||| || ||||| +++++-- coarse X scroll
     * | ||| || +++++-------- coarse Y scroll
     * | ||| ++-------------- nametable select
     * | +++----------------- fine Y scroll
     * +--------------------- unused 0
     */
    vram_address: u16,
    temporary_vram_address: u16,
    fine_x: u8,
}

pub fn nul_renderer(_x: u16, _y: u16, _rgb: (u8, u8, u8)) {}

impl PPU {
    pub fn new(renderer: fn(u16, u16, (u8, u8, u8)) -> ()) -> Self {
        Self {
            renderer,
            bus: Bus::new(),
            oam_table: [0; OAM_SIZE],
            ppu_ctrl_high: 0,
            ppu_mask: 0,
            ppu_status: 0,
            oam_addr: 0,
            ppu_data_buffer: 0,
            pallettes: [0; PALETTE_SIZE],
            scan_line: -1,
            tick: 0,
            nmi_requested: false,
            even_frame: true,
            vram_address: 0,
            temporary_vram_address: 0,
            write_toggle: false,
            fine_x: 0,
        }
    }

    #[must_use]
    pub fn clock(&mut self) -> bool {
        // manage status
        match (self.scan_line, self.tick) {
            (-1, 1) => {
                self.ppu_status = 0; // clear StatusFlag::VerticalBlank, StatusFlag::Sprite0Hit, and StatusFlag::SpriteOverflow
            }
            (241, 1) => {
                self.set_status_flag(StatusFlag::VerticalBlank, true); 
                if self.read_ctrl_flag(CtrlFlag::NmiEnabled) {
                    self.nmi_requested = true;
                }
            },
            _ => (),
        }

        #[allow(clippy::manual_range_contains)]
        if self.rendering_enabled() {
            if self.scan_line < 240 {
                self.render();
            }

            println!("tick {}, scan {}, scroll_x {}, scroll_y {}", self.tick, self.scan_line, self.get_vram_x(), self.get_vram_y());

            // manage scrolling
            match (self.scan_line, self.tick) {
                (_, t) if 1<= t && t < 256 && t % 8 == 0 => self.increment_course_x(),
                (_, 256) => {
                    self.increment_course_x();
                    self.increment_fine_y()
                }
                (_, 257) => self.copy_x(),
                (-1, t) if 280 <= t && t <= 304 => self.copy_y(),
                (_, 328) => self.increment_course_x(),
                (_, 336) => self.increment_course_x(),
                _ => (),
            }
        }

        // skip a tick on odd frames when rendering is enabled
        if self.scan_line == -1 && self.tick == 339 && !self.even_frame && self.rendering_enabled()
        {
            self.tick = 340;
        }
        self.tick += 1;
        if self.tick == 341 {
            self.tick = 0;
            self.scan_line += 1;
            if self.scan_line == 261 {
                self.scan_line = -1;
                self.even_frame = !self.even_frame;
            }
        }

        if self.nmi_requested && self.read_ctrl_flag(CtrlFlag::NmiEnabled) {
            self.nmi_requested = false;
            true
        } else {
            false
        }
    }

    fn rendering_enabled(&self) -> bool {
        self.ppu_mask & (MaskFlag::ShowBG | MaskFlag::ShowSprites) != 0
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn BusDevice>>) {
        self.bus.add_device(device);
    }

    pub fn reset(&mut self) {
        self.ppu_ctrl_high = 0;
        self.ppu_mask = 0;
        self.oam_addr = 0;
        self.oam_table = [0; 256];
        self.scan_line = -1;
        self.tick = 0;
        self.nmi_requested = false;
        self.even_frame = true;
        self.write_toggle = false;
        self.vram_address = 0;
        self.temporary_vram_address = 0;
        self.fine_x = 0;
    }

    fn render(&mut self) {
        let x = self.tick;
        let y = self.scan_line as u16;

     
        
        // shift registers
        /*
         7654 3210
         |||| ||++- 1-0: pallette number for top left quadrant of this meta tile
         |||| ++--- 3-2: pallette number for top right quadrant of this meta tile
         ||++------ 5-4: pallette number for bottom left quadrant of this meta tile
         ++-------- 7-6: pallette number for bottom right quadrant of this meta tile
        */
        let mut attribute_entry = 0;

        /*
         RRRR CCCC
         |||| ++++-------- tile column in pattern table
         ++++------------- tile row in pattern table
        */
        let mut nametable_entry = 0;

        /* low bits of pointers into pallet entry, 1 bit per pixel */
        let mut pattern_low_entry = 0;

        /* high bits of pointers into pallet entry, 1 bit per pixel */
        let mut pattern_high_entry = 0;

        // end of shift registers

        /*
         0010 NN YYYYY XXXXX
         |||| || ||||| +++++--- course X
         |||| || +++++--------- course Y
         |||| ++--------------- nametable select
         ++++------------------ 02
        */
        let nametable_address = 0x2000 | (self.vram_address & 0b0000111111111111);

        /*
         0010 NN 1111 YYY XXX
         |||| || |||| ||| +++-- X: high 3 bits of coarse X (x/4)
         |||| || |||| +++------ Y: high 3 bits of coarse Y (y/4)
         |||| || ++++---------- -: attribute offset within nametable (960 bytes)
         |||| ++--------------- N: nametable select
         ++++------------------ 0x2xxx
        */
        let attribute_address = 0x2000 | 
            (self.vram_address & 0b000011000000000) | // name table select
            0b0000001111000000 | // fixed attribute offset
            ((self.vram_address >> 4) & 0b0000000000111000) | // high 3 bits of course Y
            ((self.vram_address >> 2) & 0b0000000000000111); // high 3 bits of course X

        /*
         000 H RRRR CCCC P YYY
         ||| | |||| |||| | +++- Y: Fine Y offset, the row number within a tile
         ||| | |||| |||| +----- P: Bit plane (0: lower, 1: upper) (0 for reading bg low, 1 for reading bg high)
         ||| | |||| ++++------- C: Tile column (lower nibble of nametable_entry)
         ||| | ++++------------ R: Tile row (upper nibble of nametable_entry)
         ||| +----------------- H: Half of pattern table (0: left, 1: right) = CtrlFlag::BackgroundPatternHigh
         +++------------------- 0: Pattern table is 0x0000 - 0x01FFFF
         */
        let pattern_address: u16 = if self.read_ctrl_flag(CtrlFlag::BackgroundPatternHigh) {0x1000} else {0x0000} |
        (nametable_entry as u16) << 4 |
        ((self.vram_address & 0b0001110000000000) >> 13);


        if self.tick > 0 {
            match self.tick % 8 {
                0 if self.tick < 261 || self.tick > 320 => pattern_high_entry = self.read_ppu_bus(pattern_address | 0b00001000),
                1 if self.tick > 8 => (/* reload shifters */),
                2 => nametable_entry = self.read_ppu_bus(nametable_address),
                3 => (),
                4 if self.tick != 340  => attribute_entry = self.read_ppu_bus(attribute_address),
                4 if self.tick == 340  => nametable_entry = self.read_ppu_bus(nametable_address),
                5 => (),
                6 if self.tick < 261 || self.tick > 320  => pattern_low_entry = self.read_ppu_bus(pattern_address),
                7 => (),
                _ => (), 
            } 
        }

        if y < 240 {
            let meta_tile_row = y >> 6; // y / 64
            let meta_tile_column = x >> 6; // x / 64
            let  pallette_number = match (meta_tile_column & 0b1, meta_tile_row & 0b1) {
                (0,0) => attribute_entry & 0b11,
                (1,0) => (attribute_entry >> 2) & 0b11,
                (0,1) => (attribute_entry >> 4) & 0b11,
                (1,1) => (attribute_entry >> 6) & 0b11,
                (_,_) => unreachable!("got invalid pallette quadrant"),
            };

            let bit_rotation = 7 - self.fine_x as u32;
            let bit_selection = 1 << bit_rotation;
            // weird rotate right because sometimes we need to shift the high bit left 1
            let pixel_number = (pattern_high_entry & bit_selection).rotate_right(7 + bit_rotation) | ((pattern_low_entry & bit_selection) >> bit_rotation);
            

            /*
            00111111 xxx S PP CC
            |||||||| ||| | || ||
            |||||||| ||| | || ++- Color number from tile data
            |||||||| ||| | ++---- Palette number from attribute table or OAM
            |||||||| ||| +------- Background/Sprite select, 0=bg, 1=sprite
            |||||||| +++--------- doesn't matter, effectively set to 0 for mirroring
            ++++++++------------- 0x3F00 - 0x3FFF
            */
            let pallette_address = 0x3F00 | (pallette_number << 2) as u16 | pixel_number as u16;

            /*
            00 VV HHHH
            || || ||||
            || || ++++- Hue (phase, determines NTSC/PAL chroma)
            || ++------ Value (voltage, determines NTSC/PAL luma)
            ++--------- Unimplemented, reads back as 0
            */
            let color = self.read_ppu_bus(pallette_address);

            let rgb = rgb::translate_nes_to_rgb(color);
            let f = self.renderer;
            f(x, y, rgb);
        }
    }


    fn get_ppu_ctrl(&self) -> u8 {
        self.ppu_ctrl_high << 2 | (((self.temporary_vram_address & 0b0000110000000000) >> 10) as u8)
    }

    fn set_ppu_ctrl(&mut self, data: u8) -> u8 {
        let old = self.get_ppu_ctrl();
        self.ppu_ctrl_high = data >> 2;
        self.temporary_vram_address = (self.temporary_vram_address & !0b0000110000000000)
            | (((data & 0b00000011) as u16) << 10);
        self.nmi_requested = self.read_status_flag(StatusFlag::VerticalBlank);
        old
    }

    #[cfg(test)]
    fn set_ctrl_flag(&mut self, flag: CtrlFlag, value: bool) {
        if value {
            self.set_ppu_ctrl(self.get_ppu_ctrl() | flag);
        } else {
            self.set_ppu_ctrl(self.get_ppu_ctrl() & !flag);
        }
    }

    fn read_ctrl_flag(&mut self, flag: CtrlFlag) -> bool {
        (self.get_ppu_ctrl() & flag) != 0
    }

    #[cfg(test)]
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

    fn read_ppu_bus(&mut self, addr: u16) -> u8 {
        self.bus.read(addr)
    }

    fn write_ppu_bus(&mut self, addr: u16, data: u8) -> u8 {
        self.bus.write(addr, data)
    }

    fn inc_vram_addr(&mut self) {
        self.vram_address =
            self.vram_address
                .wrapping_add(if self.read_ctrl_flag(CtrlFlag::IncrementAcross) {
                    32
                } else {
                    1
                });
    }

    /**
     * Copy the scroll_x bits and horizontal nametable bit
     * from temp to current
     */
    fn copy_x(&mut self) {
        self.vram_address = (self.vram_address & !0b0000010000011111)
            | (self.temporary_vram_address & 0b0000010000011111);
    }

    /**
     * Copy the scroll_y bits and vertical nametable bit
     * from temp to current
     */
    fn copy_y(&mut self) {
        self.vram_address = (self.vram_address & !0b0111101111100000)
            | (self.temporary_vram_address & 0b0111101111100000);
    }

    fn get_scroll_x(&self) -> u8 {
        (((self.temporary_vram_address & 0b0000000000011111) as u8) << 3) | self.fine_x
    }

    fn set_scroll_x(&mut self, x: u8) -> u8 {
        let result = self.get_scroll_x();
        self.temporary_vram_address =
            (self.temporary_vram_address & !0b0000000000011111) | ((x >> 3) as u16);
        self.fine_x = x & 0b00000111;
        result
    }

    fn get_scroll_y(&self) -> u8 {
        (((self.temporary_vram_address & 0b0111000000000000) >> 12) as u8)
            | (((self.temporary_vram_address & 0b0000001111100000) >> 2) as u8)
    }

    fn set_scroll_y(&mut self, y: u8) -> u8 {
        let big_y = y as u16;
        let result = self.get_scroll_y();
        self.temporary_vram_address = (self.temporary_vram_address & !0b0111001111100000)
            | ((big_y & 0b111) << 12)
            | ((big_y & 0b11111000) << 2);
        result
    }

    fn get_vram_address_high(&self) -> u8 {
        ((self.temporary_vram_address & 0b0011111100000000) >> 8) as u8
    }

    fn set_vram_address_high(&mut self, data: u8) -> u8 {
        let result = self.get_vram_address_high();
        self.temporary_vram_address = (self.temporary_vram_address & !0b0011111100000000)
            | (((data & 0b00111111) as u16) << 8);
        result
    }

    fn get_vram_address_low(&self) -> u8 {
        (self.temporary_vram_address & 0b0000000011111111) as u8
    }

    fn set_vram_address_low(&mut self, data: u8) -> u8 {
        let result = self.get_vram_address_low();
        self.temporary_vram_address =
            (self.temporary_vram_address & !0b0000000011111111) | (data as u16);
        result
    }

    fn increment_course_x(&mut self) {
        // check to see if course x is maxed
        if (self.vram_address & 0b0000000000011111) == 0b000000000000011111 {
            self.vram_address &= !0b0000000000011111; // 0 the course X if maxed
            self.vram_address ^= 0b0000010000000000; // flip the horizontal nametable bit
        } else {
            self.vram_address = self.vram_address.wrapping_add(1);
        }
    }

    fn increment_fine_y(&mut self) {
        // check to see if fine y is maxed
        if (self.vram_address & 0b0111000000000000) == 0b0111000000000000 {
            self.vram_address &= !0b0111000000000000; // clear fine y if maxed
            let mut course_y = (self.vram_address & 0b00001111100000) >> 5;
            // check to see if course y is maxed in nametables (when reading attribute tables it can be bigger)
            if course_y == 29 {
                course_y = 0;
                self.vram_address ^= 0b0000100000000000; // flip the vertical nametable bit
            } else if course_y == 31 {
                // check to see if the course y is maxed in attribute tables
                course_y = 0;
            } else {
                course_y = course_y.wrapping_add(1);
            }
            // put the new course_y back in
            self.vram_address = (self.vram_address & !0b00001111100000) | (course_y << 5)
        } else {
            self.vram_address = self.vram_address.wrapping_add(0b0001000000000000);
            // not maxed, increment fine y
        }
    }

    fn get_vram_y(&self) -> u8 {
        (((self.vram_address & 0b0111000000000000) >> 12) as u8)
            | (((self.vram_address & 0b0000001111100000) >> 2) as u8)
    }

    fn get_vram_x(&self) -> u8 {
        (((self.vram_address & 0b0000000000011111) as u8) << 3) | self.fine_x
    }
}

impl BusDevice for PPU {
    fn read(&mut self, addr: u16) -> Option<u8> {
        if (CPU_ADDR_START..=CPU_ADDR_END).contains(&addr) {
            let result = match addr {
                0x2000 => self.ppu_data_buffer,
                0x2001 => self.ppu_data_buffer,
                0x2002 => {
                    self.write_toggle = false;
                    self.ppu_data_buffer = self.ppu_status | (self.ppu_data_buffer & 0x1F);
                    self.set_status_flag(StatusFlag::VerticalBlank, false);
                    self.ppu_data_buffer
                }
                0x2003 => self.ppu_data_buffer,
                0x2004 => self.oam_table[self.oam_addr as usize],
                0x2005 => self.ppu_data_buffer,
                0x2006 => self.ppu_data_buffer,
                0x2007 => {
                    if (PALETTE_START..PALETTE_END).contains(&self.vram_address) {
                        self.ppu_data_buffer = self.read_ppu_bus(self.vram_address & PALETTE_MASK);
                        self.inc_vram_addr();
                        self.ppu_data_buffer
                    } else {
                        let result = self.ppu_data_buffer;
                        self.ppu_data_buffer = self.read_ppu_bus(self.vram_address);
                        self.inc_vram_addr();
                        result
                    }
                }
                physical => unreachable!("reading from ppu register {}", physical),
            };
            Some(result)
        } else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> Option<u8> {
        if (CPU_ADDR_START..=CPU_ADDR_END).contains(&addr) {
            self.ppu_data_buffer = data;
            Some(match addr {
                0x2000 => {
                    let old = self.get_ppu_ctrl();

                    if self.read_status_flag(StatusFlag::VerticalBlank)
                    && (old & CtrlFlag::NmiEnabled != 0)
                    && (data & CtrlFlag::NmiEnabled == 0)
                    {
                        self.nmi_requested = true;
                    }
        

                    self.set_ppu_ctrl(data);



                    old
                }
                0x2001 => {
                    let old = self.ppu_mask;
                    self.ppu_mask = data;
                    old
                }
                0x2002 => 0,
                0x2003 => {
                    let old = self.oam_addr;
                    self.oam_addr = data;
                    old
                }
                0x2004 => {
                    let old = self.oam_table[self.oam_addr as usize];
                    if self.scan_line >= 240
                        || (!self.read_mask_flag(MaskFlag::ShowSprites)
                            && !self.read_mask_flag(MaskFlag::ShowBG))
                    {
                        self.oam_table[self.oam_addr as usize] = data;
                    }

                    self.oam_addr = self.oam_addr.wrapping_add(1);
                    old
                }
                0x2005 => {
                    if !self.write_toggle {
                        self.write_toggle = true;
                        self.set_scroll_x(data)
                    } else {
                        self.write_toggle = false;
                        self.set_scroll_y(data)
                    }
                }
                0x2006 => {
                    if !self.write_toggle {
                        self.write_toggle = true;
                        self.set_vram_address_high(data)
                    } else {
                        self.write_toggle = false;
                        let result = self.set_vram_address_low(data);
                        self.vram_address = self.temporary_vram_address;
                        result
                    }
                }
                0x2007 => {
                    if (PALETTE_START..PALETTE_END).contains(&self.vram_address) {
                        let result = self.write_ppu_bus(self.vram_address & PALETTE_MASK, data);
                        self.inc_vram_addr();
                        result
                    } else {
                        let result = self.write_ppu_bus(self.vram_address, data);
                        self.inc_vram_addr();
                        result
                    }
                }
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

#[cfg(test)]
pub fn create_test_configuration() -> (PPU, Rc<RefCell<crate::ram::RAM>>) {
    use crate::ram::RAM;

    let mut ppu = PPU::new(nul_renderer);
    let mem = Rc::new(RefCell::new(RAM::new(0x0000, 0xFFFF, 0xFFFF)));
    ppu.add_device(mem.clone());
    (ppu, mem)
}
