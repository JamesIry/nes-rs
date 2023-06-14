use crate::nes::apu::SoundEnableFlags;

use super::{Channel, Envelope};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct NoiseChannel {
    envelope: Envelope,
    registers: [u8; 4],
    enabled: bool,
    length_counter: u8,
}
impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            envelope: Envelope::new(),
            registers: [0xFF; 4],
            enabled: false,
            length_counter: 0,
        }
    }
}
impl Channel for NoiseChannel {
    fn set_register(&mut self, n: u8, value: u8) -> u8 {
        let old = self.read_register(n);
        self.registers[n as usize] = value;

        if n == 3 {
            self.envelope.start = true;
        }
        old
    }

    fn read_register(&self, n: u8) -> u8 {
        self.registers[n as usize]
    }

    fn set_enabled(&mut self, value: bool) {
        if !value {
            self.length_counter = 0;
        }
        self.enabled = value
    }

    fn get_enabled(&self) -> bool {
        self.enabled && self.length_counter != 0
    }

    fn get_enabled_flag(&self) -> SoundEnableFlags {
        SoundEnableFlags::Noise
    }

    fn clock(&mut self, _read_cycle: bool) -> u8 {
        0
    }
    fn quarter_frame_clock(&mut self) {}
    fn half_frame_clock(&mut self) {}
}
