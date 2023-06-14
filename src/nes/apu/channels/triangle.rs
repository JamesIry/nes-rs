use crate::nes::apu::SoundEnableFlags;

use super::Channel;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TriangleChannel {
    registers: [u8; 4],
    enabled: bool,
    length_counter: u8,
    length_counter_load: u8,
}
impl TriangleChannel {
    pub fn new() -> Self {
        Self {
            registers: [0xFF; 4],
            enabled: false,
            length_counter: 0,
            length_counter_load: 0,
        }
    }
}
impl Channel for TriangleChannel {
    fn set_register(&mut self, n: u8, value: u8) -> u8 {
        let old = self.read_register(n);
        self.registers[n as usize] = value;
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
        self.enabled
    }

    fn get_enabled_flag(&self) -> SoundEnableFlags {
        SoundEnableFlags::Triangle
    }

    fn clock(&mut self, _read_cycle: bool) -> u8 {
        0
    }
    fn quarter_frame_clock(&mut self) {}
    fn half_frame_clock(&mut self) {}
}
