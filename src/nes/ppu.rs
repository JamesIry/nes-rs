mod flags;
mod rgb;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod unit_tests;

use std::{cell::RefCell, rc::Rc};

use crate::bus::{Bus, BusDevice};

use self::{
    flags::{CtrlFlags, MaskFlags, StatusFlags},
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
const SECONDARY_OAM_SIZE: usize = 0x0080;

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
    ctrl_high_register: CtrlFlags,
    mask_register: MaskFlags,
    status_register: StatusFlags,

    bus: Bus,

    primary_oam: OAMData<PRIMARY_OAM_SIZE>,
    secondary_oam: OAMData<SECONDARY_OAM_SIZE>,
    sprite_row_data: SpriteRowSet,

    palettes: [u8; PALETTE_SIZE],
    scan_line: i16,
    tick: u16,

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
            sprite_row_data: SpriteRowSet::new(),
            ctrl_high_register: CtrlFlags::empty(),
            mask_register: MaskFlags::empty(),
            status_register: StatusFlags::VerticalBlank | StatusFlags::SpriteOverflow,
            palettes: INITIAL_PALETTE_VALUES,
            scan_line: -1,
            tick: 0,
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
    pub fn clock(&mut self) -> (bool, bool) {
        self.manage_bus_request();
        self.manage_status();
        if self.rendering_enabled() && self.scan_line < 240 {
            self.manage_sprite_evaluation();
            self.manage_shift_registers();
            self.manage_render();
            self.manage_scrolling();
        }
        let end_of_frame = self.manage_tick();
        (end_of_frame, self.manage_nmi())
    }

    fn rendering_enabled(&self) -> bool {
        self.mask_register
            .intersects(MaskFlags::ShowBG | MaskFlags::ShowSprites)
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn BusDevice>>) {
        self.bus.add_device(device);
    }

    pub fn reset(&mut self) {
        self.resetting = true;
        self.even_frame = true;
        self.ctrl_high_register = CtrlFlags::empty();
        self.mask_register = MaskFlags::empty();
        self.scan_line = -1;
        self.tick = 0;
        self.even_frame = true;
        self.write_toggle = false;
        self.vram_address = VramAddress::new();
        self.bg_shift_registers = BGShiftRegisterSet::new();
        self.secondary_oam = OAMData::new();
        self.sprite_row_data = SpriteRowSet::new();
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
                self.status_register = StatusFlags::empty(); // clear StatusFlag::VerticalBlank, StatusFlag::Sprite0Hit, and StatusFlag::SpriteOverflow
                self.resetting = false; //
                self.primary_oam.write_enabled = false;
            }
            (241, 1) => {
                self.set_status_flag(StatusFlags::VerticalBlank, true);
                self.primary_oam.write_enabled = true;
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

                self.secondary_oam.load_addr(0);
                self.secondary_oam.write_enabled = true;
                self.secondary_oam.has_sprite0 = false;
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
                    self.primary_oam.load_addr(0);
                    self.primary_oam.read_enabled = true;
                    self.secondary_oam.load_addr(0);
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
                        let sprite_height = if self.read_ctrl_flag(CtrlFlags::SpriteSizeLarge) {
                            16
                        } else {
                            8
                        };

                        if y <= self.scan_line
                            && self.scan_line < y + sprite_height
                            && !self.read_status_flag(StatusFlags::SpriteOverflow)
                        {
                            if !self.secondary_oam.write_enabled {
                                self.set_status_flag(StatusFlags::SpriteOverflow, true);
                            } else if self.primary_oam.addr < 4 {
                                self.secondary_oam.has_sprite0 = true;
                            }

                            self.secondary_oam.inc_addr();
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
                self.secondary_oam.load_addr(0);
                self.sprite_row_data.set_current_sprite(0);

                // latch seondary_oam down into the sprite data we're going to render
                for i in 0..8 {
                    self.sprite_row_data.current_sprite().sprite0 =
                        i == 0 && self.secondary_oam.has_sprite0;
                    self.sprite_row_data.current_sprite().y = self.secondary_oam.read_data();
                    self.secondary_oam.inc_addr();
                    self.sprite_row_data.current_sprite().tile_id = self.secondary_oam.read_data();
                    self.secondary_oam.inc_addr();
                    self.sprite_row_data.current_sprite().attributes =
                        self.secondary_oam.read_data();
                    self.secondary_oam.inc_addr();
                    self.sprite_row_data.current_sprite().x = self.secondary_oam.read_data() as i16;
                    self.secondary_oam.inc_addr();
                    self.sprite_row_data.inc_sprite();
                }
            }
            t if 258 <= t && t <= 340 => {}

            _ => unreachable!("Unreachable tick number {}", self.tick),
        }
    }

    fn manage_shift_registers(&mut self) {
        if self.tick > 0 {
            match self.tick % 8 {
                1 => {
                    self.bus_request = BusRequest::Read(self.vram_address.get_nametable_address());
                    if self.tick >= 9 {
                        self.bg_shift_registers.latch();
                    }
                }
                2 => self
                    .bg_shift_registers
                    .load_name_table_data(self.data_buffer),

                3 if self.tick != 339 => {
                    self.bus_request = BusRequest::Read(self.vram_address.get_attribute_address())
                }
                4 if self.tick != 340 => self
                    .bg_shift_registers
                    .load_attribute_data(self.data_buffer, self.vram_address.get_attribute_shift()),

                3 if self.tick == 339 => {
                    self.bus_request = BusRequest::Read(self.vram_address.get_nametable_address())
                }
                4 if self.tick == 340 => self
                    .bg_shift_registers
                    .load_name_table_data(self.data_buffer),

                5 => {
                    if 261 <= self.tick && self.tick <= 320 {
                        if -1 <= self.scan_line && self.scan_line <= 239 {
                            self.bus_request =
                                BusRequest::Read(self.compute_base_sprite_pattern_address());
                        }
                    } else {
                        let address = self.bg_shift_registers.get_pattern_address(
                            self.read_ctrl_flag(CtrlFlags::BackgroundPatternHigh),
                            self.vram_address.get_fine_y(),
                        );
                        self.bus_request = BusRequest::Read(address);
                    };
                }
                6 => {
                    if 261 <= self.tick && self.tick <= 320 {
                        if -1 <= self.scan_line && self.scan_line <= 239 {
                            self.sprite_row_data
                                .current_sprite()
                                .set_pattern_low(self.data_buffer);
                        }
                    } else {
                        self.bg_shift_registers
                            .load_pattern_data_low(self.data_buffer);
                    }
                }
                7 => {
                    if 261 <= self.tick && self.tick <= 320 {
                        if -1 <= self.scan_line && self.scan_line <= 239 {
                            self.bus_request = BusRequest::Read(
                                self.compute_base_sprite_pattern_address() | 0b00001000,
                            );
                        }
                    } else {
                        let address = self.bg_shift_registers.get_pattern_address(
                            self.read_ctrl_flag(CtrlFlags::BackgroundPatternHigh),
                            self.vram_address.get_fine_y(),
                        ) | 0b00001000;
                        self.bus_request = BusRequest::Read(address);
                    };
                }
                0 => {
                    if 261 <= self.tick && self.tick <= 320 {
                        if -1 <= self.scan_line && self.scan_line <= 239 {
                            self.sprite_row_data
                                .current_sprite()
                                .set_pattern_high(self.data_buffer);
                            self.sprite_row_data.inc_sprite();
                        }
                    } else if 0 < self.tick {
                        self.bg_shift_registers
                            .load_pattern_data_high(self.data_buffer);
                    }
                }

                _ => (),
            }

            if self.tick <= 336 {
                self.bg_shift_registers.shift();
            }
            if self.tick <= 256 {
                self.sprite_row_data.shift();
            }
        }
    }

    fn compute_base_sprite_pattern_address(&mut self) -> u16 {
        let sprite_large_mode = self.read_ctrl_flag(CtrlFlags::SpriteSizeLarge);
        let sprite_high_mode = self.read_ctrl_flag(CtrlFlags::SpriteTableHigh);
        let result = self.sprite_row_data.current_sprite().get_pattern_address(
            sprite_large_mode,
            sprite_high_mode,
            self.scan_line as u16,
        );

        result
    }

    fn manage_render(&mut self) {
        let x = self.tick;
        let y = self.scan_line as u16;

        if x < 256 && y < 240 {
            let (mut bg_palette, mut bg_color) = {
                self.bg_shift_registers
                    .get_palette_number_and_color(self.temporary_vram_address.fine_x)
            };

            let (sprite0, mut sprite_palette, mut sprite_color, bg_priority) = {
                if let Some(sprite) = self.sprite_row_data.first_opaque() {
                    let (palette_number, color) = sprite.get_palette_number_and_color();
                    (
                        sprite.sprite0,
                        palette_number,
                        color,
                        sprite.get_bg_priority(),
                    )
                } else {
                    (false, 0x0010, 0, true)
                }
            };

            if bg_color != 0 && sprite_color != 0 && sprite0 {
                self.status_register |= StatusFlags::Sprite0Hit;
            }

            if !(self.read_mask_flag(MaskFlags::ShowBG)
                && (x >= 8 || self.read_mask_flag(MaskFlags::ShowLeft8BG)))
            {
                bg_palette = 0;
                bg_color = 0;
            }
            if !(self.read_mask_flag(MaskFlags::ShowSprites)
                && (x >= 8 || self.read_mask_flag(MaskFlags::ShowLeft8Sprites)))
            {
                sprite_palette = 0b0100;
                sprite_color = 0;
            }

            let (palette_number, color) = match (bg_color, sprite_color, bg_priority) {
                (0, 0, _) => (0, 0),
                (0, s, _) => (sprite_palette, s),
                (b, 0, _) => (bg_palette, b),
                (_, s, false) => (sprite_palette, s),
                (b, _, true) => (bg_palette, b),
            };

            /*
             * 00111111 xxx 1 PP CC
             * |||||||| ||| | || ||
             * |||||||| ||| | || ++- Color number from tile data
             * |||||||| ||| | ++---- Palette number from attribute table or OAM
             * |||||||| ||| +------- Background/Sprite select, 0=bg, 1=sprite
             * |||||||| +++--------- doesn't matter, effectively set to 0 by mirroring
             * ++++++++------------- 0x3F00 - 0x3FFF
             */
            let palette_address = 0x3F00 | (palette_number << 2) | color;

            const SHOW_GRID: bool = false;

            /*
             * 00 VV HHHH
             * || || ||||
             * || || ++++- Hue (phase, determines NTSC/PAL chroma)
             * || ++------ Value (voltage, determines NTSC/PAL luma)
             ++--------- Unimplemented, reads back as 0
            */
            let color = self.read_palette(palette_address);

            let (r, g, b) = if SHOW_GRID && ((x % 32 == 0) || (y % 32 == 0)) {
                (255, 0, 0)
            } else if SHOW_GRID && ((x % 16 == 0) || (y % 16 == 0)) {
                (0, 255, 0)
            } else if SHOW_GRID && ((x % 8 == 0) || (y % 8 == 0)) {
                (0, 0, 255)
            } else {
                translate_nes_to_rgb(color)
            };
            let f = &mut self.renderer;
            f(x, y, r, g, b);
        }
    }

    fn manage_tick(&mut self) -> bool {
        // skip a tick on odd frames when rendering is enabled
        if self.scan_line == -1 && self.tick == 339 && !self.even_frame && self.rendering_enabled()
        {
            self.tick = 340;
        }
        let mut end_of_frame = false;
        self.tick += 1;
        if self.tick == 341 {
            self.tick = 0;
            self.scan_line += 1;
            if self.scan_line == 261 {
                end_of_frame = true;
                self.scan_line = -1;
                self.even_frame = !self.even_frame;
            }
        }
        end_of_frame
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
        if self.read_mask_flag(MaskFlags::Greyscale) {
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
        !(self.read_status_flag(StatusFlags::VerticalBlank)
            && self.read_ctrl_flag(CtrlFlags::NmiEnabled))
    }

    fn get_ctrl_flags(&self) -> CtrlFlags {
        self.ctrl_high_register
            | CtrlFlags::from_bits_truncate(self.temporary_vram_address.get_nametable_bits())
    }

    fn set_ctrl_flags(&mut self, data: CtrlFlags) -> CtrlFlags {
        let old = self.get_ctrl_flags();
        if !self.resetting {
            self.ctrl_high_register = CtrlFlags::from_bits_truncate(data.bits() & 0b11111100);
            self.temporary_vram_address.set_nametable_bits(data.bits());
        }
        old
    }

    #[cfg(test)]
    fn set_ctrl_flag(&mut self, flag: CtrlFlags, value: bool) {
        if value {
            self.set_ctrl_flags(self.get_ctrl_flags() | flag);
        } else {
            self.set_ctrl_flags(self.get_ctrl_flags() & !flag);
        }
    }

    fn read_ctrl_flag(&self, flag: CtrlFlags) -> bool {
        self.get_ctrl_flags().contains(flag)
    }

    #[cfg(test)]
    fn set_mask_flag(&mut self, flag: MaskFlags, value: bool) {
        self.mask_register.set(flag, value);
    }

    fn read_mask_flag(&self, flag: MaskFlags) -> bool {
        self.mask_register.contains(flag)
    }

    fn set_status_flag(&mut self, flag: StatusFlags, value: bool) {
        self.status_register.set(flag, value);
    }

    fn read_status_flag(&self, flag: StatusFlags) -> bool {
        self.status_register.contains(flag)
    }

    fn inc_vram_addr(&mut self) {
        let amount = if self.read_ctrl_flag(CtrlFlags::IncrementAcross) {
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
                    let result = self.status_register.bits() | (self.data_buffer & 0x1F);
                    self.set_status_flag(StatusFlags::VerticalBlank, false);
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

                    let flags = CtrlFlags::from_bits_retain(data);

                    self.set_ctrl_flags(flags);

                    old.bits()
                }
                0x2001 => {
                    let old = self.mask_register;
                    if !self.resetting {
                        self.mask_register = MaskFlags::from_bits_truncate(data);
                    }
                    old.bits()
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

    fn latch(&mut self) {
        self.data = (self.data & 0b1111111100000000) | (self.prefetch as u16)
    }

    fn shift(&mut self) {
        self.data <<= 1;
    }

    fn bit(&self, n: u8) -> u16 {
        (self.data >> (15 - n)) & 0b1
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

    fn latch(&mut self) {
        self.high.latch();
        self.low.latch();
    }

    fn shift(&mut self) {
        self.high.shift();
        self.low.shift();
    }

    fn bits(&self, fine_x: u8) -> u16 {
        (self.high.bit(fine_x) << 1) | self.low.bit(fine_x)
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
        self.attribute_data.shift();
        self.pattern_data.shift();
    }

    fn latch(&mut self) {
        self.pattern_data.latch();
        self.attribute_data.latch();
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

    fn get_pixel_color_number(&self, fine_x: u8) -> u16 {
        self.pattern_data.bits(fine_x)
    }

    fn get_pallete_number(&self, fine_x: u8) -> u16 {
        self.attribute_data.bits(fine_x)
    }

    fn get_palette_number_and_color(&self, fine_x: u8) -> (u16, u16) {
        let pixel_color_number = self.get_pixel_color_number(fine_x);

        let palette_number = if pixel_color_number == 0 {
            0
        } else {
            self.get_pallete_number(fine_x)
        };

        (palette_number, pixel_color_number)
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
    /**
     * SIZE/4 = 64 sprites
     * 0 - Y position (-1 because sprites are evaluated 1 scanline ahead)
     * 1 - Tile number
     * 2 - Attributes
     *     V H R 000 PP
     *     | | | ||| ||
     *     | | | ||| ++- Palette (4 to 7) of sprite
     *     | | | +++---- Unimplemented (read 0)
     *     | | +-------- Priority (0: in front of background; 1: behind background)
     *     | +---------- Flip sprite horizontally
     *     +------------ Flip sprite vertically
     * 3 - X position
     */
    table: [u8; SIZE],
    read_enabled: bool,
    write_enabled: bool,
    addr_mask: u8,
    has_sprite0: bool,
}

impl<const SIZE: usize> OAMData<SIZE> {
    fn new() -> Self {
        Self {
            addr: 0,
            table: [0; SIZE],
            read_enabled: true,
            write_enabled: true,
            addr_mask: (SIZE - 1) as u8,
            has_sprite0: false,
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

    fn is_full(&self) -> bool {
        self.addr as usize == SIZE - 1
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct SpriteRowData {
    y: u8,
    tile_id: u8,
    attributes: u8,
    x: i16,
    sprite0: bool,

    pattern_high: u8,
    pattern_low: u8,
}
impl SpriteRowData {
    fn new() -> Self {
        Self {
            y: 0xFF,
            tile_id: 0xFF,
            attributes: 0xFF,
            x: 0xFF,
            sprite0: false,

            pattern_high: 0xFF,
            pattern_low: 0xFF,
        }
    }

    fn get_pattern_address(&self, large_sprite_mode: bool, sprite_high_mode: bool, y: u16) -> u16 {
        let mut y_offset = y.wrapping_sub(self.y as u16);

        let sprite_high = if large_sprite_mode {
            y_offset > 7
        } else {
            sprite_high_mode
        };

        if self.get_vertical_flip() {
            let max_sprite_height: u16 = if large_sprite_mode { 15 } else { 7 };
            y_offset = max_sprite_height.wrapping_sub(y_offset);
        }

        /*
        000 H RRRR CCCC P YYY
        ||| | |||| |||| | +++- Y: Y offset, the row number within a tile
        ||| | |||| |||| +----- P: Bit plane (0: lower, 1: upper) (0 for reading sprite low, 1 for reading sprite high)
        ||| | |||| ++++------- C: Tile column (lower nibble of tile id)
        ||| | ++++------------ R: Tile row (upper nibble of tile id)
        ||| +----------------- H: Half of pattern table (0: left, 1: right) = CtrlFlag::SpritePatternHigh for 8x8 sprites,
                                                                              or 0/1 for upper/lower half on 8/16 sprites
        +++------------------- 0: Pattern table is 0x0000 - 0x01FFF
        */
        (if sprite_high { 0x1000 } else { 0x0000 })
            | ((self.tile_id as u16) << 4)
            | (y_offset & 0b00000111)
        // the "bit plane" is set to 1 by the pattern data fetching code as needed
    }

    fn get_palette_number_and_color(&self) -> (u16, u16) {
        let pixel_color_number = self.get_pixel_color_number();
        let palette_number = if pixel_color_number == 0 {
            0
        } else {
            self.get_pallete_number()
        };

        (0b0100 | palette_number, pixel_color_number)
    }

    fn get_pallete_number(&self) -> u16 {
        (self.attributes & 0b11) as u16
    }

    fn get_pixel_color_number(&self) -> u16 {
        (((self.pattern_high >> 6) & 0b10) | ((self.pattern_low >> 7) & 0b01)) as u16
    }

    /**
     * True - bg in front, false - bg behind
     */
    fn get_bg_priority(&self) -> bool {
        self.attributes & 0b00100000 != 0
    }

    fn get_horizontal_flip(&self) -> bool {
        self.attributes & 0b01000000 != 0
    }

    fn get_vertical_flip(&self) -> bool {
        self.attributes & 0b10000000 != 0
    }

    fn shift(&mut self) {
        if self.x != 255 {
            if self.live() {
                self.pattern_high <<= 1;
                self.pattern_low <<= 1;
            }
            self.x -= 1;
        }
    }

    fn live(&self) -> bool {
        -8 < self.x && self.x <= 0
    }

    fn set_pattern_high(&mut self, data: u8) {
        self.pattern_high = if self.get_horizontal_flip() {
            SpriteRowData::reverse_byte(data)
        } else {
            data
        }
    }

    fn set_pattern_low(&mut self, data: u8) {
        self.pattern_low = if self.get_horizontal_flip() {
            SpriteRowData::reverse_byte(data)
        } else {
            data
        }
    }

    fn reverse_byte(mut b: u8) -> u8 {
        // 0123 4567 -> 4567 0123
        b = (b & 0xF0) >> 4 | (b & 0x0F) << 4;
        // 45 67,01 23 -> 67 45,23 01
        b = (b & 0xCC) >> 2 | (b & 0x33) << 2;
        // 6 7,4 5,2 3,0 1 -> 7 6,5 4,3 2,1 0
        b = (b & 0xAA) >> 1 | (b & 0x55) << 1;
        b
    }
}

impl Default for SpriteRowData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct SpriteRowSet {
    sprite_data: [SpriteRowData; 8],
    sprite_number: usize,
    write_enabled: bool,
}

impl SpriteRowSet {
    fn new() -> Self {
        Self {
            sprite_data: [SpriteRowData::new(); 8],
            sprite_number: 0,
            write_enabled: true,
        }
    }

    fn set_current_sprite(&mut self, n: usize) {
        self.sprite_number = n;
    }

    fn inc_sprite(&mut self) {
        self.sprite_number = (self.sprite_number + 1) & 0b00000111;
    }

    fn shift(&mut self) {
        for sprite in &mut self.sprite_data {
            sprite.shift();
        }
    }

    fn first_opaque(&self) -> Option<SpriteRowData> {
        self.sprite_data
            .into_iter()
            .find(|data| data.live() && data.get_pixel_color_number() != 0)
    }

    fn current_sprite(&mut self) -> &mut SpriteRowData {
        &mut self.sprite_data[self.sprite_number]
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
