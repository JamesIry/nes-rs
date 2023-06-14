#![allow(clippy::upper_case_acronyms)]

mod channels;

extern crate bitflags;

use std::{cell::RefCell, rc::Rc};

use crate::{bus::BusDevice, cpu::CPU};

use self::channels::{
    dmc::DMCChannel, noise::NoiseChannel, pulse::PulseChannel, triangle::TriangleChannel, Channel,
};

const RANGE_START: u16 = 0x4000;
const RANGE_END: u16 = 0x401F;
const ADDR_MASK: u16 = 0x401F;

pub struct APU {
    cpu: Rc<RefCell<CPU>>,
    read_cycle: bool,
    last_read: u8,

    frame_counter: u16,
    frame_counter_reset_state: FrameCounterResetState,

    oam_dma_state: OamDmaState,
    oam_dma_data: u8,

    input_port1: u8,
    input_port2: u8,

    input_port_ctrl: u8,
    oam_dma_page: u8,
    input_registers: [u8; 2],
    sound_enable_register_high: SoundEnableFlags, //0x4015 ish
    frame_counter_control: FrameCounterFlags,     //0x4017 ish
    pulse_channel1: PulseChannel,
    pulse_channel2: PulseChannel,
    triangle_channel: TriangleChannel,
    noise_channel: NoiseChannel,
    dmc_channel: DMCChannel,
}

impl APU {
    pub fn new(cpu: Rc<RefCell<CPU>>) -> Self {
        Self {
            cpu,
            read_cycle: true,
            last_read: 0xFF,

            frame_counter: 15,
            frame_counter_reset_state: FrameCounterResetState::None,

            oam_dma_state: OamDmaState::NoDma,
            oam_dma_data: 0,

            input_port1: 0,
            input_port2: 0,

            oam_dma_page: 0xFF,
            input_port_ctrl: 0xFF,
            input_registers: [0xFF; 2],
            sound_enable_register_high: SoundEnableFlags::empty(),
            frame_counter_control: FrameCounterFlags::empty(),
            pulse_channel1: PulseChannel::new(false),
            pulse_channel2: PulseChannel::new(true),
            triangle_channel: TriangleChannel::new(),
            noise_channel: NoiseChannel::new(),
            dmc_channel: DMCChannel::new(),
        }
    }

    #[must_use]
    pub fn clock(&mut self) -> (bool, f32) {
        self.read_cycle = !self.read_cycle;

        self.manage_input_ports();

        self.manage_oam_dma();
        self.manage_frame_counter();

        let read_cycle = self.read_cycle;
        let outputs = self.channel_set().map(|c| c.clock(read_cycle) as f32);
        let pulse_out = if outputs[0] == 0.0 && outputs[1] == 0.0 {
            0.0
        } else {
            95.88 / ((8128.0 / (outputs[0] + outputs[1])) + 100.0)
        };

        let tnd_out = if outputs[2] == 0.0 && outputs[3] == 0.0 && outputs[4] == 0.0 {
            0.0
        } else {
            159.79
                / (1.0 / (outputs[2] / 8227.0 + outputs[3] / 12241.0 + outputs[4] / 22638.0)
                    + 100.0)
        };

        (
            self.sound_enable_register_high
                .contains(SoundEnableFlags::FrameInterrupt),
            pulse_out + tnd_out,
        )
    }

    pub fn reset(&mut self) {
        self.read_cycle = true;
        self.oam_dma_state = OamDmaState::NoDma;
        self.oam_dma_data = 0;
        self.input_port1 = 0;
        self.input_port2 = 0;
        self.last_read = 0;
        // self.frame_counter_reset_state = FrameCounterResetState::None;

        self.oam_dma_page = 0xFF;
        self.input_port_ctrl = 0xFF;
        self.input_registers = [0xFF; 2];
        self.sound_enable_register_high = SoundEnableFlags::empty();

        // TODO not sure these are quite right for reset state
        self.pulse_channel1 = PulseChannel::new(false);
        self.pulse_channel2 = PulseChannel::new(true);
        self.triangle_channel = TriangleChannel::new();
        self.noise_channel = NoiseChannel::new();
        self.dmc_channel = DMCChannel::new();

        // not updated at reset
        // self.frame_counter = 0;
        // self.frame_counter_control;
    }

