mod flags;
mod rgb;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod unit_tests;

use std::{cell::RefCell, rc::Rc};

use crate::bus::{Bus, BusDevice};

use self::{
    flags::{CtrlFlag, MaskFlag, StatusFlag},
    rgb::translate_nes_to_rgb,
};

const CPU_ADDR_START: u16 = 0x2000;
const CPU_ADDR_END: u16 = 0x3FFF;
const CPU_ADDR_MASK: u16 = 0x2007;
const PALETTE_START: u16 = 0x3000;
const PALETTE_END: u16 = 0xFFFF;
const PALETTE_SIZE: usize = 0x0020;
const PALETTE_MASK: u16 = 0x001F;
const PRIMARY_OAM_SIZE: usize = 0x0100;
const SECONDARY_OAM_SIZE: usize = 0x0020;

// blargg's power on palette values. why not?
const INITIAL_PALETTE_VALUES: [u8; PALETTE_SIZE] = [
    0x09, 0x01, 0x00, 0x01, 0x00, 0x02, 0x02, 0x0D, 0x08, 0x10, 0x08, 0x24, 0x00, 0x00, 0x04, 0x2C,
    0x09, 0x01, 0x34, 0x03, 0x00, 0x04, 0x00, 0x14, 0x08, 0x3A, 0x00, 0x02, 0x00, 0x20, 0x2C, 0x08,
];

/**
 * The main source for building this out was https://www.nesdev.org/wiki/PPU
 */
#[allow(clippy::upper_case_acronyms)]
pub struct PPU {
    renderer: Box<dyn FnMut(u16, u16, u8, u8, u8)>,
    ctrl_high_register: u8,
    mask_register: u8,
    status_register: u8,

    bus: Bus,

    primary_oam: OAMData<PRIMARY_OAM_SIZE>,
    secondary_oam: OAMData<SECONDARY_OAM_SIZE>,

    palettes: [u8; PALETTE_SIZE],
    scan_line: i16,
    tick: u16,

    nmi_requested: bool,
    even_frame: bool,

    write_toggle: bool,
    vram_address: VramAddress,
    temporary_vram_address: VramAddress,

    bg_shift_registers: BGShiftRegisterSet,
    oam_buffer: u8,
    sprite_eval_state: SpriteEvalState,

    bus_request: BusRequest,
    data_buffer: u8,

    resetting: bool,
}

impl PPU {
    #[cfg(test)]
    pub fn nul_renderer() -> Box<dyn FnMut(u16, u16, u8, u8, u8)> {
        Box::new(|_x: u16, _y: u16, _r: u8, _g: u8, _b: u8| ())
    }

    pub fn new(renderer: Box<dyn FnMut(u16, u16, u8, u8, u8)>) -> Self {
        Self {
            resetting: true,
            renderer,
            bus: Bus::new(),
            primary_oam: OAMData::new(),
            secondary_oam: OAMData::new(),
            ctrl_high_register: 0,
            mask_register: 0,
            status_register: StatusFlag::VerticalBlank | StatusFlag::SpriteOverflow,
            palettes: INITIAL_PALETTE_VALUES,
            scan_line: -1,
            tick: 0,
            nmi_requested: false,
            even_frame: true,
            vram_address: VramAddress::new(),
            temporary_vram_address: VramAddress::new(),
            write_toggle: false,

            bg_shift_registers: BGShiftRegisterSet::new(),
            oam_buffer: 0,
            sprite_eval_state: SpriteEvalState::ReadY,

            data_buffer: 0,
            bus_request: BusRequest::None,
        }
    }

    #[must_use]
    pub fn clock(&mut self) -> bool {
        self.manage_bus_request();
        self.manage_status();
        if self.rendering_enabled() && self.scan_line < 240 {
            self.manage_sprite_evaluation();
            self.manage_shift_registers();
            self.manage_render();
            self.manage_scrolling();
        }
        self.manage_tick();
        self.manage_nmi()
    }

