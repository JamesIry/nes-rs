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
    ctrl_high_register: u8,
    mask_register: u8,
    status_register: u8,
    oam_addr: u8,
    data_buffer: u8,
    last_read_buffer: u8,
    bus: Bus,
    oam_table: [u8; OAM_SIZE],
    pallettes: [u8; PALETTE_SIZE],
    scan_line: i16,
    tick: u16,

    nmi_requested: bool,
    even_frame: bool,

    write_toggle: bool,
    vram_address: VramAddress,
    temporary_vram_address: VramAddress,
}

pub fn nul_renderer(_x: u16, _y: u16, _rgb: (u8, u8, u8)) {}

impl PPU {
    pub fn new(renderer: fn(u16, u16, (u8, u8, u8)) -> ()) -> Self {
        Self {
            renderer,
            bus: Bus::new(),
            oam_table: [0; OAM_SIZE],
            ctrl_high_register: 0,
            mask_register: 0,
            status_register: 0,
            oam_addr: 0,
            data_buffer: 0,
            last_read_buffer: 0,
            pallettes: [0; PALETTE_SIZE],
            scan_line: -1,
            tick: 0,
            nmi_requested: false,
            even_frame: true,
            vram_address: VramAddress::new(),
            temporary_vram_address: VramAddress::new(),
            write_toggle: false,

        }
    }

