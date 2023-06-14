use super::SoundEnableFlags;

pub mod dmc;
pub mod noise;
pub mod pulse;
pub mod triangle;

pub trait Channel {
    fn set_register(&mut self, n: u8, value: u8) -> u8;
    fn read_register(&self, n: u8) -> u8;
    fn get_enabled_flag(&self) -> SoundEnableFlags;
    fn set_enabled(&mut self, value: bool);
    fn get_enabled(&self) -> bool;
    fn quarter_frame_clock(&mut self);
    fn clock(&mut self, read_cycle: bool) -> u8;
    fn half_frame_clock(&mut self);
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Envelope {
    start: bool,
    divider: u8,
    loop_enable: bool,
    constant_volume: bool,
    decay_level: u8,
    period_or_volume: u8,
    output: u8,
}

impl Envelope {
    pub fn new() -> Self {
        Self {
            start: true,
            divider: 0,
            loop_enable: false,
            constant_volume: false,
            decay_level: 0,
            period_or_volume: 0,
            output: 0,
        }
    }

    fn quarter_frame_clock(&mut self) {
        if self.start {
            self.start = false;
            self.decay_level = 15;
            self.divider = self.decay_level;
        } else if self.divider == 0 {
            self.divider = self.period_or_volume;
            if self.decay_level == 0 {
                if self.loop_enable {
                    self.decay_level = 15;
                }
            } else {
                self.decay_level -= 1;
            }
        } else {
            self.divider -= 1;
        }

        self.output = if self.constant_volume {
            self.period_or_volume
        } else {
            self.decay_level
        };
    }

    fn load_bits(&mut self, value: u8) {
        self.loop_enable = value & 0b00100000 != 0;
        self.constant_volume = value & 0b00010000 != 0;
        self.period_or_volume = value & 0b00001111;
    }

    fn read_bits(&self) -> u8 {
        (if self.loop_enable { 0b00100000 } else { 0 })
            | (if self.constant_volume { 0b00010000 } else { 0 })
            | (self.period_or_volume & 0b00001111)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FrequencyTimer {
    every_other_clock: bool,
    period: u16,
    value: u16,
}

impl FrequencyTimer {
    pub fn new(every_other_clock: bool) -> Self {
        Self {
            every_other_clock,
            period: 0,
            value: 0,
        }
    }

    fn clock(&mut self, read_cycle: bool) -> bool {
        let mut result = false;
        if !self.every_other_clock || !read_cycle {
            if self.value == 0 {
                self.value = self.period;
                result = true;
            } else {
                self.value -= 1;
            }
        }
        result
    }

    fn load_low_bits(&mut self, value: u8) {
        self.period = (self.period & 0b1111111100000000) | value as u16
    }

    fn read_low_bits(&self) -> u8 {
        (self.period & 0b0000000011111111) as u8
    }

    fn load_high_bits(&mut self, value: u8) {
        self.period = (self.period & 0b0000000011111111) | (((value & 0b00000111) as u16) << 8);
    }

    fn read_high_bits(&self) -> u8 {
        ((self.period >> 8) & 0b00000111) as u8
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct LengthCounter {
    period_index: u8,
    value: u8,
    halted: bool,
}

impl LengthCounter {
    pub fn new() -> Self {
        Self {
            period_index: 0,
            value: 0,
            halted: false,
        }
    }

    fn half_frame_clock(&mut self) {
        const PERIOD_LOOKUP: [u8; 32] = [
            10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20,
            96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
        ];

        if !self.halted {
            if self.value == 0 {
                self.value = PERIOD_LOOKUP[self.period_index as usize];
            } else {
                self.value -= 1;
            }
        }
    }

    fn gate(&self) -> bool {
        self.value != 0
    }

    fn load_halted_bits(&mut self, value: u8) {
        self.halted = value & 0b00100000 != 0;
    }

    fn load_period_index_bits(&mut self, value: u8) {
        self.period_index = (value & 0b11111000) >> 3
    }

    fn read_period_index_bits(&self) -> u8 {
        (self.period_index << 3) & 0b11111000
    }
}