    fn rendering_enabled(&self) -> bool {
        self.mask_register & (MaskFlag::ShowBG | MaskFlag::ShowSprites) != 0
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn BusDevice>>) {
        self.bus.add_device(device);
    }

    pub fn reset(&mut self) {
        self.resetting = true;
        self.even_frame = true;
        self.ctrl_high_register = 0;
        self.mask_register = 0;
        self.scan_line = -1;
        self.tick = 0;
        self.nmi_requested = false;
        self.even_frame = true;
        self.write_toggle = false;
        self.vram_address = VramAddress::new();
        self.bg_shift_registers = BGShiftRegisterSet::new();
        self.bus_request = BusRequest::None;

        // ** unchanged by reset
        // ppu_status
        // oam_data
        // vram_address (but temp is cleared)
        // palette_table
    }

    fn manage_bus_request(&mut self) {
        match self.bus_request {
            BusRequest::Read(addr) => {
                self.data_buffer = self.bus.read(addr);
                self.bus_request = BusRequest::None;
            }
            BusRequest::Write(addr, data) => {
                self.bus.write(addr, data);
                self.bus_request = BusRequest::None;
            }
            BusRequest::None => (),
        }
    }

    fn manage_status(&mut self) {
        // manage status
        match (self.scan_line, self.tick) {
            (-1, 1) => {
                self.status_register = 0; // clear StatusFlag::VerticalBlank, StatusFlag::Sprite0Hit, and StatusFlag::SpriteOverflow
                self.resetting = false; //
            }
            (241, 1) => {
                self.set_status_flag(StatusFlag::VerticalBlank, true);
                if self.read_ctrl_flag(CtrlFlag::NmiEnabled) {
                    self.nmi_requested = true;
                }
            }
            _ => (),
        }
    }

    #[allow(clippy::manual_range_contains)]
    fn manage_scrolling(&mut self) {
        /*println!(
            "tick {}, scan {}, scroll_x {}, scroll_y {}",
            self.tick,
            self.scan_line,
            self.vram_address.get_x(),
            self.vram_address.get_y()
        );*/

        // manage scrolling
        match (self.scan_line, self.tick) {
            (_, t) if 1 <= t && t < 256 && t % 8 == 0 => self.vram_address.increment_coarse_x(),
            (_, 256) => {
                self.vram_address.increment_coarse_x();
                self.vram_address.increment_y()
            }
            (_, 257) => self.vram_address.copy_x_from(&self.temporary_vram_address),
            (-1, t) if 280 <= t && t <= 304 => {
                self.vram_address.copy_y_from(&self.temporary_vram_address)
            }
            (_, 328) => self.vram_address.increment_coarse_x(),
            (_, 336) => self.vram_address.increment_coarse_x(),
            _ => (),
        }
    }