    #[must_use]
    pub fn clock(&mut self) -> bool {
        // manage status
        match (self.scan_line, self.tick) {
            (-1, 1) => {
                self.status_register = 0; // clear StatusFlag::VerticalBlank, StatusFlag::Sprite0Hit, and StatusFlag::SpriteOverflow
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
        if self.rendering_enabled() && self.scan_line < 240 {
            self.render();
    

            println!("tick {}, scan {}, scroll_x {}, scroll_y {}", self.tick, self.scan_line, self.vram_address.get_x(), self.vram_address.get_y());

            // manage scrolling
            match (self.scan_line, self.tick) {
                (_, t) if 1<= t && t < 256 && t % 8 == 0 => self.vram_address.increment_coarse_x(),
                (_, 256) => {
                    self.vram_address.increment_coarse_x();
                    self.vram_address.increment_y()
                }
                (_, 257) => self.vram_address.copy_x_from(&self.temporary_vram_address),
                (-1, t) if 280 <= t && t <= 304 => self.vram_address.copy_y_from(&self.temporary_vram_address),
                (_, 328) => self.vram_address.increment_coarse_x(),
                (_, 336) => self.vram_address.increment_coarse_x(),
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

        if self.nmi_requested {
            self.nmi_requested = false;
            self.read_ctrl_flag(CtrlFlag::NmiEnabled)
        } else {
            false
        }
    }

    fn rendering_enabled(&self) -> bool {
        self.mask_register & (MaskFlag::ShowBG | MaskFlag::ShowSprites) != 0
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn BusDevice>>) {
        self.bus.add_device(device);
    }

    pub fn reset(&mut self) {
        self.ctrl_high_register = 0;
        self.mask_register = 0;
        self.oam_addr = 0;
        self.oam_table = [0; 256];
        self.scan_line = -1;
        self.tick = 0;
        self.nmi_requested = false;
        self.even_frame = true;
        self.write_toggle = false;
        self.vram_address = VramAddress::new();
        self.temporary_vram_address = VramAddress::new();
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

        let nametable_address = self.vram_address.get_nametable_address();

        let attribute_address = self.vram_address.get_attribute_address();

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
        ((nametable_entry as u16) << 4) |
        (self.vram_address.get_fine_y() as u16);


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

        if x < 256 && y < 240 {
            let meta_tile_row = y >> 6; // y / 64
            let meta_tile_column = x >> 6; // x / 64
            let  pallette_number = match (meta_tile_column & 0b1, meta_tile_row & 0b1) {
                (0,0) => attribute_entry,
                (1,0) => attribute_entry >> 2,
                (0,1) => attribute_entry >> 4,
                (1,1) => attribute_entry >> 6,
                (_,_) => unreachable!("got invalid pallette quadrant"),
            } & 0b11;

            let bit_rotation = 7 - self.temporary_vram_address.fine_x as u32;
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


    fn get_ctrl_flags(&self) -> u8 {
        self.ctrl_high_register | self.temporary_vram_address.get_nametable_bits()
    }

    fn set_ctrl_flags(&mut self, data: u8) -> u8 {
        let old = self.get_ctrl_flags();
        self.ctrl_high_register = data & 0b11111100;
        self.temporary_vram_address.set_nametable_bits(data);
        old
    }

    fn set_ctrl_flag(&mut self, flag: CtrlFlag, value: bool) {
        if value {
            self.set_ctrl_flags(self.get_ctrl_flags() | flag);
        } else {
            self.set_ctrl_flags(self.get_ctrl_flags() & !flag);
        }
    }

    fn read_ctrl_flag(&mut self, flag: CtrlFlag) -> bool {
        (self.get_ctrl_flags() & flag) != 0
    }

    fn set_mask_flag(&mut self, flag: MaskFlag, value: bool) {
        if value {
            self.mask_register |= flag;
        } else {
            self.mask_register &= !flag;
        }
    }

    fn read_mask_flag(&mut self, flag: MaskFlag) -> bool {
        (self.mask_register & flag) != 0
    }

    fn set_status_flag(&mut self, flag: StatusFlag, value: bool) {
        if value {
            self.status_register |= flag;
        } else {
            self.status_register &= !flag;
        }
    }

    fn read_status_flag(&mut self, flag: StatusFlag) -> bool {
        (self.status_register & flag) != 0
    }

    fn read_ppu_bus(&mut self, addr: u16) -> u8 {
        self.bus.read(addr)
    }

    fn write_ppu_bus(&mut self, addr: u16, data: u8) -> u8 {
        self.bus.write(addr, data)
    }

    fn inc_vram_addr(&mut self) {
        let amount = if self.read_ctrl_flag(CtrlFlag::IncrementAcross) {
            32
        } else {
            1
        };
        self.vram_address.inc_address(amount);
    }
}

impl BusDevice for PPU {
    fn read(&mut self, addr: u16) -> Option<u8> {
        if (CPU_ADDR_START..=CPU_ADDR_END).contains(&addr) {
            self.last_read_buffer = match addr {
                0x2000 => self.last_read_buffer,
                0x2001 => self.last_read_buffer,
                0x2002 => {
                    self.write_toggle = false;
                    self.data_buffer = self.status_register | (self.last_read_buffer & 0x1F);
                    self.set_status_flag(StatusFlag::VerticalBlank, false);
                    self.data_buffer
                }
                0x2003 => self.last_read_buffer,
                0x2004 => {self.data_buffer = self.oam_table[self.oam_addr as usize]; self.data_buffer},
                0x2005 => self.last_read_buffer,
                0x2006 => self.last_read_buffer,
                0x2007 => {
                    if (PALETTE_START..PALETTE_END).contains(&self.vram_address.register) {
                        self.data_buffer = self.read_ppu_bus(self.vram_address.register & PALETTE_MASK);
                        self.inc_vram_addr();
                        self.data_buffer
                    } else {
                        let result = self.data_buffer;
                        self.data_buffer = self.read_ppu_bus(self.vram_address.register);
                        self.inc_vram_addr();
                        result
                    }
                }
                physical => unreachable!("reading from ppu register {}", physical),
            };
            Some(self.last_read_buffer)
        } else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> Option<u8> {
        if (CPU_ADDR_START..=CPU_ADDR_END).contains(&addr) {
            self.data_buffer = data;
            self.last_read_buffer = data;
            Some(match addr {
                0x2000 => {
                    let old = self.get_ctrl_flags();

                    if self.read_status_flag(StatusFlag::VerticalBlank)
                    && (old & CtrlFlag::NmiEnabled != 0)
                    && (data & CtrlFlag::NmiEnabled == 0)
                    {
                        self.nmi_requested = true;
                    }
                    self.set_ctrl_flags(data);

                    old
                }
                0x2001 => {
                    let old = self.mask_register;
                    self.mask_register = data;
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
                        self.temporary_vram_address.set_x(data)
                    } else {
                        self.write_toggle = false;
                        self.temporary_vram_address.set_y(data)
                    }
                }
                0x2006 => {
                    if !self.write_toggle {
                        self.write_toggle = true;
                        self.temporary_vram_address.set_address_high(data)
                    } else {
                        self.write_toggle = false;
                        let result = self.temporary_vram_address.set_address_low(data);
                        self.vram_address.register = self.temporary_vram_address.register;
                        result
                    }
                }
                0x2007 => {
                    if (PALETTE_START..PALETTE_END).contains(&self.vram_address.register) {
                        let result = self.write_ppu_bus(self.vram_address.register & PALETTE_MASK, data);
                        self.inc_vram_addr();
                        result
                    } else {
                        let result = self.write_ppu_bus(self.vram_address.register, data);
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


struct VramAddress {
    /**
     * 0 yyy V H YYYYY XXXXX
     * | ||| | | ||||| +++++-- coarse X scroll
     * | ||| | | +++++-------- coarse Y scroll
     * | ||| | +-------------- horizontal nametable select
     * | ||| +---------------- vertical nametalbe select
     * | +++------------------ fine Y scroll
     * +---------------------- unused 0
     */
    register: u16,
    /**
     * Low 3 bits of x scroll
     */
    fine_x: u8,
}

impl VramAddress {
    fn new() -> Self {
        Self { register: 0, fine_x: 0 }
    }
    fn get_horizontal_nametable_selected(&self) -> bool {
        self.register & 0b0000010000000000 != 0
    }

    fn set_horizontal_nametable_selected(&mut self, value: bool) -> bool {
        let old = self.get_horizontal_nametable_selected();
        self.register = if value {self.register | 0b0000010000000000} else {self.register & !0b0000010000000000};
        old
    }

    fn get_vertical_nametable_selected(&self) -> bool {
        self.register & 0b0000100000000000 != 0
    }

    fn set_vertical_nametable_selected(&mut self, value: bool) -> bool {
        let old = self.get_vertical_nametable_selected();
        self.register = if value {self.register | 0b0000100000000000} else {self.register & !0b0000100000000000};
        old
    }

    fn get_x(&self) -> u8 {
        (self.get_coarse_x() << 3) | self.fine_x
    }

    fn set_x(&mut self, x: u8) -> u8 {
        let old = self.get_x();
        self.set_coarse_x(x >> 3);
        self.fine_x = x & 0b00000111;
        old
    }

    fn get_coarse_x(&self) -> u8 {
        (self.register & 0b0000000000011111) as u8
    }

    fn set_coarse_x(&mut self, x: u8) -> u8 {
        let result = self.get_coarse_x();
        self.register =
        (self.register & !0b0000000000011111) | ((x & 0b00011111) as u16);
        result
    }

    fn get_y(&self) -> u8 {
        (self.get_coarse_y() << 3) | self.get_fine_y()
    }

    fn set_y(&mut self, y: u8) -> u8 {
        let old = self.get_y();
        self.set_fine_y(y);
        self.set_coarse_y(y >> 3);
        old
    }

    fn get_coarse_y(&self) -> u8 {
        ((self.register & 0b0000001111100000) >> 5) as u8
    }

    fn set_coarse_y(&mut self, y: u8) -> u8 {
        let result = self.get_coarse_y();
        self.register = (self.register & !0b0000001111100000)
            | (((y & 0b00011111) as u16) << 5);
        result
    }

    fn get_fine_y(&self) -> u8 {
        ((self.register & 0b0111000000000000) >> 12) as u8
    }

    fn set_fine_y(&mut self, y: u8) {
        self.register = (self.register & !0b0111000000000000)
        | (((y & 0b00000111) as u16) << 12)
    }

    fn get_address_high(&self) -> u8 {
        ((self.register & 0b0011111100000000) >> 8) as u8
    }

    fn set_address_high(&mut self, data: u8) -> u8 {
        let result = self.get_address_high();
        self.register = (self.register & !0b0011111100000000)
            | (((data & 0b00111111) as u16) << 8);
        result
    }

    fn get_address_low(&self) -> u8 {
        (self.register & 0b0000000011111111) as u8
    }

    fn set_address_low(&mut self, data: u8) -> u8 {
        let result = self.get_address_low();
        self.register =
            (self.register & !0b0000000011111111) | (data as u16);
        result
    }

    fn increment_coarse_x(&mut self) {
        // check to see if coarse x is maxed
        let coarse_x = self.get_coarse_x();
        if self.get_coarse_x() == 31 {
            self.set_coarse_x(0);
            self.set_horizontal_nametable_selected(!self.get_horizontal_nametable_selected());
        } else {
            self.set_coarse_x(coarse_x.wrapping_add(1));
        }
    }

    fn increment_y(&mut self) {
        let y = self.get_y();
        // check to see if y is maxed
        // y can "overflow" past 239 when reading attribute tables, so check
        // for both 239 and 255
        if y == 239 || y == 255 {
            self.set_y(0);
            self.set_vertical_nametable_selected(!self.get_vertical_nametable_selected());
        } else {
            self.set_y(y.wrapping_add(1));
        }
    }

    fn copy_x_from(&mut self, other: &VramAddress) {
        self.set_x(other.get_x());
        self.set_horizontal_nametable_selected(other.get_horizontal_nametable_selected());
    }

    fn copy_y_from(&mut self, other: &VramAddress) {
        self.set_y(other.get_y());
        self.set_vertical_nametable_selected(other.get_vertical_nametable_selected());
    }

    fn get_nametable_bits(&self) -> u8 {
        ((self.register & 0b0000110000000000) >> 10) as u8
    }

    fn set_nametable_bits(&mut self, bits: u8) {
        self.register = (self.register & !0b0000110000000000) | (((bits & 0b00000011) as u16) << 10);
    }

    fn inc_address(&mut self, ammount: u16) {
        self.register = self.register.wrapping_add(ammount);
    }

    fn get_nametable_address(&self) -> u16 {
        /*
         0010 NN YYYYY XXXXX
         |||| || ||||| +++++--- coarse X
         |||| || +++++--------- coarse Y
         |||| ++--------------- nametable select
         ++++------------------ 02
        */
        0x2000 | (self.register & 0b0000111111111111)
    }

    fn get_attribute_address(&self) -> u16 {
            /*
         0010 NN 1111 YYY XXX
         |||| || |||| ||| +++-- X: high 3 bits of coarse X (x/4)
         |||| || |||| +++------ Y: high 3 bits of coarse Y (y/4)
         |||| || ++++---------- -: fixed attribute offset within nametable (960 bytes)
         |||| ++--------------- N: nametable select
         ++++------------------ 0x2xxx
        */
        0x2000 | 
            (self.register & 0b000011000000000) | // name table select
            0b0000001111000000 | // fixed attribute offset
            ((self.register >> 4) & 0b0000000000111000) | // high 3 bits of coarse Y
            ((self.register >> 2) & 0b0000000000000111) // high 3 bits of coarse X
    }
}