    pub fn set_input_port1(&mut self, value: u8) {
        self.input_port1 = value;
    }

    pub fn set_input_port2(&mut self, value: u8) {
        self.input_port2 = value;
    }

    fn manage_oam_dma(&mut self) {
        match (self.oam_dma_state, self.read_cycle) {
            (OamDmaState::NoDma, _) => (),
            (OamDmaState::Requested, true) => {
                self.cpu.as_ref().borrow_mut().set_rdy(false);
                self.oam_dma_state = OamDmaState::Ready;
            }
            (OamDmaState::Requested, false) => (),
            (OamDmaState::Ready, false) => {
                self.read_oam_dma(0);
                self.oam_dma_state = OamDmaState::Executing(0);
            }
            (OamDmaState::Ready, true) => unreachable!("Oam state ready on even frame"),
            (OamDmaState::Executing(offset), true) => self.read_oam_dma(offset as u16),
            (OamDmaState::Executing(offset), false) => {
                self.write_oam_dma();
                if offset == 255 {
                    self.cpu.as_ref().borrow_mut().set_rdy(true);
                    self.oam_dma_state = OamDmaState::NoDma;
                } else {
                    self.oam_dma_state = OamDmaState::Executing(offset + 1);
                }
            }
        }
    }

    fn manage_frame_counter(&mut self) {
        match (self.frame_counter_reset_state, self.read_cycle) {
            (FrameCounterResetState::WaitingForRead, true) => {
                self.frame_counter_reset_state = FrameCounterResetState::WaitingForWrite;
                self.frame_counter += 1;
            }

            (FrameCounterResetState::WaitingForWrite, false) => {
                self.frame_counter = 0;
                if self
                    .frame_counter_control
                    .contains(FrameCounterFlags::FiveStepMode)
                {
                    self.clock_half_frame();
                    self.clock_quarter_frame();
                }
                self.frame_counter_reset_state = FrameCounterResetState::None
            }
            _ => self.frame_counter += 1,
        }

        if !self
            .frame_counter_control
            .contains(FrameCounterFlags::FiveStepMode)
        {
            match self.frame_counter {
                7457 => self.clock_quarter_frame(),
                14913 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                22371 => {
                    self.clock_quarter_frame();
                }
                29828 => self.set_frame_interrupt(true),
                29829 => {
                    self.set_frame_interrupt(true);
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                29830 => {
                    self.set_frame_interrupt(true);
                    self.frame_counter = 0;
                }
                _ => (),
            }
        } else {
            match self.frame_counter {
                7457 => self.clock_quarter_frame(),
                14913 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                22371 => {
                    self.clock_quarter_frame();
                }
                29829 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                29830 => (), // "extra," do nothing
                37281 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                37282 => self.frame_counter = 0,
                _ => (),
            }
        }
    }

    fn set_frame_interrupt(&mut self, value: bool) {
        if value {
            if !self
                .frame_counter_control
                .contains(FrameCounterFlags::IRQInhibit)
            {
                self.sound_enable_register_high
                    .insert(SoundEnableFlags::FrameInterrupt);
            }
        } else {
            self.sound_enable_register_high
                .remove(SoundEnableFlags::FrameInterrupt);
        }
    }

    fn clock_half_frame(&mut self) {
        for c in self.channel_set() {
            c.half_frame_clock();
        }
    }

    fn clock_quarter_frame(&mut self) {
        for c in self.channel_set() {
            c.quarter_frame_clock();
        }
    }

    fn read_oam_dma(&mut self, offset: u16) {
        let addr = ((self.oam_dma_page as u16) << 8) | offset;
        self.oam_dma_data = self.cpu.as_ref().borrow_mut().read_bus_byte(addr);
    }

    fn write_oam_dma(&self) {
        self.cpu
            .as_ref()
            .borrow_mut()
            .write_bus_byte(0x2004, self.oam_dma_data);
    }

    fn manage_input_ports(&mut self) {
        if self.input_port_ctrl & 0b00000001 != 0 {
            self.input_registers[0] = self.input_port1;
            self.input_registers[1] = self.input_port2;
        }
    }

    fn bus_read_input_register(&mut self, input_no: usize) -> u8 {
        let result = self.input_registers[input_no] & 0b00000001;
        if self.input_port_ctrl & 0b00000001 == 0 {
            self.input_registers[input_no] >>= 1;
            self.input_registers[input_no] |= 0b10000000;
        }
        result
    }

    #[inline]
    fn channel_set(&mut self) -> [&mut dyn Channel; 5] {
        [
            &mut self.pulse_channel1,
            &mut self.pulse_channel2,
            &mut self.triangle_channel,
            &mut self.noise_channel,
            &mut self.dmc_channel,
        ]
    }
}

#[allow(clippy::manual_range_contains)]
impl BusDevice for APU {
    fn read(&mut self, addr: u16) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            let physical = addr & ADDR_MASK;
            let result = match physical {
                0x4000..=0x4013 => self.last_read,
                0x4015 => {
                    let status =
                        self.channel_set()
                            .iter()
                            .fold(SoundEnableFlags::empty(), |f, c| {
                                f | if c.get_enabled() {
                                    c.get_enabled_flag()
                                } else {
                                    SoundEnableFlags::empty()
                                }
                            });

                    let result = (status | self.sound_enable_register_high).bits()
                        | (self.last_read & 0b00100000);
                    self.set_frame_interrupt(false);
                    result

                    // note, this read doesn't affect self.last_read
                }
                0x4016 => {
                    self.last_read = self.bus_read_input_register(0);
                    self.last_read
                }
                0x4017 => {
                    self.last_read = self.bus_read_input_register(1);
                    self.last_read
                }
                _ => 0xFF,
            };
            Some(result)
        } else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            let physical = addr & ADDR_MASK;
            let old = match physical {
                0x4000..=0x4013 => {
                    let channel_number = (physical >> 2) & 0xF;
                    let channel = &mut self.channel_set()[channel_number as usize];
                    let reg = (physical & 0b00000011) as u8;
                    let old = channel.read_register(reg);
                    channel.set_register(reg, data);
                    old
                }
                0x4014 => {
                    let old = self.oam_dma_page;
                    self.oam_dma_state = OamDmaState::Requested;
                    self.oam_dma_page = data;
                    old
                }
                0x4015 => {
                    let status = SoundEnableFlags::from_bits_truncate(data);
                    for channel in self.channel_set() {
                        channel.set_enabled(status.contains(channel.get_enabled_flag()));
                    }
                    self.sound_enable_register_high
                        .set(SoundEnableFlags::DMCInterrupt, false);
                    0xFF // this is just to make nestest.rs happy
                }
                0x4016 => {
                    let old = self.input_port_ctrl;
                    self.input_port_ctrl = data;
                    old
                }
                0x4017 => {
                    let old = self.frame_counter_control.bits();
                    self.frame_counter_control = FrameCounterFlags::from_bits_truncate(data);
                    if self
                        .frame_counter_control
                        .contains(FrameCounterFlags::IRQInhibit)
                    {
                        self.set_frame_interrupt(false);
                    }
                    self.frame_counter_reset_state = FrameCounterResetState::WaitingForRead;
                    old
                }
                _ => 0xFF,
            };
            Some(old)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OamDmaState {
    NoDma,
    Requested,
    Ready,
    Executing(u8),
}

bitflags::bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct SoundEnableFlags: u8 {
        const DMCInterrupt = 0b10000000;
        const FrameInterrupt = 0b01000000;

        const DMC = 0b00010000;
        const Noise = 0b00001000;
        const Triangle = 0b00000100;
        const Pulse2 = 0b00000010;
        const Pulse1 = 0b00000001;
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct FrameCounterFlags: u8 {
        const FiveStepMode = 0b10000000;
        const IRQInhibit = 0b01000000;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FrameCounterResetState {
    None,
    WaitingForRead,
    WaitingForWrite,
}