    #[allow(clippy::manual_range_contains)]
    fn manage_sprite_evaluation(&mut self) {
        if self.scan_line < 0 {
            return;
        }

        match self.tick {
            0 => {
                self.primary_oam.load_addr(0);
                self.primary_oam.read_enabled = false;
                self.primary_oam.write_enabled = false;

                self.secondary_oam.load_addr(0);
                self.secondary_oam.write_enabled = true;
            }
            t if 1 <= t && t <= 64 => {
                match t % 2 {
                    1 => {
                        self.oam_buffer = self.primary_oam.read_data();
                        self.primary_oam.inc_addr();
                    }
                    0 => {
                        self.secondary_oam.write_data(self.oam_buffer);
                        self.secondary_oam.inc_addr();
                    }
                    _ => unreachable!("tick was neither even nor odd"),
                };
            }
            t if 65 <= t && t <= 256 => {
                if t == 65 {
                    self.primary_oam.read_enabled = true;
                    self.sprite_eval_state = SpriteEvalState::ReadY;
                }

                // 192 cycles to work with
                // the minimum is 64 reads then writes of y, so 128 cycles.
                // At most 9 of them will match (8 "real" and 1 "overflow")
                // and require 6 additional cycles each, so 54 additional cycles.
                // Total is 182.
                // There are 10 cycles I can't account for. Just "extras?"
                match self.sprite_eval_state {
                    SpriteEvalState::ReadY => {
                        self.oam_buffer = self.primary_oam.read_data();
                        self.primary_oam.inc_addr();
                        self.sprite_eval_state = SpriteEvalState::WriteCompareY;
                    }
                    SpriteEvalState::WriteCompareY => {
                        let y = self.oam_buffer as i16;
                        self.secondary_oam.write_data(self.oam_buffer);
                        self.secondary_oam.inc_addr();
                        let sprite_height = if self.read_ctrl_flag(CtrlFlag::SpriteSizeLarge) {
                            16
                        } else {
                            8
                        };

                        if y <= self.scan_line
                            && self.scan_line < y + sprite_height
                            && !self.read_status_flag(StatusFlag::SpriteOverflow)
                        {
                            if !self.secondary_oam.write_enabled {
                                self.set_status_flag(StatusFlag::SpriteOverflow, true);
                            }
                            self.sprite_eval_state = SpriteEvalState::ReadTileIndex;
                        } else {
                            // skip over the tile, attribute, and x data since y didn't match
                            self.primary_oam.inc_addr();
                            self.primary_oam.inc_addr();
                            self.primary_oam.inc_addr();

                            // this "extra increment" is what causes the famous sprite
                            // overflow bug. Triggered when write is disabled, i.e we've
                            // found 8 renderable sprites but, to get here, haven't found
                            // an overflow sprite yet
                            if !self.secondary_oam.write_enabled {
                                self.primary_oam.inc_addr();
                            }
                            self.sprite_eval_state = SpriteEvalState::ReadY;
                        }
                    }
                    SpriteEvalState::ReadTileIndex => {
                        self.oam_buffer = self.primary_oam.read_data();
                        self.primary_oam.inc_addr();
                        self.sprite_eval_state = SpriteEvalState::WriteTileIndex;
                    }
                    SpriteEvalState::WriteTileIndex => {
                        self.secondary_oam.write_data(self.oam_buffer);
                        self.secondary_oam.inc_addr();
                        self.sprite_eval_state = SpriteEvalState::ReadAttributes;
                    }
                    SpriteEvalState::ReadAttributes => {
                        self.oam_buffer = self.primary_oam.read_data();
                        self.primary_oam.inc_addr();
                        self.sprite_eval_state = SpriteEvalState::WriteTileAttributes;
                    }
                    SpriteEvalState::WriteTileAttributes => {
                        self.secondary_oam.write_data(self.oam_buffer);
                        self.secondary_oam.inc_addr();
                        self.sprite_eval_state = SpriteEvalState::ReadX;
                    }
                    SpriteEvalState::ReadX => {
                        self.oam_buffer = self.primary_oam.read_data();
                        self.primary_oam.inc_addr();
                        self.sprite_eval_state = SpriteEvalState::WriteX;
                    }
                    SpriteEvalState::WriteX => {
                        self.secondary_oam.write_data(self.oam_buffer);
                        if self.secondary_oam.is_full() {
                            self.secondary_oam.write_enabled = false;
                        }
                        self.secondary_oam.inc_addr();
                        self.sprite_eval_state = SpriteEvalState::ReadY;
                    }
                }
            }
            257 => {
                self.primary_oam.load_addr(0);
                self.primary_oam.write_enabled = true;
                self.secondary_oam.load_addr(0);
            }
            t if 258 <= t && t <= 340 => {}
            _ => unreachable!("Unreachable tick number {}", self.tick),
        }
    }

