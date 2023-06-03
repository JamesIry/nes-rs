// https://c74project.com/microcode/

#![allow(clippy::upper_case_acronyms)]

#[cfg(test)]
mod unit_tests {
    mod test_addressing_modes;
    mod test_clock_and_interrupts;
    mod test_decode;
    mod test_instructions;
    mod test_unofficial_instructions;
}
#[cfg(test)]
mod functional_tests {
    mod functional_test;
}
pub mod decode;
pub mod flags;
pub mod instructions;
pub mod monitor;

use std::cell::RefCell;
use std::rc::Rc;

use crate::bus::Bus;
use crate::bus::BusDevice;
use crate::cpu::flags::*;
use crate::cpu::instructions::*;

use self::monitor::Monitor;
use self::monitor::NulMonitor;

static NMI_ADDR: u16 = 0xFFFA;
static RESET_ADDR: u16 = 0xFFFC;
static IRQ_ADDR: u16 = 0xFFFE;
static SIGN_BIT: u8 = 0b10000000;
static HIGH_BYTE_MASK: u16 = 0xFF00;
static LOW_BYTE_MASK: u16 = 0x00FF;
static STACK_BASE: u16 = 0x0100;

pub struct CPU {
    pub cpu_type: CPUType,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub status: u8,
    pub sp: u8,
    pub pc: u16,
    pub cycles: usize,
    rdy: bool,
    jammed: bool,
    trapped: bool,
    // internal state of executing instuciton
    instruction: Instruction,
    mode: Mode,
    remaining_cycles: u8,
    extra_cycles: u8,
    cycle_on_page_boundary: bool,
    interrupt: Option<Interrupt>,
    pub monitor: Box<dyn Monitor>,
    bus: Bus,
}

impl Default for CPU {
    fn default() -> Self {
        Self::new(CPUType::MOS6502)
    }
}

impl CPU {
    pub fn new(cpu_type: CPUType) -> Self {
        Self {
            cpu_type,
            a: 0,
            x: 0,
            y: 0,
            status: Flag::Break | Flag::Unused,
            sp: 0,
            pc: 0,
            jammed: true,
            trapped: false,
            instruction: Instruction::NOP,
            mode: Mode::Imp,
            remaining_cycles: 0,
            extra_cycles: 0,
            cycle_on_page_boundary: false,
            interrupt: None,
            cycles: 0,
            rdy: true,
            monitor: Box::new(NulMonitor {}),
            bus: Bus::new(),
        }
    }

    #[cfg(test)]
    pub fn reset_to(&mut self, addr: u16) -> u8 {
        self.reset();
        let cycles = self.run_instruction();
        self.pc = addr;
        cycles
    }

    #[cfg(test)]
    pub fn run_instruction(&mut self) -> u8 {
        let mut cycles = 0;

        if self.remaining_cycles == 0 && self.extra_cycles == 0 {
            self.clock();
            cycles += 1;
        }
        while self.remaining_cycles != 0 || self.extra_cycles != 0 {
            self.clock();

            cycles += 1;
        }

        cycles
    }

    #[cfg(test)]
    pub fn stuck(&self) -> bool {
        self.jammed || self.trapped
    }

    pub fn reset(&mut self) {
        self.cycles = 0;
        // while other interrupts will wait for the current instruction
        // to complete, reset starts on the next clock
        self.remaining_cycles = 0;
        self.extra_cycles = 0;
        self.jammed = false;
        self.interrupt = Some(Interrupt::RST);
        self.trapped = false;
    }

    pub fn nmi(&mut self) {
        if self.interrupt != Some(Interrupt::RST) {
            self.interrupt = Some(Interrupt::NMI);
        }
        self.trapped = false;
    }

    #[allow(unused)]
    pub fn irq(&mut self) {
        if self.interrupt.is_none() && !self.read_flag(Flag::InterruptDisable) {
            self.interrupt = Some(Interrupt::IRQ);
            self.trapped = false;
        }
    }

