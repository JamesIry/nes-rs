use crate::nes::apu::SoundEnableFlags;

use super::{Channel, Envelope, FrequencyTimer, LengthCounter};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PulseChannel {
    envelope: Envelope,
    sweep: PulseSweep,
    frequency_timer: FrequencyTimer,
    length_counter: LengthCounter,
    sequencer: PulseSequencer,

    enabled: bool,
}
impl PulseChannel {
    pub fn new(channel2: bool) -> Self {
        Self {
            envelope: Envelope::new(),
            sweep: PulseSweep::new(channel2),
            frequency_timer: FrequencyTimer::new(true),
            length_counter: LengthCounter::new(),
            sequencer: PulseSequencer::new(),

            enabled: false,
        }
    }
}
impl Channel for PulseChannel {
    fn set_register(&mut self, n: u8, value: u8) -> u8 {
        let old = self.read_register(n);
        match n {
            0 => {
                self.sequencer.load_bits(value);
                self.envelope.load_bits(value);
                self.length_counter.halted = self.envelope.loop_enable;
            }
            1 => {
                self.sweep.load_bits(value);
            }
            2 => {
                self.frequency_timer.load_low_bits(value);
            }
            3 => {
                self.frequency_timer.load_high_bits(value);
                self.length_counter.load_bits(value, self.enabled);
                self.envelope.start = true;
                self.sequencer.start = true;
            }
            _ => unreachable!("Invalid register {}", n),
        }
        old
    }

    fn read_register(&self, n: u8) -> u8 {
        match n {
            0 => self.sequencer.read_bits() | self.envelope.read_bits(),
            1 => self.sweep.read_bits(),
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
        if self.sweep.twos_complement {
            SoundEnableFlags::Pulse2
        } else {
            SoundEnableFlags::Pulse1
        }
    }

    fn clock(&mut self, read_cycle: bool) -> u8 {
        if self.frequency_timer.clock(read_cycle) {
            self.sequencer.advance_position();
        }

        if self.enabled && self.sweep.gate() && self.sequencer.gate() && self.length_counter.gate()
        {
            self.envelope.output
        } else {
            0
        }
    }

    fn quarter_frame_clock(&mut self) {
        self.envelope.quarter_frame_clock();
    }

    fn half_frame_clock(&mut self) {
        self.frequency_timer.period = self.sweep.half_frame_clock(self.frequency_timer.period);

        self.length_counter.half_frame_clock();
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PulseSweep {
    enabled: bool,
    period: u8,
    negative: bool,
    shift_count: u8,
    twos_complement: bool,
    muting: bool,
    start: bool,
    divider: u8,
}
impl PulseSweep {
    pub fn new(twos_complement: bool) -> Self {
        Self {
            twos_complement,
            enabled: false,
            period: 0,
            negative: false,
            shift_count: 0,
            muting: false,
            start: false,
            divider: 0,
        }
    }

    fn half_frame_clock(&mut self, current_period: u16) -> u16 {
        let raw_change = current_period >> self.shift_count;
        let change = match (self.negative, self.twos_complement) {
            (false, _) => raw_change,
            (true, true) => (!raw_change).wrapping_add(1),
            (true, false) => !raw_change,
        };
        let raw_target_period = current_period.wrapping_add(change);
        // clamp to 0 if negative
        let target_period = if raw_target_period & 0b1000000000000000 != 0 {
            0
        } else {
            raw_target_period
        };
        self.muting = current_period < 8 || target_period > 0x07FF;

        let result = if self.divider == 0 && self.enabled && !self.muting {
            target_period
        } else {
            current_period
        };

        if self.divider == 0 || self.start {
            self.divider = self.period;
            self.start = false;
        } else {
            self.divider -= 1;
        }

        result
    }

    fn gate(&self) -> bool {
        !self.muting
    }

    fn load_bits(&mut self, value: u8) {
        self.enabled = value & 0b10000000 != 0;
        self.period = (value & 0b01110000) >> 4;
        self.negative = value & 0b00001000 != 0;
        self.shift_count = value & 0b00000111;
        self.start = true;
    }

    fn read_bits(&self) -> u8 {
        (if self.enabled { 0b10000000 } else { 0 })
            | ((self.period << 4) & 0b01110000)
            | (if self.negative { 0b00001000 } else { 0 })
            | (self.shift_count & 0b00000111)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct PulseSequencer {
    duty_cycle: u8,
    sequence: u8,
    position: u8,
    start: bool,
}

const PULSE_SEQUENCE_TABLE: [u8; 4] = [0b00000001, 0b00000011, 0b00001111, 0b11111100];
impl PulseSequencer {
    fn new() -> Self {
        Self {
            duty_cycle: 0,
            sequence: PULSE_SEQUENCE_TABLE[0],
            position: 0,
            start: false,
        }
    }

    fn advance_position(&mut self) {
        if self.start {
            self.start = false;
            self.position = 0;
        } else if self.position == 0 {
            self.position = 7;
        } else {
            self.position -= 1;
        }
    }

    fn gate(&self) -> bool {
        ((self.sequence >> self.position) & 1) != 0
    }

    fn load_bits(&mut self, value: u8) {
        self.duty_cycle = (value & 0b11000000) >> 6;
        self.sequence = PULSE_SEQUENCE_TABLE[self.duty_cycle as usize];
    }

    fn read_bits(&self) -> u8 {
        (self.duty_cycle << 6) & 0b11000000
    }
}
