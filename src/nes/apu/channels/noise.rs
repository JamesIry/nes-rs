use crate::nes::apu::SoundEnableFlags;

use super::{Channel, Envelope, FrequencyTimer, LengthCounter};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct NoiseChannel {
    envelope: Envelope,
    length_counter: LengthCounter,
    frequency_timer: FrequencyTimer,
    sequencer: NoiseSequencer,

    enabled: bool,
}
impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            envelope: Envelope::new(),
            length_counter: LengthCounter::new(),
            frequency_timer: FrequencyTimer::new(true),
            sequencer: NoiseSequencer::new(),

            enabled: false,
        }
    }
}
const NOISE_PERIOD_LOOKUP: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];
impl Channel for NoiseChannel {
    fn set_register(&mut self, n: u8, value: u8) -> u8 {
        let old = self.read_register(n);
        match n {
            0 => {
                self.envelope.load_bits(value);
                self.length_counter.halted = self.envelope.loop_enable;
            }
            1 => {}
            2 => {
                self.sequencer.load_bits(value);
                self.frequency_timer.period =
                    NOISE_PERIOD_LOOKUP[self.sequencer.period_index as usize];
            }
            3 => {
                self.length_counter.load_bits(value, self.enabled);
                self.envelope.start = true;
            }
            _ => unreachable!("Invalid register {}", n),
        }
        old
    }

    fn read_register(&self, n: u8) -> u8 {
        match n {
            0 => self.envelope.read_bits(),
            1 => 0,
            2 => self.sequencer.read_bits(),
            3 => self.length_counter.read_bits(),
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
        SoundEnableFlags::Noise
    }

    fn clock(&mut self, read_cycle: bool) -> u8 {
        if self.frequency_timer.clock(read_cycle) {
            self.sequencer.advance_position();
        }

        if self.enabled && self.sequencer.gate() && self.length_counter.gate() {
            self.envelope.output
        } else {
            0
        }
    }

    fn quarter_frame_clock(&mut self) {
        self.envelope.quarter_frame_clock();
    }

    fn half_frame_clock(&mut self) {
        self.length_counter.half_frame_clock();
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct NoiseSequencer {
    mode: bool,
    period_index: u8,
    shift_register: u16,
}
impl NoiseSequencer {
    fn new() -> Self {
        Self {
            mode: false,
            period_index: 0,
            shift_register: 1,
        }
    }

    fn load_bits(&mut self, value: u8) {
        self.mode = value & 0b10000000 != 0;
        self.period_index = value & 0b00001111;
    }

    fn read_bits(&self) -> u8 {
        (if self.mode { 0b10000000 } else { 0 }) | (self.period_index & 0b00001111)
    }

    fn advance_position(&mut self) {
        let other_bit = if self.mode { 6 } else { 1 };
        let feedback = (self.shift_register ^ (self.shift_register >> other_bit)) & 1;

        self.shift_register >>= 1;
        self.shift_register |= feedback << 14;
    }

    fn gate(&self) -> bool {
        // this logic is backwards, not sure it matters
        self.shift_register & 1 == 0
    }
}
