// https://c74project.com/microcode/

#[cfg(test)]
mod test {
    mod functional_test;
    mod test_addressing_modes;
    mod test_clock_and_interrupts;
    mod test_decode;
    mod test_instructions;
}
mod decode;

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::device::BusDevice;

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
    halted: bool,
    trapped: bool,
    bus: Vec<Box<dyn BusDevice>>,
    // internal state of executing instuciton
    instruction: Instruction,
    mode: Mode,
    remaining_cycles: u8,
    extra_cycles: u8,
    cycle_on_page_boundary: bool,
    interrupt: Option<Interrupt>,
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
            status: Flag::Break | Flag::Unused | Flag::Zero,
            sp: 0,
            pc: 0,
            halted: true,
            trapped: false,
            bus: Vec::new(),
            instruction: Instruction::NOP,
            mode: Mode::Imp,
            remaining_cycles: 0,
            extra_cycles: 0,
            cycle_on_page_boundary: false,
            interrupt: None,
        }
    }

    pub fn reset(&mut self) {
        // while other interrupts will wait for the current instruction
        // to complete, reset starts on the next clock
        self.remaining_cycles = 0;
        self.extra_cycles = 0;
        self.halted = false;
        self.interrupt = Some(Interrupt::RST);
        self.trapped = false;
    }

    pub fn nmi(&mut self) {
        if self.interrupt != Some(Interrupt::RST) {
            self.interrupt = Some(Interrupt::NMI);
        }
        self.trapped = false;
    }

    pub fn irq(&mut self) {
        if self.interrupt.is_none() && !self.read_flag(Flag::InterruptDisable) {
            self.interrupt = Some(Interrupt::IRQ);
            self.trapped = false;
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

    pub fn clock(&mut self) {
        if self.halted {
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
                    let op = self.fetch_byte();
                    crate::cpu::decode::decode(op)
                }
            };

            self.instruction = instruction;
            self.mode = mode;
            self.remaining_cycles = cycles - 1;
            self.extra_cycles = 0;
            self.cycle_on_page_boundary = cycle_on_boundary;
        }
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
            self.set_flag(Flag::InterruptDisable, true);
        }

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
            Instruction::NOP => (PageBoundary::NotCrossed, Branch::NotTaken),
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
        self.write_value((Location::A, PageBoundary::NotCrossed), f(self.a, m.0));
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

        self.write_value_without_flags((Location::A, PageBoundary::NotCrossed), (tmp & 0xFF) as u8);
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

        self.write_value_without_flags((Location::A, PageBoundary::NotCrossed), (tmp & 0xFF) as u8);
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

        if let Location::Addr(addr) = location.0 {
            if condition {
                // a jmp back to itself is the most common form of
                // trap used in tests. Another common one is a conditional
                // branch relative back to the same spot. Detecting
                // these conditions should make it easier to run tests to completion
                if starting_pc == addr {
                    self.trapped = true;
                }
                self.pc = addr
            }
            (
                location.1,
                if condition {
                    Branch::Taken
                } else {
                    Branch::NotTaken
                },
            )
        } else {
            unreachable!("Branching to unsupported location {:?}", location.0)
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

    fn read_value(&mut self, location: (Location, PageBoundary)) -> (u8, PageBoundary) {
        let value = match location.0 {
            Location::A => self.a,
            Location::X => self.x,
            Location::Y => self.y,
            Location::SP => self.sp,
            Location::Status => self.status,
            Location::Addr(addr) => self.read_bus_byte(addr),
            Location::Imm => self.fetch_byte(),
            Location::Imp => unreachable!("Attempt to read value on implied addressing mode"),
        };

        (value, location.1)
    }

    fn write_value(&mut self, location: (Location, PageBoundary), value: u8) -> PageBoundary {
        let page_boundary = self.write_value_without_flags(location, value);

        self.set_flag(Flag::Negative, value & SIGN_BIT != 0);
        self.set_flag(Flag::Zero, value == 0);

        page_boundary
    }

    fn write_value_without_flags(
        &mut self,
        location: (Location, PageBoundary),
        value: u8,
    ) -> PageBoundary {
        match location.0 {
            Location::A => self.a = value,
            Location::X => self.x = value,
            Location::Y => self.y = value,
            Location::SP => self.sp = value,
            Location::Status => self.status = value,
            Location::Addr(addr) => self.write_bus_byte(addr, value),
            Location::Imm => unreachable!("Attempt to write value on immediate addressing mode"),
            Location::Imp => unreachable!("Attempt to write value on implied addressing mode"),
        }

        location.1
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

    fn get_location(&mut self, mode: Mode) -> (Location, PageBoundary) {
        match mode {
            Mode::Abs => {
                let addr = self.fetch_word();
                (Location::Addr(addr), PageBoundary::NotCrossed)
            }
            Mode::AbsX => {
                let orig_addr = self.fetch_word();
                let addr = orig_addr.wrapping_add(self.x as u16);

                (Location::Addr(addr), CPU::page_changed(orig_addr, addr))
            }
            Mode::AbsY => {
                let orig_addr = self.fetch_word();
                let addr = orig_addr.wrapping_add(self.y as u16);

                (Location::Addr(addr), CPU::page_changed(orig_addr, addr))
            }
            Mode::Status => (Location::Status, PageBoundary::NotCrossed),
            Mode::A => (Location::A, PageBoundary::NotCrossed),
            Mode::X => (Location::X, PageBoundary::NotCrossed),
            Mode::Y => (Location::Y, PageBoundary::NotCrossed),
            Mode::SP => (Location::SP, PageBoundary::NotCrossed),
            Mode::Imm => (Location::Imm, PageBoundary::NotCrossed),
            Mode::Imp => (Location::Imp, PageBoundary::NotCrossed),
            Mode::AbsInd => {
                let orig_addr = self.fetch_word();
                let lb = self.read_bus_byte(orig_addr);
                // the 6502 has a bug where instead of incrementing the full address before
                // reading the the next byte, it only increments the low byte of the address.
                // Weird? yes. Hence never "JMP ($xxFF)" because what happens will be weird as it
                // will read the address from $xxFF and then $xx00
                let addr_high = CPU::high_byte(orig_addr);
                let addr_low = CPU::low_byte(orig_addr).wrapping_add(1);
                let hb = self.read_bus_byte(CPU::to_word(addr_low, addr_high));
                let addr = CPU::to_word(lb, hb);

                (Location::Addr(addr), CPU::page_changed(orig_addr, addr))
            }
            Mode::IndX => {
                let zp_addr = self.fetch_byte().wrapping_add(self.x);
                let addr = self.read_bus_word(zp_addr as u16);

                (Location::Addr(addr), PageBoundary::NotCrossed)
            }
            Mode::IndY => {
                let zp_addr = self.fetch_byte() as u16;
                let orig_addr = self.read_bus_word(zp_addr);
                let addr = orig_addr.wrapping_add(self.y as u16);

                (Location::Addr(addr), CPU::page_changed(orig_addr, addr))
            }
            Mode::Rel => {
                let offset = self.fetch_byte();
                let new_pc = if offset & SIGN_BIT != 0 {
                    let offset = !offset + 1;
                    self.pc.wrapping_sub(offset as u16)
                } else {
                    self.pc.wrapping_add(offset as u16)
                };

                (Location::Addr(new_pc), CPU::page_changed(self.pc, new_pc))
            }
            Mode::Zp => {
                let addr = self.fetch_byte() as u16;
                (Location::Addr(addr), PageBoundary::NotCrossed)
            }
            Mode::Zpx => {
                let addr = (self.fetch_byte().wrapping_add(self.x)) as u16;
                (Location::Addr(addr), PageBoundary::NotCrossed)
            }
            Mode::Zpy => {
                let addr = (self.fetch_byte().wrapping_add(self.y)) as u16;
                (Location::Addr(addr), PageBoundary::NotCrossed)
            }
        }
    }

    fn page_changed(addr1: u16, addr2: u16) -> PageBoundary {
        if (addr1 & HIGH_BYTE_MASK) != (addr2 & HIGH_BYTE_MASK) {
            PageBoundary::Crossed
        } else {
            PageBoundary::NotCrossed
        }
    }

    pub fn add_bus_device(&mut self, device: Box<dyn BusDevice>) {
        self.bus.push(device)
    }

    pub fn read_bus_byte(&self, addr: u16) -> u8 {
        for device in &self.bus {
            if let Some(data) = device.read(addr) {
                return data;
            }
        }
        0
    }

    pub fn read_bus_word(&self, addr: u16) -> u16 {
        let lb = self.read_bus_byte(addr);
        let hb = self.read_bus_byte(addr.wrapping_add(1));
        CPU::to_word(lb, hb)
    }

    pub fn write_bus_byte(&mut self, addr: u16, data: u8) {
        for device in &mut self.bus {
            if device.write(addr, data) {
                return;
            }
        }
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
        let page_boundary = if location.0 == Location::Status {
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

    pub fn stuck(&self) -> bool {
        self.halted || self.trapped
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CPUType {
    RP2A03,
    MOS6502,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    ADC,
    AND,
    ASL,
    BCC,
    BCS,
    BEQ,
    BIT,
    BMI,
    BNE,
    BPL,
    BRK,
    BVC,
    BVS,
    CLC,
    CLD,
    CLI,
    CLV,
    CMP,
    CPX,
    CPY,
    DEC,
    DEX,
    DEY,
    EOR,
    INC,
    INX,
    INY,
    JMP,
    JSR,
    LDA,
    LDX,
    LDY,
    LSR,
    NOP,
    ORA,
    PHA,
    PHP,
    PLA,
    PLP,
    ROL,
    ROR,
    RTI,
    RTS,
    SBC,
    SEC,
    SED,
    SEI,
    STA,
    STX,
    STY,
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
    // the following aren't real instructions
    // they're pseudo instructions used to
    // implement interrupt logic
    RST,
    IRQ,
    NMI,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    Abs,
    AbsX,
    AbsY,
    A,
    SP,
    Status,
    Imm,
    Imp,
    AbsInd,
    IndX,
    IndY,
    Rel,
    Zp,
    Zpx,
    Zpy,
    // the following aren't real addressing modes of the 6502
    // they are used internally to make implementation more uniform
    X,
    Y,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Location {
    A,
    X,
    Y,
    SP,
    Status,
    Addr(u16),
    Imm,
    Imp,
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

#[repr(u8)]
enum Flag {
    Carry = 0b00000001,
    Zero = 0b00000010,
    InterruptDisable = 0b00000100,
    Decimal = 0b00001000,
    Break = 0b00010000,
    Unused = 0b00100000,
    Overflow = 0b01000000,
    Negative = 0b10000000,
}

impl BitOr<Self> for Flag {
    type Output = u8;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as u8 | rhs as u8
    }
}

impl BitOr<u8> for Flag {
    type Output = u8;

    fn bitor(self, rhs: u8) -> Self::Output {
        self as u8 | rhs
    }
}

impl BitOr<Flag> for u8 {
    type Output = u8;

    fn bitor(self, rhs: Flag) -> Self::Output {
        self | rhs as u8
    }
}

impl BitOrAssign<Flag> for u8 {
    fn bitor_assign(&mut self, rhs: Flag) {
        *self |= rhs as u8
    }
}

impl BitAnd<Self> for Flag {
    type Output = u8;

    fn bitand(self, rhs: Self) -> Self::Output {
        self as u8 & rhs as u8
    }
}

impl BitAnd<u8> for Flag {
    type Output = u8;

    fn bitand(self, rhs: u8) -> Self::Output {
        self as u8 & rhs
    }
}

impl BitAnd<Flag> for u8 {
    type Output = u8;

    fn bitand(self, rhs: Flag) -> Self::Output {
        self & rhs as u8
    }
}

impl BitAndAssign<Flag> for u8 {
    fn bitand_assign(&mut self, rhs: Flag) {
        *self &= rhs as u8
    }
}

impl Not for Flag {
    type Output = u8;

    fn not(self) -> Self::Output {
        !(self as u8)
    }
}