    pub fn clock(&mut self) {
        if self.jammed || !self.rdy {
            return;
        }

        if self.remaining_cycles > 0 {
            if self.remaining_cycles == 1 {
                let (page_boundary, branch_taken) = self.execute(self.instruction, self.mode);
                if page_boundary == PageBoundary::Crossed && self.cycle_on_page_boundary {
                    self.extra_cycles += 1;
                }
                if branch_taken == Branch::Taken {
                    self.extra_cycles += 1;
                }
                self.monitor.end_instruction().unwrap(); // TODO propogate error
            }
            self.remaining_cycles -= 1;
        } else if self.extra_cycles > 0 {
            self.extra_cycles -= 1;
        } else {
            let (instruction, mode, cycles, cycle_on_boundary) = match self.interrupt {
                Some(interrupt) => {
                    let instruction = match interrupt {
                        Interrupt::BRK => {
                            unreachable!("Interrupted with BRK, which shouldn't be possible")
                        }
                        Interrupt::IRQ => Instruction::IRQ,
                        Interrupt::NMI => Instruction::NMI,
                        Interrupt::RST => Instruction::RST,
                    };
                    (instruction, Mode::Imp, 7, false)
                }
                None => {
                    self.monitor
                        .new_instruction(
                            self.cycles,
                            self.pc,
                            self.sp,
                            self.a,
                            self.x,
                            self.y,
                            self.status,
                        )
                        .unwrap(); // TODO propogate error
                    let op = self.fetch_byte();
                    crate::cpu::decode::decode(op)
                }
            };

            self.instruction = instruction;
            self.mode = mode;
            self.remaining_cycles = cycles.wrapping_sub(1);
            self.extra_cycles = 0;
            self.cycle_on_page_boundary = cycle_on_boundary;
        }
        self.cycles += 1;
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn BusDevice>>) {
        self.bus.add_device(device);
    }

    pub fn set_rdy(&mut self, rdy: bool) {
        self.rdy = rdy;
    }

    #[cfg(test)]
    pub fn is_rdy(&self) -> bool {
        self.rdy
    }

    #[cfg(test)]
    pub fn cycles(&self) -> usize {
        self.cycles
    }

    fn interrupt(&mut self, interrupt: Interrupt) -> (PageBoundary, Branch) {
        if interrupt == Interrupt::RST {
            self.sp = 0xFD; // reset doesn't push onto the stack, but simulates it
        } else {
            self.push_word(
                self.pc
                    .wrapping_add(if interrupt == Interrupt::BRK { 1 } else { 0 }),
            );
            let status = if interrupt == Interrupt::BRK {
                self.status | Flag::Break
            } else {
                self.status & !Flag::Break
            };

            self.push_byte(status);
        }
        self.set_flag(Flag::InterruptDisable, true);

        let addr = match interrupt {
            Interrupt::BRK => IRQ_ADDR,
            Interrupt::IRQ => IRQ_ADDR,
            Interrupt::NMI => NMI_ADDR,
            Interrupt::RST => RESET_ADDR,
        };
        self.pc = self.read_bus_word(addr);
        self.interrupt = None;

        (PageBoundary::NotCrossed, Branch::NotTaken)
    }

    fn push_word(&mut self, value: u16) {
        self.push_byte(CPU::high_byte(value));
        self.push_byte(CPU::low_byte(value));
    }