    fn manage_shift_registers(&mut self) {
        let name_table_address = self.vram_address.get_nametable_address();
        let attribute_address = self.vram_address.get_attribute_address();
        let attribute_shift = self.vram_address.get_attribute_shift();
        let pattern_address = self.bg_shift_registers.get_pattern_address(
            self.read_ctrl_flag(CtrlFlag::BackgroundPatternHigh),
            self.vram_address.get_fine_y(),
        );

        if self.tick > 0 {
            match self.tick % 8 {
                1 => {
                    self.bus_request = BusRequest::Read(name_table_address);
                }
                2 => self
                    .bg_shift_registers
                    .load_name_table_data(self.data_buffer),

                3 if self.tick != 339 => self.bus_request = BusRequest::Read(attribute_address),
                4 if self.tick != 340 => self
                    .bg_shift_registers
                    .load_attribute_data(self.data_buffer, attribute_shift),

                3 if self.tick == 339 => self.bus_request = BusRequest::Read(name_table_address),
                4 if self.tick == 340 => self
                    .bg_shift_registers
                    .load_name_table_data(self.data_buffer),

                5 if self.tick < 261 || self.tick > 320 => {
                    self.bus_request = BusRequest::Read(pattern_address)
                }
                6 if self.tick < 261 || self.tick > 320 => self
                    .bg_shift_registers
                    .load_pattern_data_low(self.data_buffer),

                7 if self.tick < 261 || self.tick > 320 => {
                    self.bus_request = BusRequest::Read(pattern_address | 0b00001000)
                }
                0 if (1 <= self.tick && self.tick < 261) || self.tick > 320 => {
                    self.bg_shift_registers
                        .load_pattern_data_high(self.data_buffer);

                    self.bg_shift_registers.shift();
                }

                _ => (),
            }
        }
    }

    fn manage_render(&mut self) {
        let x = self.tick;
        let y = self.scan_line as u16;

        if x < 256 && y < 240 {
            let bg_color = if self.read_mask_flag(MaskFlag::ShowBG)
                && (x >= 8 || self.read_mask_flag(MaskFlag::ShowLeft8BG))
            {
                let palette_address = self.bg_shift_registers.get_palette_address(
                    false,
                    x,
                    self.temporary_vram_address.fine_x,
                );

                /*
                00 VV HHHH
                || || ||||
                || || ++++- Hue (phase, determines NTSC/PAL chroma)
                || ++------ Value (voltage, determines NTSC/PAL luma)
                ++--------- Unimplemented, reads back as 0
                */
                self.read_palette(palette_address)
            } else {
                0
            };

            // TODO composite with sprite data

            const SHOW_GRID: bool = false;

            let (r, g, b) = if SHOW_GRID && ((x % 32 == 0) || (y % 32 == 0)) {
                (255, 0, 0)
            } else if SHOW_GRID && ((x % 16 == 0) || (y % 16 == 0)) {
                (0, 255, 0)
            } else if SHOW_GRID && ((x % 8 == 0) || (y % 8 == 0)) {
                (0, 0, 255)
            } else {
                translate_nes_to_rgb(bg_color)
            };
            let f = &mut self.renderer;
            f(x, y, r, g, b);
        }
    }

    fn manage_tick(&mut self) {
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
    }

    fn read_palette(&self, addr: u16) -> u8 {
        let mirrored = addr & PALETTE_MASK;

        // 10/14/18/1C are mapped to 00/04/08/0C
        let physical = if mirrored & 0b11110011 == 0b00010000 {
            mirrored & 0b00001100
        } else {
            mirrored
        };

        let data = self.palettes[physical as usize];
        // greyscale mode asks off the low bits
        if self.read_mask_flag(MaskFlag::Greyscale) {
            data & 0b00110000
        } else {
            data & 0b00111111
        }
    }

    fn write_palette(&mut self, addr: u16, data: u8) -> u8 {
        let mirrored = addr & PALETTE_MASK;

        // 10/14/18/1C are mapped to 00/04/08/0C
        let physical = if mirrored & 0b00010011 == 0b00010000 {
            mirrored & 0b00001100
        } else {
            mirrored
        };
        let old = self.palettes[physical as usize];
        self.palettes[physical as usize] = data & 0b00111111;
        old
    }

