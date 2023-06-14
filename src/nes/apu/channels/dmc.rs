use crate::nes::apu::SoundEnableFlags;

use super::Channel;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct DMCChannel {
    registers: [u8; 4],
    samples_remaining: u8,
}
impl DMCChannel {
    pub fn new() -> Self {
        Self {
            registers: [0xFF; 4],
            samples_remaining: 0,
        }
    }

    pub fn restart(&mut self) {
        // todo
    }
}
impl Channel for DMCChannel {
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
            self.samples_remaining = 0;
        } else if self.samples_remaining == 0 {
            self.restart();
        }
    }

    fn get_enabled(&self) -> bool {
        self.samples_remaining != 0
    }

    fn get_enabled_flag(&self) -> SoundEnableFlags {
        SoundEnableFlags::DMC
    }

    fn clock(&mut self, _read_cycle: bool) -> u8 {
        0
    }
    fn quarter_frame_clock(&mut self) {}
    fn half_frame_clock(&mut self) {}
}
