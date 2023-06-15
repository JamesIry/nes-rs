use crate::nes::apu::SoundEnableFlags;

use super::{Channel, FrequencyTimer, LengthCounter};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TriangleChannel {
    linear_counter: LinearCounter,
    length_counter: LengthCounter,
    frequency_timer: FrequencyTimer,
    sequencer: TriangleSequencer,

    enabled: bool,
}
impl TriangleChannel {
    pub fn new() -> Self {
        Self {
            linear_counter: LinearCounter::new(),
            length_counter: LengthCounter::new(),
            frequency_timer: FrequencyTimer::new(false),
            sequencer: TriangleSequencer::new(),

            enabled: false,
        }
    }
}
impl Channel for TriangleChannel {
    fn set_register(&mut self, n: u8, value: u8) -> u8 {
        let old = self.read_register(n);
        match n {
            0 => {
                self.linear_counter.load_bits(value);
                self.length_counter.halted = self.linear_counter.control_flag;
            }
            1 => {}
            2 => {
                self.frequency_timer.load_low_bits(value);
            }
            3 => {
                self.frequency_timer.load_high_bits(value);
                self.length_counter.load_bits(value, self.enabled);
                self.linear_counter.start = true;
            }
            _ => unreachable!("Invalid register {}", n),
        }
        old
    }

    fn read_register(&self, n: u8) -> u8 {
        match n {
            0 => self.linear_counter.read_bits(),
            1 => 0,
            2 => self.frequency_timer.read_low_bits(),
            3 => self.frequency_timer.read_high_bits() | self.length_counter.read_bits(),
            _ => unreachable!("Invalid register {}", n),
        }
    }

    fn set_enabled(&mut self, value: bool) {
        if !value {
            self.length_counter.value = 0;
        }
        self.enabled = value
    }

    fn get_enabled(&self) -> bool {
        self.enabled && self.length_counter.value != 0
    }

    fn get_enabled_flag(&self) -> SoundEnableFlags {
        SoundEnableFlags::Triangle
    }
    fn clock(&mut self, read_cycle: bool) -> u8 {
        if self.frequency_timer.clock(read_cycle)
            && self.linear_counter.gate()
            && self.length_counter.gate()
        {
            self.sequencer.advance_position();
        }

        if self.enabled && self.linear_counter.gate() && self.length_counter.gate() {
            self.sequencer.output
        } else {
            0
        }
    }

    fn quarter_frame_clock(&mut self) {
        self.linear_counter.quarter_frame_clock();
    }

    fn half_frame_clock(&mut self) {
        self.length_counter.half_frame_clock();
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct TriangleSequencer {
    count_up: bool,
    output: u8,
}

impl TriangleSequencer {
    fn new() -> Self {
        Self {
            count_up: false,
            output: 15,
        }
    }
    fn advance_position(&mut self) {
        if !self.count_up {
            if self.output == 0 {
                self.count_up = true;
            } else {
                self.output -= 1
            }
        } else if self.output == 15 {
            self.count_up = false;
        } else {
            self.output += 1
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct LinearCounter {
    start: bool,
    period: u8,
    count: u8,
    control_flag: bool,
}

impl LinearCounter {
    fn new() -> Self {
        Self {
            start: false,
            period: 0,
            count: 0,
            control_flag: false,
        }
    }

    fn quarter_frame_clock(&mut self) {
        if self.start {
            self.count = self.period;
        } else if self.count != 0 {
            self.count -= 1;
        }
        if !self.control_flag {
            self.start = false;
        }
    }

    fn load_bits(&mut self, value: u8) {
        self.control_flag = value & 0b10000000 != 0;
        self.period = value & 0b01111111;
    }

    fn read_bits(&self) -> u8 {
        (if self.control_flag { 0b10000000 } else { 0 }) | (self.period & 0b01111111)
    }

    fn gate(&self) -> bool {
        self.count != 0
    }
}