    #[must_use]
    fn manage_nmi(&mut self) -> bool {
        if self.nmi_requested {
            self.nmi_requested = false;
            self.read_ctrl_flag(CtrlFlag::NmiEnabled)
        } else {
            false
        }
    }

    fn get_ctrl_flags(&self) -> u8 {
        self.ctrl_high_register | self.temporary_vram_address.get_nametable_bits()
    }

    fn set_ctrl_flags(&mut self, data: u8) -> u8 {
        let old = self.get_ctrl_flags();
        if !self.resetting {
            self.ctrl_high_register = data & 0b11111100;
            self.temporary_vram_address.set_nametable_bits(data);
        }
        old
    }

    #[cfg(test)]
    fn set_ctrl_flag(&mut self, flag: CtrlFlag, value: bool) {
        if value {
            self.set_ctrl_flags(self.get_ctrl_flags() | flag);
        } else {
            self.set_ctrl_flags(self.get_ctrl_flags() & !flag);
        }
    }

    fn read_ctrl_flag(&self, flag: CtrlFlag) -> bool {
        (self.get_ctrl_flags() & flag) != 0
    }

    #[cfg(test)]
    fn set_mask_flag(&mut self, flag: MaskFlag, value: bool) {
        if value {
            self.mask_register |= flag;
        } else {
            self.mask_register &= !flag;
        }
    }

    fn read_mask_flag(&self, flag: MaskFlag) -> bool {
        (self.mask_register & flag) != 0
    }

    fn set_status_flag(&mut self, flag: StatusFlag, value: bool) {
        if value {
            self.status_register |= flag;
        } else {
            self.status_register &= !flag;
        }
    }

