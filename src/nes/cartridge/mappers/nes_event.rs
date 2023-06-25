use crate::{
    bus::InterruptFlags,
    nes::cartridge::{CartridgeCore, Mapper, MirrorType},
};

/**
 * Mapper 105
 */
pub struct NesEvent {
    core: CartridgeCore,

    sram_disabled: bool,
    control_reg: u8,
    irq_counter_enabled: bool,
    irq_occurred: bool,
    prg_chip_select: u8,
    prg_bank_reg_a: u8,
    prg_bank_reg_b: u8,
    shift_register: u8,
    shift_count: u8,
    cycle_count: usize,
    last_write_cycle: usize,
    irq_count: u32,
    irq_counter_target: u32,
    initialized: InitState,
}

impl NesEvent {
    pub fn new(core: CartridgeCore, dip_switches: u8) -> Self {
        let mut result = Self {
            core,

            sram_disabled: false,
            control_reg: 0b00001100,
            irq_counter_enabled: false,
            irq_occurred: false,
            prg_chip_select: 0,
            prg_bank_reg_a: 0,
            prg_bank_reg_b: 0,
            shift_register: 0,
            shift_count: 0,
            cycle_count: 0,
            last_write_cycle: 0xFFFF,
            irq_count: 0,
            irq_counter_target: 0b0010_0000_0000_0000_0000_0000_0000_0000
                | ((dip_switches as u32) << 25),
            initialized: InitState::NotStarted,
        };
        result.reconfigure_banks();

        result
    }

    fn configure(&mut self, addr: u16, value: u8) -> u8 {
        let old = self.shift_register;

        if self.cycle_count != self.last_write_cycle + 1 {
            self.last_write_cycle = self.cycle_count;
            if value & 0b10000000 != 0 {
                self.control_reg |= 0b00001100;
                self.shift_count = 0;
                self.shift_register = 0;
                self.reconfigure_banks();
            } else {
                self.shift_register >>= 1;
                self.shift_register |= (value & 0b00000001) << 4;
                self.shift_count += 1;
                if self.shift_count == 5 {
                    let value = self.shift_register;
                    self.shift_register = 0;
                    self.shift_count = 0;
                    match addr {
                        0x8000..=0x9FFF => self.control_reg = value,
                        0xA000..=0xBFFF => self.set_irq_reg(value),
                        0xC000..=0xDFFF => {
                            unimplemented!("This register isn't used in this mapper")
                        }
                        0xE000..=0xFFFF => self.set_prg_reg_b(value),
                        _ => unreachable!("Couldn't find register for {}", addr),
                    }
                    self.reconfigure_banks();
                }
            }
        }
        old
    }

    fn reconfigure_banks(&mut self) {
        let mirror_type = match self.mirror_mode() {
            0 => MirrorType::SingleScreen(0),
            1 => MirrorType::SingleScreen(1),
            2 => MirrorType::Vertical,
            3 => MirrorType::Horizontal,
            _ => unreachable!("Invalid mirror mode {}", self.mirror_mode()),
        };
        self.core.vram.set_mirror_type(mirror_type);

        match (self.initialized, self.prg_chip_select, self.prg_bank_mode()) {
            (s, _, _) if s == InitState::NotStarted || s == InitState::Started => {
                self.core.prg_rom.set_bank_size_k(32);
                self.core.prg_rom.set_bank(0, 0);
            }
            (InitState::Initialized, 0, _) => {
                self.core.prg_rom.set_bank_size_k(32);
                self.core.prg_rom.set_bank(0, self.prg_bank_reg_a as i16);
            }
            (InitState::Initialized, 1, 0..=1) => {
                self.core.prg_rom.set_bank_size_k(32);
                self.core
                    .prg_rom
                    .set_bank(0, (self.prg_bank_reg_b >> 1) as i16 + 4);
            }
            (InitState::Initialized, 1, 2) => {
                self.core.prg_rom.set_bank_size_k(16);
                #[allow(clippy::identity_op)]
                self.core.prg_rom.set_bank(0, 0 + 8);
                self.core
                    .prg_rom
                    .set_bank(1, self.prg_bank_reg_b as i16 + 8);
            }
            (InitState::Initialized, 1, 3) => {
                self.core.prg_rom.set_bank_size_k(16);
                self.core
                    .prg_rom
                    .set_bank(0, self.prg_bank_reg_b as i16 + 8);
                self.core.prg_rom.set_bank(1, -1);
            }

            _ => unreachable!("Invalid prg bank mode {}", self.prg_bank_mode()),
        };
    }

    fn mirror_mode(&self) -> u8 {
        self.control_reg & 0b00000011
    }

    fn prg_bank_mode(&self) -> u8 {
        (self.control_reg & 0b00001100) >> 2
    }

    fn set_prg_reg_b(&mut self, value: u8) {
        self.prg_bank_reg_b = value & 0b00000111;
        self.sram_disabled = value & 0b00010000 != 0;
    }

    fn set_irq_reg(&mut self, value: u8) {
        self.irq_counter_enabled = value & 0b0001_0000 == 0;
        if self.irq_counter_enabled {
            if self.initialized == InitState::NotStarted {
                self.initialized = InitState::Started
            }
        } else {
            self.irq_count = 0;
            self.irq_occurred = false;
            if self.initialized == InitState::Started {
                self.initialized = InitState::Initialized
            }
        }
        self.prg_chip_select = (value & 0b0000_1000) >> 3;
        self.prg_bank_reg_a = (value & 0b0000_0110) >> 1;
    }
}

impl Mapper for NesEvent {
    fn read_cpu(&mut self, addr: u16) -> u8 {
        if self.core.sram.contains_addr(addr) {
            if self.sram_disabled {
                0
            } else {
                self.core.sram.read(addr)
            }
        } else {
            self.core.read_cpu(addr)
        }
    }
    fn write_cpu(&mut self, addr: u16, value: u8) -> u8 {
        if self.core.sram.contains_addr(addr) {
            if self.sram_disabled {
                0
            } else {
                self.core.sram.write(addr, value)
            }
        } else if self.core.prg_rom.contains_addr(addr) {
            self.configure(addr, value)
        } else {
            self.core.write_cpu(addr, value)
        }
    }

    fn read_ppu(&mut self, addr: u16) -> u8 {
        self.core.read_ppu(addr)
    }

    fn write_ppu(&mut self, addr: u16, value: u8) -> u8 {
        self.core.write_ppu(addr, value)
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        if self.irq_counter_enabled {
            self.irq_count += 1;
            if self.irq_count == self.irq_counter_target {
                self.irq_occurred = true;
            }
        }

        if self.irq_occurred {
            InterruptFlags::IRQ
        } else {
            InterruptFlags::empty()
        }
    }

    fn ppu_bus_clock(&mut self) {
        self.cycle_count += 1;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InitState {
    NotStarted,
    Started,
    Initialized,
}
