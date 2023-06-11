#![allow(clippy::upper_case_acronyms)]

extern crate bitflags;

use std::{cell::RefCell, rc::Rc};

use crate::{bus::BusDevice, cpu::CPU};

const RANGE_START: u16 = 0x4000;
const RANGE_END: u16 = 0x401F;
const ADDR_MASK: u16 = 0x401F;

/* Not really a APU yet. Just some read/write registers and oam dma */
pub struct APU {
    cpu: Rc<RefCell<CPU>>,
    read_cycle: bool,
    oam_dma_state: OamDmaState,
    oam_dma_data: u8,
    input_port_ctrl: u8,
    input_port1: u8,
    input_port2: u8,
    last_read: u8,

    oam_dma_page: u8,
    input_registers: [u8; 2],
    sound_enable_register: SoundEnableFlags,
    frame_counter_control: FrameCounterFlags,
    pulse_channels: [PulseChannel; 2],
    triangle_channel: TriangleChannel,
    noise_channel: NoiseChannel,
    dmc_channel: DMCChannel,
}

impl APU {
    pub fn new(cpu: Rc<RefCell<CPU>>) -> Self {
        Self {
            cpu,
            read_cycle: true,
            oam_dma_state: OamDmaState::NoDma,
            oam_dma_data: 0,
            input_port1: 0,
            input_port2: 0,
            last_read: 0xFF,

            oam_dma_page: 0xFF,
            input_port_ctrl: 0xFF,
            input_registers: [0xFF; 2],
            sound_enable_register: SoundEnableFlags::empty(),
            frame_counter_control: FrameCounterFlags::empty(),
            pulse_channels: [PulseChannel::new(); 2],
            triangle_channel: TriangleChannel::new(),
            noise_channel: NoiseChannel::new(),
            dmc_channel: DMCChannel::new(),
        }
    }

    pub fn clock(&mut self) {
        self.read_cycle = !self.read_cycle;

        self.manage_input_ports();

        self.manage_oam_dma();
    }

    pub fn reset(&mut self) {
        self.read_cycle = true;
        self.oam_dma_state = OamDmaState::NoDma;
        self.oam_dma_data = 0;
        self.input_port1 = 0;
        self.input_port2 = 0;
        self.last_read = 0;

        self.oam_dma_page = 0xFF;
        self.input_port_ctrl = 0xFF;
        self.input_registers = [0xFF; 2];
        self.sound_enable_register = SoundEnableFlags::empty();
        self.frame_counter_control = FrameCounterFlags::empty();
        self.pulse_channels = [PulseChannel::new(); 2];
        self.triangle_channel = TriangleChannel::new();
        self.noise_channel = NoiseChannel::new();
        self.dmc_channel = DMCChannel::new();
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
}

#[allow(clippy::manual_range_contains)]
impl BusDevice for APU {
    fn read(&mut self, addr: u16) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            let physical = addr & ADDR_MASK;
            let result = match physical {
                0x4000..=0x4013 => self.last_read,
                0x4015 => {
                    let status = if self.pulse_channels[0].get_length_counter() != 0 {
                        SoundEnableFlags::Pulse1
                    } else {
                        SoundEnableFlags::empty()
                    } | if self.pulse_channels[1].get_length_counter() != 0 {
                        SoundEnableFlags::Pulse2
                    } else {
                        SoundEnableFlags::empty()
                    } | if self.triangle_channel.get_length_counter() != 0 {
                        SoundEnableFlags::Triangle
                    } else {
                        SoundEnableFlags::empty()
                    } | if self.noise_channel.get_length_counter() != 0 {
                        SoundEnableFlags::Noise
                    } else {
                        SoundEnableFlags::empty()
                    } | if self.dmc_channel.get_samples_remaining() != 0 {
                        SoundEnableFlags::DMC
                    } else {
                        SoundEnableFlags::empty()
                    };

                    let result = status.bits()
                        | (self.sound_enable_register.bits() & 0b11000000)
                        | (self.last_read & 0b00100000);
                    self.sound_enable_register
                        .set(SoundEnableFlags::DMCInterrupt, false);
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
                    let channel: &mut dyn Channel = match channel_number {
                        0 => &mut self.pulse_channels[0],
                        1 => &mut self.pulse_channels[1],
                        2 => &mut self.triangle_channel,
                        3 => &mut self.noise_channel,
                        4 => &mut self.dmc_channel,
                        _ => unreachable!("Got {} as a channel number", channel_number),
                    };
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
                    self.sound_enable_register = SoundEnableFlags::from_bits_truncate(
                        (self.sound_enable_register.bits() & 0b01100000) | (data & 0b00011111),
                    );
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
        const _Unused = 0b00100000;
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
        const _Unused1 = 0b00100000;
        const _Unused2 = 0b00010000;
        const _Unused3 = 0b00001000;
        const _Unused4 = 0b00000100;
        const _Unused5 = 0b00000010;
        const _Unused6 = 0b00000001;
    }
}

trait Channel {
    fn set_register(&mut self, n: u8, value: u8) -> u8;
    fn read_register(&self, n: u8) -> u8;
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct PulseChannel {
    registers: [u8; 4],
}

impl PulseChannel {
    fn new() -> Self {
        Self {
            registers: [0xFF; 4],
        }
    }

    fn get_length_counter(&self) -> u8 {
        (self.registers[3] & 0b11111000) >> 3
    }
}
impl Channel for PulseChannel {
    fn set_register(&mut self, n: u8, value: u8) -> u8 {
        let old = self.read_register(n);
        self.registers[n as usize] = value;
        old
    }

    fn read_register(&self, n: u8) -> u8 {
        self.registers[n as usize]
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct TriangleChannel {
    registers: [u8; 4],
}
impl TriangleChannel {
    fn new() -> Self {
        Self {
            registers: [0xFF; 4],
        }
    }

    fn get_length_counter(&self) -> u8 {
        (self.registers[3] & 0b11111000) >> 3
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
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct NoiseChannel {
    registers: [u8; 4],
}
impl NoiseChannel {
    fn new() -> Self {
        Self {
            registers: [0xFF; 4],
        }
    }

    fn get_length_counter(&self) -> u8 {
        (self.registers[3] & 0b11111000) >> 3
    }
}
impl Channel for NoiseChannel {
    fn set_register(&mut self, n: u8, value: u8) -> u8 {
        let old = self.read_register(n);
        self.registers[n as usize] = value;
        old
    }

    fn read_register(&self, n: u8) -> u8 {
        self.registers[n as usize]
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct DMCChannel {
    registers: [u8; 4],
}
impl DMCChannel {
    fn new() -> Self {
        Self {
            registers: [0xFF; 4],
        }
    }

    fn get_samples_remaining(&self) -> u8 {
        self.registers[3]
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
}
