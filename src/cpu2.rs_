use crate::{
    bus::Bus,
    cpu::{flags::StatusFlags, CPUCycleType},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CPUType {
    RP2A03,
    MOS6502,
}

const MICRO_CODE_PER_INSTRUCTION: usize = 10;
const TOTAL_MICRO_CODE: usize = 256 * MICRO_CODE_PER_INSTRUCTION;
struct CPU {
    cpu_type: CPUType,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub status: StatusFlags,
    pub sp: u8,
    pub pc: Address,
    pub cycles: usize,

    bus: Bus,

    micro_code: [fn(&mut Self) -> (); TOTAL_MICRO_CODE],
    instruction: u8,
    bus_target: BusTarget,
    code_step: usize,
    cycle_type: CPUCycleType,
    address: Address,
}

impl CPU {
    fn new(cpu_type: CPUType) -> Self {
        let micro_code = setup_micro_code();
        Self {
            cpu_type,
            a: 0,
            x: 0,
            y: 0,
            status: StatusFlags::Break | StatusFlags::Unused,
            sp: 0,
            pc: Address::new(),

            cycles: 0,
            bus: Bus::new(),

            micro_code,
            bus_target: BusTarget::Instruction,
            instruction: 0,
            code_step: 0,
            cycle_type: CPUCycleType::Read,

            address: Address::new(),
        }
    }

    fn clock(&mut self) {
        let location = self.instruction as usize * MICRO_CODE_PER_INSTRUCTION + self.code_step;
        let f = self.micro_code[location];
        self.code_step += 1; // TODO only if RDY
        f(self);

        let target = match self.bus_target {
            BusTarget::Instruction => &mut self.instruction,
        };
        match self.cycle_type {
            CPUCycleType::Read => {
                *target = self.bus.read(self.address.addr());
            }
            CPUCycleType::Write => {
                self.bus.write(self.address.addr(), *target);
            }
        }
    }

    fn unimplemented(&mut self) {
        unimplemented!()
    }

    fn end_instruction(&mut self) {
        self.address = self.pc;
        self.bus_target = BusTarget::Instruction;
        self.code_step = 0;
        self.pc.inc();
    }

    fn asl(&mut self) {
        self.a = self.a << 1;
    }
}

fn setup_micro_code() -> [fn(&mut CPU) -> (); TOTAL_MICRO_CODE] {
    let mut mc: [fn(&mut CPU) -> (); TOTAL_MICRO_CODE] = [CPU::unimplemented; TOTAL_MICRO_CODE];
    let mut i: usize;

    // 0A ASL A
    i = 0x0A * MICRO_CODE_PER_INSTRUCTION;
    mc[i] = CPU::asl;
    mc[i + 1] = CPU::end_instruction;

    mc
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Address {
    low: u8,
    high: u8,
    carry: bool,
}

impl Address {
    fn new() -> Self {
        Self {
            low: 0,
            high: 0,
            carry: false,
        }
    }

    fn inc(&mut self) {
        self.carry = self.low == 0xFF;
        self.low = self.low.wrapping_add(1);
    }

    fn add(&mut self, value: u8) {
        let old_low = self.low;
        self.low = self.low.wrapping_add(value);
        self.carry = self.low < old_low;
    }

    fn carry(&mut self) {
        if self.carry {
            self.high = self.high.wrapping_add(1);
            self.carry = false;
        }
    }

    fn addr(&self) -> u16 {
        ((self.high as u16) << 8) | (self.low as u16)
    }

    fn set_full(&mut self, addr: u16) {
        self.low = (addr & 0b0000_0000_1111_1111) as u8;
        self.high = (addr >> 8) as u8;
        self.carry = false;
    }
}

enum BusTarget {
    Instruction,
}
