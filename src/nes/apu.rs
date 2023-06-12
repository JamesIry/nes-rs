#![allow(clippy::upper_case_acronyms)]

extern crate bitflags;

use std::{cell::RefCell, rc::Rc};

use crate::{bus::BusDevice, cpu::CPU};

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
    pub fn clock(&mut self) -> bool {
        self.read_cycle = !self.read_cycle;

        self.manage_input_ports();

        self.manage_oam_dma();
        self.manage_frame_counter();

        self.sound_enable_register_high
            .contains(SoundEnableFlags::FrameInterrupt)
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
        self.pulse_channel1.half_frame_clock();
        self.pulse_channel2.half_frame_clock();
        self.triangle_channel.half_frame_clock();
        self.noise_channel.half_frame_clock();
    }

    fn clock_quarter_frame(&mut self) {
        self.pulse_channel1.quarter_frame_clock();
        self.pulse_channel2.quarter_frame_clock();
        self.triangle_channel.quarter_frame_clock();
        self.noise_channel.quarter_frame_clock();
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

                    // note, this read doesn't not affect self.last_read
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
    struct SoundEnableFlags: u8 {
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

trait Channel {
    fn set_register(&mut self, n: u8, value: u8) -> u8;
    fn read_register(&self, n: u8) -> u8;
    fn get_enabled_flag(&self) -> SoundEnableFlags;
    fn set_enabled(&mut self, value: bool);
    fn get_enabled(&self) -> bool;
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct PulseChannel {
    envelope: Envelope,
    sweep: Sweep,

    enabled: bool,
    duty: u8,

    timer: u16,

    length_counter_load: u8,
    length_counter: u8,
}

impl PulseChannel {
    fn new(channel2: bool) -> Self {
        Self {
            envelope: Envelope::new(),
            sweep: Sweep::new(channel2),

            enabled: false,
            length_counter_load: 0,
            length_counter: 0,

            duty: 0,
            timer: 0,
        }
    }

    fn quarter_frame_clock(&mut self) {
        self.envelope.quarter_frame_clock();
    }
    fn half_frame_clock(&mut self) {
        self.timer = self.sweep.half_frame_clock(self.timer);
    }
}
impl Channel for PulseChannel {
    fn set_register(&mut self, n: u8, value: u8) -> u8 {
        let old = self.read_register(n);
        match n {
            0 => {
                self.duty = (value & 0b11000000) >> 6;
                self.envelope.loop_enable = value & 0b00100000 != 0;
                self.envelope.constant_volume = value & 0b00010000 != 0;
                self.envelope.period = value & 0b00001111;
            }
            1 => {
                self.sweep.enabled = value & 0b10000000 != 0;
                self.sweep.period = (value & 0b01110000) >> 4;
                self.sweep.negative = value & 0b00001000 != 0;
                self.sweep.shift_count = value & 0b00000111;
                self.sweep.start = true;
            }
            2 => self.timer = (self.timer & 0b1111111100000000) | value as u16,
            3 => {
                self.timer =
                    (self.timer & 0b0000000011111111) | (((value & 0b00000111) as u16) << 8);
                self.length_counter_load = (value & 0b11111000) >> 3;
                self.envelope.start_flag = true;
            }
            _ => unreachable!("Invalid register {}", n),
        }
        old
    }

    fn read_register(&self, n: u8) -> u8 {
        match n {
            0 => {
                ((self.duty << 6) & 0b11000000)
                    | (if self.envelope.loop_enable {
                        0b00010000
                    } else {
                        0
                    })
                    | (if self.envelope.constant_volume {
                        0b00010000
                    } else {
                        0
                    })
                    | (self.envelope.period & 0b00001111)
            }
            1 => {
                (if self.sweep.enabled { 0b10000000 } else { 0 })
                    | ((self.sweep.period << 4) & 0b01110000)
                    | (if self.sweep.negative { 0b00001000 } else { 0 })
                    | (self.sweep.shift_count & 0b00000111)
            }
            2 => (self.timer & 0b0000000011111111) as u8,
            3 => {
                ((self.length_counter_load << 3) & 0b11111000)
                    | (((self.timer >> 8) & 0b00000111) as u8)
            }
            _ => unreachable!("Invalid register {}", n),
        }
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
        if self.sweep.twos_complement {
            SoundEnableFlags::Pulse2
        } else {
            SoundEnableFlags::Pulse1
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct TriangleChannel {
    registers: [u8; 4],
    enabled: bool,
    length_counter: u8,
    length_counter_load: u8,
}
impl TriangleChannel {
    fn new() -> Self {
        Self {
            registers: [0xFF; 4],
            enabled: false,
            length_counter: 0,
            length_counter_load: 0,
        }
    }

    fn quarter_frame_clock(&mut self) {}
    fn half_frame_clock(&mut self) {}
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
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct NoiseChannel {
    envelope: Envelope,
    registers: [u8; 4],
    enabled: bool,
    length_counter: u8,
}
impl NoiseChannel {
    fn new() -> Self {
        Self {
            envelope: Envelope::new(),
            registers: [0xFF; 4],
            enabled: false,
            length_counter: 0,
        }
    }

    fn quarter_frame_clock(&mut self) {}
    fn half_frame_clock(&mut self) {}
}
impl Channel for NoiseChannel {
    fn set_register(&mut self, n: u8, value: u8) -> u8 {
        let old = self.read_register(n);
        self.registers[n as usize] = value;

        if n == 3 {
            self.envelope.start_flag = true;
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
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct DMCChannel {
    registers: [u8; 4],
    samples_remaining: u8,
}
impl DMCChannel {
    fn new() -> Self {
        Self {
            registers: [0xFF; 4],
            samples_remaining: 0,
        }
    }

    fn restart(&mut self) {
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
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FrameCounterResetState {
    None,
    WaitingForRead,
    WaitingForWrite,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct Envelope {
    start_flag: bool,
    divider: u8,
    loop_enable: bool,
    constant_volume: bool,
    decay_level: u8,
    period: u8,
    output: u8,
}

impl Envelope {
    fn new() -> Self {
        Self {
            start_flag: true,
            divider: 0,
            loop_enable: false,
            constant_volume: false,
            decay_level: 0,
            period: 0,
            output: 0,
        }
    }

    fn quarter_frame_clock(&mut self) {
        if self.start_flag {
            self.start_flag = false;
            self.decay_level = 15;
            self.divider = self.decay_level;
        } else if self.divider == 0 {
            self.divider = self.period;
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
            self.period
        } else {
            self.decay_level
        };
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct Sweep {
    enabled: bool,
    period: u8,
    negative: bool,
    shift_count: u8,
    twos_complement: bool,
    muting: bool,
    start: bool,
    divider: u8,
}
impl Sweep {
    fn new(twos_complement: bool) -> Self {
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
        } else {
            self.divider -= 1;
        }

        result
    }
}