    fn push_byte(&mut self, value: u8) {
        self.write_bus_byte((self.sp as u16) | STACK_BASE, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop_word(&mut self) -> u16 {
        let low_byte = self.pop_byte();
        let high_byte = self.pop_byte();
        CPU::to_word(low_byte, high_byte)
    }

    fn pop_byte(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.read_bus_byte((self.sp as u16) | STACK_BASE)
    }

    fn fetch_byte(&mut self) -> u8 {
        let value = self.read_bus_byte(self.pc);
        self.monitor.fetch_instruction_byte(value).unwrap(); // TODO propogate error?
        self.pc = self.pc.wrapping_add(1);
        value
    }

    fn fetch_word(&mut self) -> u16 {
        let lb = self.fetch_byte();
        let hb = self.fetch_byte();
        CPU::to_word(lb, hb)
    }

    // source https://www.pagetable.com/c64ref/6502/
    // https://www.masswerk.at/6502/6502_instruction_set.html

    fn execute(&mut self, instruction: Instruction, mode: Mode) -> (PageBoundary, Branch) {
        match instruction {
            Instruction::ADC => self.adc(mode),
            Instruction::AND => self.binary(|x, y| x & y, mode),
            Instruction::ASL => self.shift_left(ShiftStyle::ShiftOff, mode),
            Instruction::BCC => self.branch(mode, BranchType::CC),
            Instruction::BCS => self.branch(mode, BranchType::CS),
            Instruction::BEQ => self.branch(mode, BranchType::EQ),
            Instruction::BIT => self.test_bits(mode),
            Instruction::BMI => self.branch(mode, BranchType::MI),
            Instruction::BNE => self.branch(mode, BranchType::NE),
            Instruction::BPL => self.branch(mode, BranchType::PL),
            Instruction::BRK => self.interrupt(Interrupt::BRK),
            Instruction::BVC => self.branch(mode, BranchType::VC),
            Instruction::BVS => self.branch(mode, BranchType::VS),
            Instruction::CLC => self.set_flag(Flag::Carry, false),
            Instruction::CLD => self.set_flag(Flag::Decimal, false),
            Instruction::CLI => self.set_flag(Flag::InterruptDisable, false),
            Instruction::CLV => self.set_flag(Flag::Overflow, false),
            Instruction::CMP => self.compare(mode, self.a),
            Instruction::CPX => self.compare(mode, self.x),
            Instruction::CPY => self.compare(mode, self.y),
            Instruction::DEC => self.unary(|x| x.wrapping_sub(1), mode),
            Instruction::DEX => self.unary(|x| x.wrapping_sub(1), Mode::X),
            Instruction::DEY => self.unary(|x| x.wrapping_sub(1), Mode::Y),
            Instruction::EOR => self.binary(|x, y| x ^ y, mode),
            Instruction::INC => self.unary(|x| x.wrapping_add(1), mode),
            Instruction::INX => self.unary(|x| x.wrapping_add(1), Mode::X),
            Instruction::INY => self.unary(|x| x.wrapping_add(1), Mode::Y),
            Instruction::JMP => self.branch(mode, BranchType::JMP),
            Instruction::JSR => self.branch(mode, BranchType::JSR),
            Instruction::LDA => self.transfer(mode, Mode::A),
            Instruction::LDX => self.transfer(mode, Mode::X),
            Instruction::LDY => self.transfer(mode, Mode::Y),
            Instruction::LSR => self.shift_right(ShiftStyle::ShiftOff, mode),
            Instruction::NOP => self.nop(mode),
            Instruction::ORA => self.binary(|x, y| x | y, mode),
            Instruction::PHA => self.push(Mode::A),
            Instruction::PHP => self.push(Mode::Status),
            Instruction::PLA => self.pop(Mode::A),
            Instruction::PLP => self.pop(Mode::Status),
            Instruction::ROL => self.shift_left(ShiftStyle::Rotate, mode),
            Instruction::ROR => self.shift_right(ShiftStyle::Rotate, mode),
            Instruction::RTI => self.rti(),
            Instruction::RTS => self.rts(),
            Instruction::SBC => self.sbc(mode),
            Instruction::SEC => self.set_flag(Flag::Carry, true),
            Instruction::SED => self.set_flag(Flag::Decimal, true),
            Instruction::SEI => self.set_flag(Flag::InterruptDisable, true),
            Instruction::STA => self.transfer_without_flags(Mode::A, mode),
            Instruction::STX => self.transfer_without_flags(Mode::X, mode),
            Instruction::STY => self.transfer_without_flags(Mode::Y, mode),
            Instruction::TAX => self.transfer(Mode::A, Mode::X),
            Instruction::TAY => self.transfer(Mode::A, Mode::Y),
            Instruction::TSX => self.transfer(Mode::SP, Mode::X),
            Instruction::TXA => self.transfer(Mode::X, Mode::A),
            Instruction::TXS => self.transfer_without_flags(Mode::X, Mode::SP),
            Instruction::TYA => self.transfer(Mode::Y, Mode::A),
            Instruction::RST => self.interrupt(Interrupt::RST),
            Instruction::IRQ => self.interrupt(Interrupt::IRQ),
            Instruction::NMI => self.interrupt(Interrupt::NMI),
            // unofficial instructions
            Instruction::ANC => self.anc(mode),
            Instruction::ASR => self.asr(mode),
            Instruction::ARR => self.arr(mode),
            Instruction::DCP => self.dcp(mode),
            Instruction::ISC => self.isc(mode),
            Instruction::JAM => self.jam(),
            Instruction::LAS => self.las(mode),
            Instruction::LAX => self.lax(mode),
            Instruction::RLA => self.rla(mode),
            Instruction::RRA => self.rra(mode),
            Instruction::SAX => self.sax(mode),
            Instruction::SBX => self.sbx(mode),
            Instruction::SHA => self.sha(mode),
            Instruction::SHS => self.shs(mode),
            Instruction::SHX => self.shx(mode),
            Instruction::SHY => self.shy(mode),
            Instruction::SLO => self.slo(mode),
            Instruction::SRE => self.sre(mode),
            Instruction::XXA => self.xxa(mode),
        }
    }

    fn shift_left(&mut self, shift_style: ShiftStyle, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);

        let m = self.read_value(location);

        self.write_value(
            location,
            (m.0 << 1)
                | if shift_style == ShiftStyle::Rotate && self.read_flag(Flag::Carry) {
                    1
                } else {
                    0
                },
        );
        self.set_flag(Flag::Carry, m.0 & SIGN_BIT != 0);
        (m.1, Branch::NotTaken)
    }

    fn shift_right(&mut self, shift_style: ShiftStyle, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let m = self.read_value(location);

        self.write_value(
            location,
            (m.0 >> 1)
                | if shift_style == ShiftStyle::Rotate && self.read_flag(Flag::Carry) {
                    SIGN_BIT
                } else {
                    0
                },
        );
        self.set_flag(Flag::Carry, m.0 & 1 != 0);
        (m.1, Branch::NotTaken)
    }

    fn binary(&mut self, f: fn(u8, u8) -> u8, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let m = self.read_value(location);
        self.write_value(Location::A, f(self.a, m.0));
        (m.1, Branch::NotTaken)
    }

    fn unary(&mut self, f: fn(u8) -> u8, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);

        let m = self.read_value(location);

        self.write_value(location, f(m.0));
        (m.1, Branch::NotTaken)
    }

