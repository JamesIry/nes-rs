use std::{cell::RefCell, rc::Rc};

use crate::{cpu::CPU, nes::apu::SoundEnableFlags};

use super::{Channel, FrequencyTimer};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct DMCChannel {
    period_index: u8,
    memory_reader: MemoryReader,
    frequency_timer: FrequencyTimer,
    output_unit: OutputUnit,
}
impl DMCChannel {
    pub fn new() -> Self {
        Self {
            period_index: 0,
            memory_reader: MemoryReader::new(),
            frequency_timer: FrequencyTimer::new(false),
            output_unit: OutputUnit::new(),
        }
    }

    pub fn manage_dma(&mut self, read_cycle: bool, cpu: &mut Rc<RefCell<CPU>>) {
        if self.output_unit.sample_buffer.is_none() {
            self.output_unit.sample_buffer = self.memory_reader.fetch_dma(read_cycle, cpu)
        }
    }
}
impl Channel for DMCChannel {
    fn set_register(&mut self, n: u8, value: u8) -> u8 {
        const PERIOD_TABLE: [u16; 16] = [
            428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
        ];
        let old = self.read_register(n);
        match n {
            0 => {
                self.memory_reader.load_flag_bits(value);
                self.period_index = value & 0b00001111;
                self.frequency_timer.period = PERIOD_TABLE[self.period_index as usize];
            }
            1 => self.output_unit.load_bits(value),
            2 => self.memory_reader.load_sample_addres_bits(value),
            3 => self.memory_reader.load_sample_length_bits(value),
            _ => unreachable!("Invalid register {}", n),
        }
        old
    }

    fn read_register(&self, n: u8) -> u8 {
        match n {
            0 => self.memory_reader.read_flag_bits() | (self.period_index & 0b00001111),
            1 => self.output_unit.read_bits(),
            2 => self.memory_reader.read_sample_address_bits(),
            3 => self.memory_reader.read_sample_length_bits(),
            _ => unreachable!("Invalid register {}", n),
        }
    }

    fn set_enabled(&mut self, value: bool) {
        if !value {
            self.memory_reader.samples_remaining = 0;
        } else if self.memory_reader.samples_remaining == 0 {
            self.memory_reader.start = true;
        }
    }

    fn get_enabled(&self) -> bool {
        self.memory_reader.samples_remaining != 0
    }

    fn get_enabled_flag(&self) -> SoundEnableFlags {
        SoundEnableFlags::DMC
    }

    fn clock(&mut self, read_cycle: bool) -> u8 {
        if self.frequency_timer.clock(read_cycle) {
            self.output_unit.advance_position();
        }
        self.output_unit.output
    }
    fn quarter_frame_clock(&mut self) {}
    fn half_frame_clock(&mut self) {}
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct OutputUnit {
    bits_remaining: u8,
    silence: bool,
    shift_register: u8,
    sample_buffer: Option<u8>,
    output: u8,
}
impl OutputUnit {
    fn new() -> Self {
        Self {
            bits_remaining: 8,
            silence: true,
            shift_register: 0,
            sample_buffer: None,
            output: 0,
        }
    }
    fn advance_position(&mut self) {
        if !self.silence {
            let bit = self.shift_register & 1;
            if (bit == 1) && self.output <= 125 {
                self.output += 2;
            } else if self.output >= 2 {
                self.output -= 2;
            }
        }
        self.shift_register >>= 1;
        self.bits_remaining -= 1;
        if self.bits_remaining == 0 {
            self.bits_remaining = 8;
            if self.sample_buffer.is_none() {
                self.silence = true;
            } else {
                self.silence = false;

                self.shift_register = self.sample_buffer.take().unwrap();
            }
        }
    }

    fn load_bits(&mut self, value: u8) {
        self.output = value & 0b01111111
    }

    fn read_bits(&self) -> u8 {
        self.output & 0b01111111
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct MemoryReader {
    irq_enabled: bool,
    irq_occurred: bool,
    loop_enabled: bool,
    samples_remaining: u16,
    sample_address: u16,
    current_address: u16,
    sample_length: u16,
    start: bool,
}
impl MemoryReader {
    fn new() -> Self {
        Self {
            irq_enabled: false,
            irq_occurred: false,
            loop_enabled: false,
            samples_remaining: 0,
            sample_address: 0xC000,
            current_address: 0xC000,
            sample_length: 1,
            start: false,
        }
    }

    fn fetch_dma(&mut self, read_cycle: bool, cpu: &mut Rc<RefCell<CPU>>) -> Option<u8> {
        if self.start {
            self.start = false;
            self.current_address = self.sample_address;
            self.samples_remaining = self.sample_length;
        }

        let mut sample = None;
        if self.samples_remaining != 0 && read_cycle {
            // TODO, need some more stall states related to OAM DMA, aligning, etc
            // TODO need to drop the CPU RDY
            sample = Some(
                cpu.as_ref()
                    .borrow_mut()
                    .read_bus_byte(self.current_address),
            );
            if self.current_address == 0xFFFF {
                self.current_address = 0x8000
            } else {
                self.current_address += 1;
            }
            self.samples_remaining -= 1;
            if self.samples_remaining == 0 {
                self.start = self.loop_enabled;
                self.irq_occurred = self.irq_enabled;
            }
        }
        sample
    }

    fn load_flag_bits(&mut self, value: u8) {
        self.irq_enabled = value & 0b10000000 != 0;
        if !self.irq_enabled {
            self.irq_occurred = false;
        }
        self.loop_enabled = value & 0b01000000 != 0;
    }

    fn read_flag_bits(&self) -> u8 {
        (if self.irq_enabled { 0b10000000 } else { 0 })
            | (if self.loop_enabled { 0b01000000 } else { 0 })
    }

    fn load_sample_length_bits(&mut self, value: u8) {
        self.sample_length = ((value as u16) << 4) + 1
    }

    fn read_sample_length_bits(&self) -> u8 {
        ((self.sample_length.wrapping_sub(1)) >> 4) as u8
    }

    fn load_sample_addres_bits(&mut self, value: u8) {
        self.sample_address = 0b1100000000000000 & ((value as u16) << 6);
    }

    fn read_sample_address_bits(&self) -> u8 {
        (self.sample_address >> 6) as u8
    }
}