    fn read_status_flag(&self, flag: StatusFlag) -> bool {
        (self.status_register & flag) != 0
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
            Some(match addr & CPU_ADDR_MASK {
                0x2000 => self.data_buffer,
                0x2001 => self.data_buffer,
                0x2002 => {
                    self.write_toggle = false;
                    let result = self.status_register | (self.data_buffer & 0x1F);
                    self.set_status_flag(StatusFlag::VerticalBlank, false);
                    result
                }
                0x2003 => self.data_buffer,
                0x2004 => self.primary_oam.read_data(),
                0x2005 => self.data_buffer,
                0x2006 => self.data_buffer,
                0x2007 => {
                    let addr = self.vram_address.register;
                    let result = if (PALETTE_START..PALETTE_END).contains(&addr) {
                        self.read_palette(addr)
                    } else {
                        self.data_buffer
                    };
                    // vram is read even when in palette address range
                    self.bus_request = BusRequest::Read(addr);

                    self.inc_vram_addr();
                    result
                }
                physical => unreachable!("reading from ppu register {}", physical),
            })
        } else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> Option<u8> {
        if (CPU_ADDR_START..=CPU_ADDR_END).contains(&addr) {
            Some(match addr & CPU_ADDR_MASK {
                0x2000 => {
                    let old = self.get_ctrl_flags();

                    if self.read_status_flag(StatusFlag::VerticalBlank)
                        && (old & CtrlFlag::NmiEnabled == 0)
                        && (data & CtrlFlag::NmiEnabled != 0)
                    {
                        self.nmi_requested = true;
                    }
                    self.set_ctrl_flags(data);

                    old
                }
                0x2001 => {
                    let old = self.mask_register;
                    if !self.resetting {
                        self.mask_register = data;
                    }
                    old
                }
                0x2002 => 0,
                0x2003 => self.primary_oam.load_addr(data),
                0x2004 => {
                    let old = self.primary_oam.write_data(data);
                    self.primary_oam.inc_addr();
                    old
                }
                0x2005 => {
                    if !self.resetting {
                        if !self.write_toggle {
                            self.write_toggle = true;
                            self.temporary_vram_address.set_x(data)
                        } else {
                            self.write_toggle = false;
                            self.temporary_vram_address.set_y(data)
                        }
                    } else {
                        0
                    }
                }
                0x2006 => {
                    if !self.resetting {
                        if !self.write_toggle {
                            self.write_toggle = true;
                            self.temporary_vram_address.set_address_high(data)
                        } else {
                            self.write_toggle = false;
                            let result = self.temporary_vram_address.set_address_low(data);
                            self.vram_address.register = self.temporary_vram_address.register;
                            result
                        }
                    } else {
                        0
                    }
                }
                0x2007 => {
                    let addr = self.vram_address.register;
                    let result = if (PALETTE_START..PALETTE_END).contains(&addr) {
                        self.write_palette(addr, data)
                    } else {
                        let old = self.data_buffer;
                        self.bus_request = BusRequest::Write(addr, data);
                        old
                    };
                    self.inc_vram_addr();
                    result
                }
                physical => unreachable!("writing to ppu register {}", physical),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
pub fn create_test_configuration() -> (PPU, Rc<RefCell<crate::ram::RAM>>) {
    use crate::ram::RAM;

    let mut ppu = PPU::new(PPU::nul_renderer());
    ppu.resetting = false;
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
        Self {
            register: 0,
            fine_x: 0,
        }
    }
    fn get_horizontal_nametable_selected(&self) -> bool {
        self.register & 0b0000010000000000 != 0
    }

    fn set_horizontal_nametable_selected(&mut self, value: bool) -> bool {
        let old = self.get_horizontal_nametable_selected();
        self.register = if value {
            self.register | 0b0000010000000000
        } else {
            self.register & !0b0000010000000000
        };
        old
    }

    fn get_vertical_nametable_selected(&self) -> bool {
        self.register & 0b0000100000000000 != 0
    }

    fn set_vertical_nametable_selected(&mut self, value: bool) -> bool {
        let old = self.get_vertical_nametable_selected();
        self.register = if value {
            self.register | 0b0000100000000000
        } else {
            self.register & !0b0000100000000000
        };
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
        self.register = (self.register & !0b0000000000011111) | ((x & 0b00011111) as u16);
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
        self.register = (self.register & !0b0000001111100000) | (((y & 0b00011111) as u16) << 5);
        result
    }

    fn get_fine_y(&self) -> u8 {
        ((self.register & 0b0111000000000000) >> 12) as u8
    }

    fn set_fine_y(&mut self, y: u8) {
        self.register = (self.register & !0b0111000000000000) | (((y & 0b00000111) as u16) << 12)
    }

    fn get_address_high(&self) -> u8 {
        ((self.register & 0b0011111100000000) >> 8) as u8
    }

    fn set_address_high(&mut self, data: u8) -> u8 {
        let result = self.get_address_high();
        self.register = (self.register & !0b0011111100000000) | (((data & 0b00111111) as u16) << 8);
        result
    }

    fn get_address_low(&self) -> u8 {
        (self.register & 0b0000000011111111) as u8
    }

    fn set_address_low(&mut self, data: u8) -> u8 {
        let result = self.get_address_low();
        self.register = (self.register & !0b0000000011111111) | (data as u16);
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
        self.register =
            (self.register & !0b0000110000000000) | (((bits & 0b00000011) as u16) << 10);
    }

    fn inc_address(&mut self, ammount: u16) {
        self.register = self.register.wrapping_add(ammount);
    }

    fn get_nametable_address(&self) -> u16 {
        /*
         Basically mask out the fine y bits and bob's your uncle

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
         Mask out the Fine Y bits, squish coarse X
         and coarse Y down to their top 3 bits,
         and put 1111 in the holes left

         0010 NN 1111 YYY XXX
         |||| || |||| ||| +++-- X: high 3 bits of coarse X (x/4)
         |||| || |||| +++------ Y: high 3 bits of coarse Y (y/4)
         |||| || ++++---------- -: fixed attribute offset within nametable (960 bytes)
         |||| ++--------------- N: nametable select
         ++++------------------ 0x2xxx
        */
        0x2000 |
            (self.register & 0b0000110000000000) | // name table select
            0b0000001111000000 | // fixed attribute offset
            ((self.register >> 4) & 0b0000000000111000) | // high 3 bits of coarse Y
            ((self.register >> 2) & 0b0000000000000111) // high 3 bits of coarse X
    }

    /**
     * Attribute data is in "meta tiles", which are 4x4 arrangements of 8x8 tiles,
     * i.e. 32x32 pixels. Metatiles are divided into 4 16x16 quadrants. Upper left is 0,
     * upper right is 1, lower left is 2, and lower right is 3.
     * Upper left needs no shifting, upper right needs 2, lower left needs 4,
     * and lower right needs 6.
     */
    fn compute_attribute_shift(x: u8, y: u8) -> u8 {
        ((y >> 2) & 0b100) | ((x >> 3) & 0b010)
    }

    fn get_attribute_shift(&self) -> u8 {
        VramAddress::compute_attribute_shift(self.get_x(), self.get_y())
    }
}

struct BGShiftRegister {
    prefetch: u8,
    data: u16,
}

impl BGShiftRegister {
    fn new() -> Self {
        Self {
            prefetch: 0,
            data: 0,
        }
    }

    fn load(&mut self, data: u8) {
        self.prefetch = data;
    }

    fn shift(&mut self) {
        self.data = (self.data << 8) | (self.prefetch as u16)
    }

    fn bit(&self, n: u8) -> u16 {
        if self.data & (1 << (15 - n)) == 0 {
            0
        } else {
            1
        }
    }
}

struct BGShiftRegisterPair {
    high: BGShiftRegister,
    low: BGShiftRegister,
}

impl BGShiftRegisterPair {
    fn new() -> Self {
        Self {
            high: BGShiftRegister::new(),
            low: BGShiftRegister::new(),
        }
    }
    fn load_high(&mut self, data: u8) {
        self.high.load(data);
    }

    fn load_low(&mut self, data: u8) {
        self.low.load(data);
    }

    fn shift(&mut self) {
        self.high.shift();
        self.low.shift();
    }

    fn bits(&self, x: u16, fine_x: u8) -> u16 {
        let x_offset = fine_x.wrapping_add((x & 0b111) as u8);
        self.high.bit(x_offset) << 1 | self.low.bit(x_offset)
    }
}

struct BGShiftRegisterSet {
    /**
     * High and low bits for 2 bit pairs of color indices for each tile
     */
    pattern_data: BGShiftRegisterPair,

    /**
     * High and low bits for 2 bit pairs of pallet number for each tile
     */
    attribute_data: BGShiftRegisterPair,

    /*
     RRRR CCCC
     |||| ++++-------- tile column in pattern table
     ++++------------- tile row in pattern table
    */
    name_table_data: u8,
}

impl BGShiftRegisterSet {
    fn new() -> Self {
        Self {
            pattern_data: BGShiftRegisterPair::new(),
            attribute_data: BGShiftRegisterPair::new(),
            name_table_data: 0,
        }
    }

    fn shift(&mut self) {
        self.pattern_data.shift();
        self.attribute_data.shift();
    }

    fn load_pattern_data_high(&mut self, data: u8) {
        self.pattern_data.load_high(data);
    }

    fn load_pattern_data_low(&mut self, data: u8) {
        self.pattern_data.load_low(data);
    }

    fn load_name_table_data(&mut self, data: u8) {
        self.name_table_data = data;
    }

    fn load_attribute_data(&mut self, data: u8, attribute_data_shift: u8) {
        let bits = (data >> attribute_data_shift) & 0b11;
        self.attribute_data
            .load_high(if bits & 0b10 != 0 { 0xFF } else { 0 });
        self.attribute_data
            .load_low(if bits & 0b01 != 0 { 0xFF } else { 0 });
    }

    fn get_pattern_address(&self, background_high: bool, fine_y: u8) -> u16 {
        /*
        000 H RRRR CCCC P YYY
        ||| | |||| |||| | +++- Y: Fine Y offset, the row number within a tile
        ||| | |||| |||| +----- P: Bit plane (0: lower, 1: upper) (0 for reading bg low, 1 for reading bg high)
        ||| | |||| ++++------- C: Tile column (lower nibble of nametable_entry)
        ||| | ++++------------ R: Tile row (upper nibble of nametable_entry)
        ||| +----------------- H: Half of pattern table (0: left, 1: right) = CtrlFlag::BackgroundPatternHigh
        +++------------------- 0: Pattern table is 0x0000 - 0x01FFF
        */
        (if background_high { 0x1000 } else { 0x0000 })
            | ((self.name_table_data as u16) << 4)
            | ((fine_y & 0b00000111) as u16)
        // the "bit plane" is set to 1 by the pattern data fetching code as needed
    }

    fn get_pixel_color_number(&self, x: u16, fine_x: u8) -> u16 {
        self.pattern_data.bits(x, fine_x)
    }

    fn get_pallete_number(&self, x: u16, fine_x: u8) -> u16 {
        self.attribute_data.bits(x, fine_x)
    }

    /**
     * 00111111 xxx S PP CC
     * |||||||| ||| | || ||
     * |||||||| ||| | || ++- Color number from tile data
     * |||||||| ||| | ++---- Palette number from attribute table or OAM
     * |||||||| ||| +------- Background/Sprite select, 0=bg, 1=sprite
     * |||||||| +++--------- doesn't matter, effectively set to 0 by mirroring
     * ++++++++------------- 0x3F00 - 0x3FFF
     */
    fn get_palette_address(&self, sprite: bool, x: u16, fine_x: u8) -> u16 {
        let pixel_color_number = self.get_pixel_color_number(x, fine_x);

        let palette_number = if pixel_color_number == 0 {
            0
        } else {
            self.get_pallete_number(x, fine_x)
        };

        0x3F00 | if sprite { 0b10000 } else { 0 } | (palette_number << 2) | pixel_color_number
    }
}

/**
 * The PPU takes 2 cycles to read/write its bus, except for
 * palletes. So this enum represent a bus action to perform on
 * the next cycle
 */
enum BusRequest {
    Read(u16),
    Write(u16, u8),
    None,
}

struct OAMData<const SIZE: usize> {
    addr: u8,
    table: [u8; SIZE],
    read_enabled: bool,
    write_enabled: bool,
    addr_mask: u8,
}

impl<const SIZE: usize> OAMData<SIZE> {
    fn new() -> Self {
        Self {
            addr: 0,
            table: [0; SIZE],
            read_enabled: true,
            write_enabled: true,
            addr_mask: (SIZE - 1) as u8,
        }
    }

    fn load_addr(&mut self, addr: u8) -> u8 {
        let old = self.addr;
        self.addr = addr & self.addr_mask;
        old
    }

    fn inc_addr(&mut self) {
        self.addr = self.addr.wrapping_add(1) & self.addr_mask;
    }

    fn is_full(&self) -> bool {
        (self.addr as usize) + 1 == SIZE
    }

    fn write_data(&mut self, data: u8) -> u8 {
        let old = self.table[self.addr as usize];
        if self.write_enabled {
            self.table[self.addr as usize] = data;
        }
        old
    }

    fn read_data(&self) -> u8 {
        if self.read_enabled {
            self.table[self.addr as usize]
        } else {
            0xFF
        }
    }
}

enum SpriteEvalState {
    ReadY,
    WriteCompareY,
    ReadTileIndex,
    WriteTileIndex,
    ReadAttributes,
    WriteTileAttributes,
    ReadX,
    WriteX,
}