    fn adc(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let m_orig = self.read_value(location);
        let m = m_orig.0;
        let carry = if self.read_flag(Flag::Carry) { 1 } else { 0 };

        let mut tmp: u16 = (self.a as u16).wrapping_add(m as u16).wrapping_add(carry);

        self.set_flag(Flag::Zero, tmp & LOW_BYTE_MASK == 0);

        if self.read_flag(Flag::Decimal) && self.bcd_enabled() {
            // if the sum of the lowest digits is > 9, then
            // add 6 to fix the lowest digit back to bcd
            let fixup = if (self.a & 0x0F)
                .wrapping_add(m & 0x0F)
                .wrapping_add(if self.read_flag(Flag::Carry) { 1 } else { 0 })
                > 9
            {
                6
            } else {
                0
            };

            tmp = tmp.wrapping_add(fixup);

            // in BCD, Negative and Overflow are set weirdly - as if this was binary
            // math up to this point
            self.set_flag(Flag::Negative, tmp & (SIGN_BIT as u16) != 0);
            self.set_flag(
                Flag::Overflow,
                (self.a ^ m) & SIGN_BIT == 0 && (self.a as u16 ^ tmp) & SIGN_BIT as u16 != 0,
            );

            // if the highest digit is too high for bcd, fix it
            // note that this fixup happens after negative and overflow
            // have been set. shrug emoji
            if tmp > 0x99 {
                tmp = tmp.wrapping_add(96);
            }

            // carry flag is actually right
            self.set_flag(Flag::Carry, tmp > 0x99);
        } else {
            self.set_flag(Flag::Negative, tmp & (SIGN_BIT as u16) != 0);
            // for overflow, pretend all the values are signed.
            // if the sign bits of the inputs are the same as each other but
            // different from the sign bit of the output then there was overflow
            self.set_flag(
                Flag::Overflow,
                (self.a ^ m) & SIGN_BIT == 0 && (self.a as u16 ^ tmp) & SIGN_BIT as u16 != 0,
            );

            self.set_flag(Flag::Carry, tmp > 0xFF);
        };

        self.write_value_without_flags(Location::A, (tmp & 0xFF) as u8);
        (m_orig.1, Branch::NotTaken)
    }

    fn sbc(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let m_orig = self.read_value(location);
        let m = m_orig.0;
        let borrow = if self.read_flag(Flag::Carry) { 0 } else { 1 };

        let mut tmp: u16 = (self.a as u16).wrapping_sub(m as u16).wrapping_sub(borrow);

        self.set_flag(Flag::Negative, tmp & (SIGN_BIT as u16) != 0);
        self.set_flag(Flag::Zero, tmp & LOW_BYTE_MASK == 0);
        self.set_flag(
            Flag::Overflow,
            (self.a as u16 ^ tmp) & SIGN_BIT as u16 != 0 && (self.a ^ m) & SIGN_BIT != 0,
        );

        if self.read_flag(Flag::Decimal) && self.bcd_enabled() {
            let fixup = if (self.a & 0x0F)
                < (m & 0x0F).wrapping_add(if self.read_flag(Flag::Carry) { 0 } else { 1 })
            {
                6
            } else {
                0
            };

            tmp = tmp.wrapping_sub(fixup);
            if tmp > 0x99 {
                tmp = tmp.wrapping_sub(0x60);
            }
        };

        self.set_flag(Flag::Carry, tmp < 0x0100);

        self.write_value_without_flags(Location::A, (tmp & 0xFF) as u8);
        (m_orig.1, Branch::NotTaken)
    }

    fn compare(&mut self, mode: Mode, register: u8) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let m = self.read_value(location);
        self.set_flag(Flag::Zero, register == m.0);
        self.set_flag(Flag::Carry, register >= m.0);

        let tmp = register.wrapping_sub(m.0);

        self.set_flag(Flag::Negative, (tmp & SIGN_BIT) != 0);
        (m.1, Branch::NotTaken)
    }

    fn branch(&mut self, mode: Mode, branch_type: BranchType) -> (PageBoundary, Branch) {
        let condition = match branch_type {
            BranchType::CC => !self.read_flag(Flag::Carry),
            BranchType::CS => self.read_flag(Flag::Carry),
            BranchType::NE => !self.read_flag(Flag::Zero),
            BranchType::EQ => self.read_flag(Flag::Zero),
            BranchType::PL => !self.read_flag(Flag::Negative),
            BranchType::MI => self.read_flag(Flag::Negative),
            BranchType::VC => !self.read_flag(Flag::Overflow),
            BranchType::VS => self.read_flag(Flag::Overflow),
            BranchType::JMP => true,
            BranchType::JSR => true,
        };
        let starting_pc = self.pc.wrapping_sub(1);
        let location = self.get_location(mode);

        if branch_type == BranchType::JSR {
            self.push_word(self.pc.wrapping_sub(1));
        }

        if let Location::Addr(_, addr) = location {
            if condition {
                // a jmp back to itself is the most common form of
                // trap used in tests. Another common one is a conditional
                // branch relative back to the same spot. Detecting
                // these conditions should make it easier to run tests to completion
                if starting_pc == addr {
                    self.trapped = true;
                }
                self.pc = addr;
                (location.page_boundary(), Branch::Taken)
            } else {
                (PageBoundary::NotCrossed, Branch::NotTaken)
            }
        } else {
            unreachable!("Branching to unsupported location {:?}", location)
        }
    }

    fn transfer(&mut self, in_mode: Mode, out_mode: Mode) -> (PageBoundary, Branch) {
        let in_location = self.get_location(in_mode);
        let value = self.read_value(in_location);
        let out_location = self.get_location(out_mode);
        self.write_value(out_location, value.0);
        (value.1, Branch::NotTaken)
    }

    fn transfer_without_flags(&mut self, in_mode: Mode, out_mode: Mode) -> (PageBoundary, Branch) {
        let in_location = self.get_location(in_mode);
        let value = self.read_value(in_location);
        let out_location = self.get_location(out_mode);
        self.write_value_without_flags(out_location, value.0);
        (value.1, Branch::NotTaken)
    }

    fn read_value(&mut self, location: Location) -> (u8, PageBoundary) {
        let boundary = &location.page_boundary();
        let value = match location {
            Location::A => self.a,
            Location::X => self.x,
            Location::Y => self.y,
            Location::SP => self.sp,
            Location::Status => self.status,
            Location::Addr(_, addr) => {
                let value = self.read_bus_byte(addr);
                self.monitor.read_data_byte(addr, value).unwrap();
                value
            }
            Location::Imm => self.fetch_byte(),
            Location::Imp => {
                unreachable!("Attempt to read value on implied addressing mode")
            }
        };

        (value, *boundary)
    }

    fn write_value(&mut self, location: Location, value: u8) -> PageBoundary {
        let page_boundary = self.write_value_without_flags(location, value);

        self.set_flag(Flag::Negative, value & SIGN_BIT != 0);
        self.set_flag(Flag::Zero, value == 0);

        page_boundary
    }

    fn write_value_without_flags(&mut self, location: Location, value: u8) -> PageBoundary {
        let boundary = &location.page_boundary();
        match location {
            Location::A => self.a = value,
            Location::X => self.x = value,
            Location::Y => self.y = value,
            Location::SP => self.sp = value,
            Location::Status => self.status = value,
            Location::Addr(_, addr) => {
                let old = self.write_bus_byte(addr, value);
                self.monitor.read_data_byte(addr, old).unwrap();
            }
            Location::Imm => {
                unreachable!("Attempt to write value on immediate addressing mode")
            }
            Location::Imp => {
                unreachable!("Attempt to write value on implied addressing mode")
            }
        }

        *boundary
    }

    fn set_flag(&mut self, flag: Flag, value: bool) -> (PageBoundary, Branch) {
        if value {
            self.status |= flag;
        } else {
            self.status &= !flag;
        }
        (PageBoundary::NotCrossed, Branch::NotTaken)
    }

    fn read_flag(&self, flag: Flag) -> bool {
        (self.status & flag as u8) != 0
    }

    fn get_location(&mut self, mode: Mode) -> Location {
        match mode {
            Mode::Abs => {
                let addr = self.fetch_word();
                Location::Addr(addr, addr)
            }
            Mode::AbsX => {
                let orig_addr = self.fetch_word();
                let addr = orig_addr.wrapping_add(self.x as u16);
                Location::Addr(orig_addr, addr)
            }
            Mode::AbsY => {
                let orig_addr = self.fetch_word();
                let addr = orig_addr.wrapping_add(self.y as u16);
                Location::Addr(orig_addr, addr)
            }
            Mode::Status => Location::Status,
            Mode::A => Location::A,
            Mode::X => Location::X,
            Mode::Y => Location::Y,
            Mode::SP => Location::SP,
            Mode::Imm => Location::Imm,
            Mode::Imp => Location::Imp,
            Mode::AbsInd => {
                let orig_addr = self.fetch_word();
                let lb = self.read_value(Location::Addr(orig_addr, orig_addr)).0;
                // the 6502 has a bug where instead of incrementing the full address before
                // reading the the next byte, it only increments the low byte of the address.
                // Weird? yes. Hence never "JMP ($xxFF)" because what happens will be weird as it
                // will read the address from $xxFF and then $xx00
                let addr_high = CPU::high_byte(orig_addr);
                let addr_low = CPU::low_byte(orig_addr).wrapping_add(1);
                let hb = self
                    .read_value(Location::Addr(
                        CPU::to_word(addr_low, addr_high),
                        CPU::to_word(addr_low, addr_high),
                    ))
                    .0;
                let addr = CPU::to_word(lb, hb);
                Location::Addr(orig_addr, addr)
            }
            Mode::IndX => {
                let zp_addr_low = self.fetch_byte().wrapping_add(self.x) as u16;
                let zp_addr_high = (zp_addr_low as u8).wrapping_add(1) as u16;
                let addr_low = self.read_value(Location::Addr(zp_addr_low, zp_addr_low)).0;
                let addr_high = self
                    .read_value(Location::Addr(zp_addr_high, zp_addr_high))
                    .0;
                let addr = CPU::to_word(addr_low, addr_high);
                Location::Addr(addr, addr)
            }
            Mode::IndY => {
                let zp_addr_low = self.fetch_byte() as u16;
                let zp_addr_high = (zp_addr_low as u8).wrapping_add(1) as u16;
                let orig_addr_low = self.read_value(Location::Addr(zp_addr_low, zp_addr_low)).0;
                let orig_addr_high = self
                    .read_value(Location::Addr(zp_addr_high, zp_addr_high))
                    .0;
                let orig_addr = CPU::to_word(orig_addr_low, orig_addr_high);
                let addr = orig_addr.wrapping_add(self.y as u16);
                Location::Addr(orig_addr, addr)
            }
            Mode::Rel => {
                let offset = self.fetch_byte();
                let new_pc = if offset & SIGN_BIT != 0 {
                    let offset = !offset + 1;
                    self.pc.wrapping_sub(offset as u16)
                } else {
                    self.pc.wrapping_add(offset as u16)
                };

                Location::Addr(self.pc, new_pc)
            }

            Mode::Zp => {
                let addr = self.fetch_byte() as u16;
                Location::Addr(addr, addr)
            }
            Mode::Zpx => {
                let addr = (self.fetch_byte().wrapping_add(self.x)) as u16;
                Location::Addr(addr, addr)
            }
            Mode::Zpy => {
                let addr = (self.fetch_byte().wrapping_add(self.y)) as u16;
                Location::Addr(addr, addr)
            }
        }
    }

    pub fn read_bus_byte(&mut self, addr: u16) -> u8 {
        self.bus.read(addr)
    }

    pub fn read_bus_word(&mut self, addr: u16) -> u16 {
        let lb = self.read_bus_byte(addr);
        let hb = self.read_bus_byte(addr.wrapping_add(1));
        CPU::to_word(lb, hb)
    }

    pub fn write_bus_byte(&mut self, addr: u16, data: u8) -> u8 {
        self.bus.write(addr, data)
    }

    fn low_byte(value: u16) -> u8 {
        (value & LOW_BYTE_MASK) as u8
    }

    fn high_byte(value: u16) -> u8 {
        (value >> 8) as u8
    }

    fn to_word(low_byte: u8, high_byte: u8) -> u16 {
        (high_byte as u16) << 8 | (low_byte as u16)
    }

    fn push(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let m = self.read_value(location);
        self.push_byte(m.0);
        (m.1, Branch::NotTaken)
    }

    fn pop(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let value = self.pop_byte()
            | if mode == Mode::Status {
                // in the real 6502, break and unused aren't
                // actually flags. But when pushed on the
                // stack Unused is always set and break is
                // set except on NMI or IRQ. The easiest way
                // to make that happen is to set Break and Unused
                // flags on power up and on pop
                Flag::Break | Flag::Unused
            } else {
                0
            };
        let location = self.get_location(mode);
        let page_boundary = if location == Location::Status {
            self.write_value_without_flags(location, value)
        } else {
            self.write_value(location, value)
        };

        (page_boundary, Branch::NotTaken)
    }

    fn rti(&mut self) -> (PageBoundary, Branch) {
        self.pop(Mode::Status);
        self.pc = self.pop_word();
        (PageBoundary::NotCrossed, Branch::NotTaken)
    }

    fn rts(&mut self) -> (PageBoundary, Branch) {
        self.pc = self.pop_word().wrapping_add(1);
        (PageBoundary::NotCrossed, Branch::NotTaken)
    }

    fn test_bits(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let m = self.read_value(location);
        let result = self.a & m.0;
        self.set_flag(Flag::Negative, (m.0 & Flag::Negative) != 0);
        self.set_flag(Flag::Overflow, (m.0 & Flag::Overflow) != 0);
        self.set_flag(Flag::Zero, result == 0);
        (m.1, Branch::NotTaken)
    }

    fn bcd_enabled(&self) -> bool {
        match self.cpu_type {
            CPUType::RP2A03 => false,
            CPUType::MOS6502 => true,
        }
    }

    // all the unofficial nops need to have side effects
    fn nop(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        match location {
            Location::A => (),
            Location::X => (),
            Location::Y => (),
            Location::SP => (),
            Location::Status => (),
            Location::Addr(_, _) => {
                self.read_value(location);
            }
            Location::Imm => {
                self.read_value(location);
            }
            Location::Imp => (),
        }
        (location.page_boundary(), Branch::NotTaken)
    }

    fn anc(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let result = self.binary(|a, m| a & m, mode);
        self.set_flag(Flag::Carry, self.a & SIGN_BIT != 0);
        result
    }

    fn arr(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let operand = self.read_value(location);
        let anded = self.a & operand.0;
        let value = (anded >> 1)
            | if self.read_flag(Flag::Carry) {
                SIGN_BIT
            } else {
                0
            };

        self.write_value(Location::A, value);
        self.set_flag(
            Flag::Overflow,
            ((value & 0b010000000) >> 1) != value & 0b00100000,
        );
        if self.bcd_enabled() && self.read_flag(Flag::Decimal) {
            self.set_flag(
                Flag::Carry,
                (operand.0 & 0xF0).wrapping_add(operand.0 & 0x10) > 0x50,
            );
        } else {
            self.set_flag(Flag::Carry, value & 0b01000000 != 0);
        }

        (location.page_boundary(), Branch::NotTaken)
    }

    fn asr(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let orig = self.a & self.read_value(location).0;
        let value = orig >> 1;

        self.set_flag(Flag::Negative, false);
        self.set_flag(Flag::Carry, orig & 1 != 0);
        self.set_flag(Flag::Zero, value == 0);

        self.a = value;
        (location.page_boundary(), Branch::NotTaken)
    }

    fn dcp(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let value = self.read_value(location).0.wrapping_sub(1);

        let diff = self.a.wrapping_sub(value);

        self.set_flag(Flag::Negative, diff & SIGN_BIT != 0);
        self.set_flag(Flag::Zero, diff == 0);
        self.set_flag(Flag::Carry, value <= self.a);

        (
            self.write_value_without_flags(location, value),
            Branch::NotTaken,
        )
    }

    fn isc(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let m = self.read_value(location).0.wrapping_add(1);

        let borrow = if self.read_flag(Flag::Carry) { 0 } else { 1 };

        let tmp: u16 = (self.a as u16).wrapping_sub(m as u16).wrapping_sub(borrow);

        self.set_flag(Flag::Negative, tmp & (SIGN_BIT as u16) != 0);
        self.set_flag(Flag::Zero, tmp & LOW_BYTE_MASK == 0);
        self.set_flag(
            Flag::Overflow,
            (self.a as u16 ^ tmp) & SIGN_BIT as u16 != 0 && (self.a ^ m) & SIGN_BIT != 0,
        );

        self.set_flag(Flag::Carry, tmp < 0x0100);

        self.write_value_without_flags(Location::A, (tmp & 0xFF) as u8);
        (
            self.write_value_without_flags(location, m),
            Branch::NotTaken,
        )
    }

    fn jam(&mut self) -> (PageBoundary, Branch) {
        self.jammed = true;
        (PageBoundary::NotCrossed, Branch::NotTaken)
    }

    fn las(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let v1 = self.read_value(location);
        let value = v1.0 & self.sp;
        self.write_value(Location::A, value);
        self.x = value;
        self.sp = value;
        (location.page_boundary(), Branch::NotTaken)
    }

    fn lax(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let value = self.read_value(location);
        self.write_value(Location::A, value.0);
        self.write_value_without_flags(Location::X, value.0);
        (location.page_boundary(), Branch::NotTaken)
    }

    fn sax(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let value = self.a & self.x;
        let location = self.get_location(mode);
        (
            self.write_value_without_flags(location, value),
            Branch::NotTaken,
        )
    }

    fn sha(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);

        let value = CPU::h_plus_1(location);
        let data = self.a & self.x & value;
        (
            self.write_value_without_flags(location, data),
            Branch::NotTaken,
        )
    }

    fn h_plus_1(location: Location) -> u8 {
        match location {
            Location::Addr(orig, _) => ((orig >> 8) as u8).wrapping_add(1),
            _ => unreachable!("SHA, SHX, SHY, or TAS with invalid location {:?}", location),
        }
    }

    fn rla(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let orig = self.read_value(location).0;
        let m = (orig << 1) | if self.read_flag(Flag::Carry) { 1 } else { 0 };

        self.set_flag(Flag::Carry, orig & SIGN_BIT != 0);
        self.write_value(Location::A, self.a & m);
        (
            self.write_value_without_flags(location, m),
            Branch::NotTaken,
        )
    }

    fn rra(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let orig = self.read_value(location).0;
        let m = (orig >> 1)
            | if self.read_flag(Flag::Carry) {
                SIGN_BIT
            } else {
                0
            };

        self.set_flag(Flag::Carry, orig & 1 != 0);

        let value = (self.a as u16)
            .wrapping_add(m as u16)
            .wrapping_add((orig & 1) as u16);

        if self.bcd_enabled() && self.read_flag(Flag::Decimal) {
            self.set_flag(Flag::Carry, value > 0x99);
        } else {
            self.set_flag(Flag::Carry, value > 0xFF);
        }

        self.set_flag(
            Flag::Overflow,
            (self.a ^ m) & SIGN_BIT == 0 && (self.a as u16 ^ value) & SIGN_BIT as u16 != 0,
        );

        self.write_value(Location::A, (value & 0xFF) as u8);
        (
            self.write_value_without_flags(location, m),
            Branch::NotTaken,
        )
    }

    fn sbx(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let value = (self.a & self.x).wrapping_sub(self.read_value(location).0);

        self.set_flag(Flag::Carry, (value & SIGN_BIT) == 0);

        (self.write_value(Location::X, value), Branch::NotTaken)
    }

    fn shs(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let v1 = self.a & self.x;
        self.sp = v1;
        let location = self.get_location(mode);
        let value = v1 & CPU::h_plus_1(location);
        (
            self.write_value_without_flags(location, value),
            Branch::NotTaken,
        )
    }

    fn shx(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let value = self.x & CPU::h_plus_1(location);
        (
            self.write_value_without_flags(location, value),
            Branch::NotTaken,
        )
    }

    fn shy(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let value = self.y & CPU::h_plus_1(location);
        (
            self.write_value_without_flags(location, value),
            Branch::NotTaken,
        )
    }

    fn slo(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let orig = self.read_value(location).0;
        let m = orig << 1;

        self.write_value_without_flags(location, m);
        self.set_flag(Flag::Carry, orig & SIGN_BIT != 0);
        self.write_value(Location::A, self.a | m);
        (location.page_boundary(), Branch::NotTaken)
    }

    fn sre(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let orig = self.read_value(location).0;
        let m = orig >> 1;

        self.write_value_without_flags(location, m);
        self.set_flag(Flag::Carry, orig & 1 != 0);
        self.write_value(Location::A, self.a ^ m);
        (location.page_boundary(), Branch::NotTaken)
    }

    fn xxa(&mut self, mode: Mode) -> (PageBoundary, Branch) {
        let location = self.get_location(mode);
        let m = self.read_value(location).0;
        let value = (self.a | 0xEE) & self.x & m;
        self.write_value(location, value);
        (location.page_boundary(), Branch::NotTaken)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CPUType {
    RP2A03,
    MOS6502,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Location {
    A,
    X,
    Y,
    SP,
    Status,
    Addr(u16, u16),
    Imm,
    Imp,
}

impl Location {
    fn page_boundary(&self) -> PageBoundary {
        match self {
            Location::A => PageBoundary::NotCrossed,
            Location::X => PageBoundary::NotCrossed,
            Location::Y => PageBoundary::NotCrossed,
            Location::SP => PageBoundary::NotCrossed,
            Location::Status => PageBoundary::NotCrossed,
            Location::Addr(addr1, addr2) => {
                if (addr1 & HIGH_BYTE_MASK) != (addr2 & HIGH_BYTE_MASK) {
                    PageBoundary::Crossed
                } else {
                    PageBoundary::NotCrossed
                }
            }
            Location::Imm => PageBoundary::NotCrossed,
            Location::Imp => PageBoundary::NotCrossed,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum BranchType {
    CC,
    CS,
    EQ,
    MI,
    NE,
    PL,
    VC,
    VS,
    JMP,
    JSR,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Branch {
    NotTaken,
    Taken,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum PageBoundary {
    NotCrossed,
    Crossed,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ShiftStyle {
    ShiftOff,
    Rotate,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Interrupt {
    BRK,
    IRQ,
    NMI,
    RST,
}

#[cfg(test)]
pub fn create_test_configuration() -> (CPU, Rc<RefCell<crate::ram::RAM>>) {
    use crate::ram::RAM;

    let mut cpu = CPU::default();
    let mem = Rc::new(RefCell::new(RAM::new(0x0000, 0xFFFF, 0xFFFF)));
    cpu.add_device(mem.clone());
    (cpu, mem)
}
