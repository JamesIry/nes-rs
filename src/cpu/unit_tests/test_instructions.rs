#![allow(clippy::field_reassign_with_default)]

use crate::cpu::flags::*;
use crate::cpu::instructions::Instruction::*;
use crate::cpu::instructions::Mode::*;
use crate::cpu::*;

#[test]
fn test_nop() {
    let mut cpu = CPU::default();
    cpu.status = 0;
    cpu.a = 0x42;
    cpu.x = 0x43;
    cpu.y = 0x44;
    cpu.pc = 0;
    cpu.sp = 0xFF;

    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(NOP, Imp)
    );
    assert_eq!(cpu.status, 0);
    assert_eq!(cpu.a, 0x42);
    assert_eq!(cpu.x, 0x43);
    assert_eq!(cpu.y, 0x44);
    assert_eq!(cpu.pc, 0);
    assert_eq!(cpu.sp, 0xFF);
}

#[test]
fn test_flags() {
    let mut cpu = CPU::default();
    cpu.status = 0;

    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(SEC, Imp)
    );
    assert!(cpu.read_flag(Flag::Carry));
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(CLC, Imp)
    );
    assert!(!cpu.read_flag(Flag::Carry));

    cpu.status |= Flag::Zero;
    assert!(cpu.read_flag(Flag::Zero));
    cpu.status &= !Flag::Zero;
    assert!(!cpu.read_flag(Flag::Zero));

    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(SEI, Imp)
    );
    assert!(cpu.read_flag(Flag::InterruptDisable));
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(CLI, Imp)
    );
    assert!(!cpu.read_flag(Flag::InterruptDisable));

    cpu.status |= Flag::Break;
    assert!(cpu.read_flag(Flag::Break));
    cpu.status &= !Flag::Break;
    assert!(!cpu.read_flag(Flag::Break));

    cpu.status |= Flag::Unused;
    assert!(cpu.read_flag(Flag::Unused));
    cpu.status &= !Flag::Unused;
    assert!(!cpu.read_flag(Flag::Unused));

    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(SED, Imp)
    );
    assert!(cpu.read_flag(Flag::Decimal));
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(CLD, Imp)
    );
    assert!(!cpu.read_flag(Flag::Decimal));

    cpu.status |= Flag::Overflow;
    assert!(cpu.read_flag(Flag::Overflow));
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(CLV, Imp)
    );
    assert!(!cpu.read_flag(Flag::Overflow));

    cpu.status |= Flag::Negative;
    assert!(cpu.read_flag(Flag::Negative));
    cpu.status &= !Flag::Negative;
    assert!(!cpu.read_flag(Flag::Negative));

    assert_eq!(cpu.status, 0);
}

#[test]
fn test_increments() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();
    cpu.status = 0;

    cpu.x = 0x44;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(INX, Imp)
    );
    assert_eq!(0x45, cpu.x);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(DEX, Imp)
    );
    assert_eq!(0x44, cpu.x);

    cpu.x = 0xFF;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(INX, Imp)
    );
    assert_eq!(0x00, cpu.x);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(DEX, Imp)
    );
    assert_eq!(0xFF, cpu.x);

    cpu.y = 0x46;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(INY, Imp)
    );
    assert_eq!(0x47, cpu.y);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(DEY, Imp)
    );
    assert_eq!(0x46, cpu.y);

    cpu.write_bus_byte(0, 0x01);
    cpu.write_bus_byte(1, 0xFF);
    cpu.pc = 0;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(INC, Zp)
    );
    assert_eq!(0x00, cpu.read_bus_byte(1));
    cpu.pc = 0;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(DEC, Zp)
    );
    assert_eq!(0xFF, cpu.read_bus_byte(1));
}

#[test]
fn test_transfers() {
    let mut cpu = CPU::default();
    cpu.status = 0;

    cpu.a = 0x42;
    cpu.x = 0x00;
    cpu.y = 0x00;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(TAX, Imp)
    );
    assert_eq!(0x42, cpu.x);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(TAY, Imp)
    );
    assert_eq!(0x42, cpu.y);

    cpu.sp = 0x43;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(TSX, Imp)
    );
    assert_eq!(0x43, cpu.x);

    cpu.x = 0x44;
    cpu.status = Flag::Negative | Flag::Zero;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(TXS, Imp)
    );
    assert_eq!(0x44, cpu.sp);
    // TXS doesn't modify flags
    assert_eq!(Flag::Negative | Flag::Zero, cpu.status);

    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(TXA, Imp)
    );
    assert_eq!(0x44, cpu.a);

    cpu.y = 0x45;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(TYA, Imp)
    );
    assert_eq!(0x45, cpu.a);
}

#[test]
fn test_load_store() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();

    cpu.status = 0;

    cpu.write_bus_byte(0, 0x7F);
    cpu.write_bus_byte(1, 0x00);
    cpu.write_bus_byte(2, 0xFF);

    cpu.pc = 0;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LDA, Imm)
    );
    assert_eq!(0x01, cpu.pc);
    assert_eq!(0x7F, cpu.a);
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(!cpu.read_flag(Flag::Negative));

    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LDA, Imm)
    );
    assert_eq!(0x02, cpu.pc);
    assert_eq!(0x00, cpu.a);
    assert!(cpu.read_flag(Flag::Zero));
    assert!(!cpu.read_flag(Flag::Negative));

    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LDA, Imm)
    );
    assert_eq!(0x03, cpu.pc);
    assert_eq!(0xFF, cpu.a);
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(cpu.read_flag(Flag::Negative));

    cpu.pc = 0;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LDX, Imm)
    );
    assert_eq!(0x7F, cpu.x);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LDX, Imm)
    );
    assert_eq!(0x00, cpu.x);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LDX, Imm)
    );
    assert_eq!(0xFF, cpu.x);

    cpu.pc = 0;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LDY, Imm)
    );
    assert_eq!(0x7F, cpu.y);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LDY, Imm)
    );
    assert_eq!(0x00, cpu.y);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LDY, Imm)
    );
    assert_eq!(0xFF, cpu.y);

    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x10);
    cpu.a = 0x83;
    cpu.status = Flag::Zero | Flag::Negative;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(STA, Zp)
    );
    assert_eq!(0x83, cpu.read_bus_byte(0x10));
    // stores don't modify flags
    assert_eq!(Flag::Zero | Flag::Negative, cpu.status);

    cpu.write_bus_byte(1, 0x11);
    cpu.x = 0x84;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(STX, Zp)
    );
    assert_eq!(0x84, cpu.read_bus_byte(0x11));

    cpu.write_bus_byte(2, 0x12);
    cpu.y = 0x85;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(STY, Zp)
    );
    assert_eq!(0x85, cpu.read_bus_byte(0x12));
}

#[test]
fn test_shift() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();

    cpu.set_flag(Flag::Carry, false);
    cpu.a = 0x42;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ASL, A)
    );
    assert!(!cpu.read_flag(Flag::Carry));
    assert_eq!(0x84, cpu.a);

    cpu.set_flag(Flag::Carry, true);
    cpu.a = 0b10010010;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ASL, A)
    );
    assert!(cpu.read_flag(Flag::Carry));
    assert_eq!(0b00100100, cpu.a);

    cpu.set_flag(Flag::Carry, false);
    cpu.a = 0x42;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ROL, A)
    );
    assert!(!cpu.read_flag(Flag::Carry));
    assert_eq!(0x84, cpu.a);

    cpu.set_flag(Flag::Carry, true);
    cpu.a = 0b10010010;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ROL, A)
    );
    assert!(cpu.read_flag(Flag::Carry));
    assert_eq!(0b00100101, cpu.a);

    cpu.write_bus_byte(0, 0x01);

    cpu.pc = 0;
    cpu.write_bus_byte(1, 0x42);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ASL, Zp)
    );
    assert_eq!(0x84, cpu.read_bus_byte(0x01));

    cpu.set_flag(Flag::Carry, false);
    cpu.a = 0x42;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LSR, A)
    );
    assert!(!cpu.read_flag(Flag::Carry));
    assert_eq!(0x21, cpu.a);

    cpu.set_flag(Flag::Carry, true);
    cpu.a = 0b10010011;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LSR, A)
    );
    assert!(cpu.read_flag(Flag::Carry));
    assert_eq!(0b01001001, cpu.a);

    cpu.set_flag(Flag::Carry, false);
    cpu.a = 0x42;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ROR, A)
    );
    assert!(!cpu.read_flag(Flag::Carry));
    assert_eq!(0x21, cpu.a);

    cpu.set_flag(Flag::Carry, true);
    cpu.a = 0b10010011;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ROR, A)
    );
    assert!(cpu.read_flag(Flag::Carry));
    assert_eq!(0b11001001, cpu.a);

    cpu.pc = 0;
    cpu.write_bus_byte(1, 0x42);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LSR, Zp)
    );
    assert_eq!(0x21, cpu.read_bus_byte(0x01));
}

#[test]
fn test_logic() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();

    cpu.write_bus_byte(0, 0x0F);

    cpu.pc = 0;
    cpu.a = 0b10010110;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(AND, Imm)
    );
    assert_eq!(0b00000110, cpu.a);

    cpu.pc = 0;
    cpu.a = 0b10010110;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(EOR, Imm)
    );
    assert_eq!(0b10011001, cpu.a);

    cpu.pc = 0;
    cpu.a = 0b10010110;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ORA, Imm)
    );
    assert_eq!(0b10011111, cpu.a);
}

#[test]
fn test_bit() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();

    cpu.write_bus_byte(0, 0x01);
    cpu.write_bus_byte(1, 0b11000000);

    cpu.pc = 0;
    cpu.a = 0b00000000;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BIT, Zp)
    );
    assert_eq!(0b00000000, cpu.a);
    assert!(cpu.read_flag(Flag::Negative));
    assert!(cpu.read_flag(Flag::Overflow));
    assert!(cpu.read_flag(Flag::Zero));

    cpu.pc = 0;
    cpu.a = 0b10000000;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BIT, Zp)
    );
    assert_eq!(0b10000000, cpu.a);
    assert!(cpu.read_flag(Flag::Negative));
    assert!(cpu.read_flag(Flag::Overflow));
    assert!(!cpu.read_flag(Flag::Zero));

    cpu.write_bus_byte(1, 0b01000000);
    cpu.pc = 0;
    cpu.a = 0b10000000;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BIT, Zp)
    );
    assert_eq!(0b10000000, cpu.a);
    assert!(!cpu.read_flag(Flag::Negative));
    assert!(cpu.read_flag(Flag::Overflow));
    assert!(cpu.read_flag(Flag::Zero));

    cpu.write_bus_byte(1, 0b10000000);
    cpu.pc = 0;
    cpu.a = 0b10000000;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BIT, Zp)
    );
    assert_eq!(0b10000000, cpu.a);
    assert!(cpu.read_flag(Flag::Negative));
    assert!(!cpu.read_flag(Flag::Overflow));
    assert!(!cpu.read_flag(Flag::Zero));
}

#[test]
fn test_compare() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();

    cpu.write_bus_byte(0, 0x42);

    cpu.pc = 0;
    cpu.a = 0x41;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(CMP, Imm)
    );
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(!cpu.read_flag(Flag::Carry));
    assert!(cpu.read_flag(Flag::Negative));

    cpu.pc = 0;
    cpu.a = 0x42;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(CMP, Imm)
    );
    assert!(cpu.read_flag(Flag::Zero));
    assert!(cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));

    cpu.pc = 0;
    cpu.a = 0x43;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(CMP, Imm)
    );
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));

    cpu.pc = 0;
    cpu.a = 0x00;
    cpu.x = 0x43;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(CPX, Imm)
    );
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));

    cpu.pc = 0;
    cpu.x = 0x00;
    cpu.y = 0x43;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(CPY, Imm)
    );
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));
}

#[test]
fn test_branch() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();

    cpu.status = 0;

    // check for backwards relative
    cpu.pc = 0x10;
    cpu.write_bus_byte(0x10, 0xFE);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::Taken),
        cpu.execute(BCC, Rel)
    );
    assert_eq!(0x0F, cpu.pc);

    // carry flag

    cpu.pc = 0x10;
    cpu.write_bus_byte(0x10, 0x02);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::Taken),
        cpu.execute(BCC, Rel)
    );
    assert_eq!(0x13, cpu.pc);

    cpu.set_flag(Flag::Carry, true);
    cpu.pc = 0x10;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BCC, Rel)
    );
    assert_eq!(0x11, cpu.pc);

    cpu.pc = 0x10;
    cpu.write_bus_byte(0x10, 0x02);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::Taken),
        cpu.execute(BCS, Rel)
    );
    assert_eq!(0x13, cpu.pc);

    cpu.set_flag(Flag::Carry, false);
    cpu.pc = 0x10;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BCS, Rel)
    );
    assert_eq!(0x11, cpu.pc);

    // zero flag

    cpu.pc = 0x10;
    cpu.write_bus_byte(0x10, 0x02);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::Taken),
        cpu.execute(BNE, Rel)
    );
    assert_eq!(0x13, cpu.pc);

    cpu.set_flag(Flag::Zero, true);
    cpu.pc = 0x10;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BNE, Rel)
    );
    assert_eq!(0x11, cpu.pc);

    cpu.pc = 0x10;
    cpu.write_bus_byte(0x10, 0x02);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::Taken),
        cpu.execute(BEQ, Rel)
    );
    assert_eq!(0x13, cpu.pc);

    cpu.set_flag(Flag::Zero, false);
    cpu.pc = 0x10;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BEQ, Rel)
    );
    assert_eq!(0x11, cpu.pc);

    // Negative flag

    cpu.pc = 0x10;
    cpu.write_bus_byte(0x10, 0x02);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::Taken),
        cpu.execute(BPL, Rel)
    );
    assert_eq!(0x13, cpu.pc);

    cpu.set_flag(Flag::Negative, true);
    cpu.pc = 0x10;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BPL, Rel)
    );
    assert_eq!(0x11, cpu.pc);

    cpu.pc = 0x10;
    cpu.write_bus_byte(0x10, 0x02);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::Taken),
        cpu.execute(BMI, Rel)
    );
    assert_eq!(0x13, cpu.pc);

    cpu.set_flag(Flag::Negative, false);
    cpu.pc = 0x10;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BMI, Rel)
    );
    assert_eq!(0x11, cpu.pc);

    // Overflow flag

    cpu.pc = 0x10;
    cpu.write_bus_byte(0x10, 0x02);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::Taken),
        cpu.execute(BVC, Rel)
    );
    assert_eq!(0x13, cpu.pc);

    cpu.set_flag(Flag::Overflow, true);
    cpu.pc = 0x10;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BVC, Rel)
    );
    assert_eq!(0x11, cpu.pc);

    cpu.pc = 0x10;
    cpu.write_bus_byte(0x10, 0x02);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::Taken),
        cpu.execute(BVS, Rel)
    );
    assert_eq!(0x13, cpu.pc);

    cpu.set_flag(Flag::Overflow, false);
    cpu.pc = 0x10;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BVS, Rel)
    );
    assert_eq!(0x11, cpu.pc);
}

#[test]
fn test_stack() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();
    cpu.sp = 0xFF;

    cpu.a = 0x42;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(PHA, Imp)
    );
    assert_eq!(0xFE, cpu.sp);
    assert_eq!(0x42, cpu.read_bus_byte(0x01FF));

    cpu.status = 0xFF;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(PHP, Imp)
    );
    assert_eq!(0xFD, cpu.sp);
    assert_eq!(0xFF, cpu.read_bus_byte(0x01FE));

    cpu.status = 0x00;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(PLP, Imp)
    );
    assert_eq!(0xFE, cpu.sp);
    assert_eq!(0xFF, cpu.status);

    cpu.a = 0x00;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(PLA, Imp)
    );
    assert_eq!(0xFF, cpu.sp);
    assert_eq!(0x42, cpu.a);

    // make sure wrap around works properly
    cpu.write_bus_byte(0x100, 0x43);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(PLA, Imp)
    );
    assert_eq!(0x00, cpu.sp);
    assert_eq!(0x43, cpu.a);

    cpu.a = 0x44;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(PHA, Imp)
    );
    assert_eq!(0xFF, cpu.sp);
    assert_eq!(0x44, cpu.read_bus_byte(0x0100));
}

#[test]
fn test_adc_binary() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();
    cpu.set_flag(Flag::Decimal, false);

    cpu.set_flag(Flag::Carry, false);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x21);
    cpu.a = 0x20;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ADC, Imm)
    );
    assert_eq!(0x41, cpu.a);
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(!cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));
    assert!(!cpu.read_flag(Flag::Overflow));

    cpu.set_flag(Flag::Carry, true);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x21);
    cpu.a = 0x20;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ADC, Imm)
    );
    assert_eq!(0x42, cpu.a);
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(!cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));
    assert!(!cpu.read_flag(Flag::Overflow));

    cpu.set_flag(Flag::Carry, false);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x01);
    cpu.a = 0xFF;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ADC, Imm)
    );
    assert_eq!(0x00, cpu.a);
    assert!(cpu.read_flag(Flag::Zero));
    assert!(cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));
    assert!(!cpu.read_flag(Flag::Overflow));

    cpu.set_flag(Flag::Carry, false);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x7F);
    cpu.a = 0x7F;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ADC, Imm)
    );
    assert_eq!(0xFE, cpu.a);
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(!cpu.read_flag(Flag::Carry));
    assert!(cpu.read_flag(Flag::Negative));
    assert!(cpu.read_flag(Flag::Overflow));
}

#[test]
fn test_adc_decimal() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();
    cpu.set_flag(Flag::Decimal, true);

    cpu.set_flag(Flag::Carry, false);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x21);
    cpu.a = 0x20;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ADC, Imm)
    );
    assert_eq!(0x41, cpu.a);
    assert!(!cpu.read_flag(Flag::Carry));

    cpu.set_flag(Flag::Carry, true);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x21);
    cpu.a = 0x20;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ADC, Imm)
    );
    assert_eq!(0x42, cpu.a);
    assert!(!cpu.read_flag(Flag::Carry));

    cpu.set_flag(Flag::Carry, false);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x99);
    cpu.a = 0x01;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ADC, Imm)
    );
    assert_eq!(0x00, cpu.a);
    assert!(cpu.read_flag(Flag::Carry));
}

#[test]
fn test_sbc_binary() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();
    cpu.set_flag(Flag::Decimal, false);

    cpu.set_flag(Flag::Carry, true);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x21);
    cpu.a = 0x41;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(SBC, Imm)
    );
    assert_eq!(0x20, cpu.a);
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));
    assert!(!cpu.read_flag(Flag::Overflow));

    cpu.set_flag(Flag::Carry, false);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x21);
    cpu.a = 0x42;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(SBC, Imm)
    );
    assert_eq!(0x20, cpu.a);
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));
    assert!(!cpu.read_flag(Flag::Overflow));

    cpu.set_flag(Flag::Carry, true);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x01);
    cpu.a = 0x00;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(SBC, Imm)
    );
    assert_eq!(0xFF, cpu.a);
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(!cpu.read_flag(Flag::Carry));
    assert!(cpu.read_flag(Flag::Negative));
    assert!(!cpu.read_flag(Flag::Overflow));

    cpu.set_flag(Flag::Carry, true);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x7F);
    cpu.a = 0xFE;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(SBC, Imm)
    );
    assert_eq!(0x7F, cpu.a);
    assert!(!cpu.read_flag(Flag::Zero));
    assert!(cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));
    assert!(cpu.read_flag(Flag::Overflow));
}

#[test]
fn test_sbc_decimal() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();
    cpu.set_flag(Flag::Decimal, true);

    cpu.set_flag(Flag::Carry, true);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x20);
    cpu.a = 0x41;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(SBC, Imm)
    );
    assert_eq!(0x21, cpu.a);
    assert!(cpu.read_flag(Flag::Carry));

    cpu.set_flag(Flag::Carry, false);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x20);
    cpu.a = 0x42;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(SBC, Imm)
    );
    assert_eq!(0x21, cpu.a);
    assert!(cpu.read_flag(Flag::Carry));

    cpu.set_flag(Flag::Carry, true);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x99);
    cpu.a = 0x00;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(SBC, Imm)
    );
    assert_eq!(0x01, cpu.a);
    assert!(!cpu.read_flag(Flag::Carry));

    cpu.set_flag(Flag::Carry, false);
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x00);
    cpu.a = 0x90;
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(SBC, Imm)
    );
    assert_eq!(0x89, cpu.a);
    assert!(cpu.read_flag(Flag::Carry));
}

#[test]
fn test_jmp() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();

    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::Taken),
        cpu.execute(JMP, Abs)
    );
    assert_eq!(0x1234, cpu.pc);

    cpu.pc = 0;
    cpu.write_bus_byte(0x1234, 0x67);
    cpu.write_bus_byte(0x1235, 0x45);
    assert_eq!(
        (PageBoundary::Crossed, Branch::Taken),
        cpu.execute(JMP, AbsInd)
    );
    assert_eq!(0x4567, cpu.pc);

    // test the jmp indirect bug where the low byte is xxFF

    cpu.pc = 0;
    cpu.write_bus_byte(0, 0xFF);
    cpu.write_bus_byte(1, 0x12);
    cpu.write_bus_byte(0x12FF, 0xAB);
    cpu.write_bus_byte(0x1200, 0x89);
    assert_eq!(
        (PageBoundary::Crossed, Branch::Taken),
        cpu.execute(JMP, AbsInd)
    );
    assert_eq!(0x89AB, cpu.pc);
}

#[test]
fn test_jsr_rts() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();
    cpu.status = 0;

    cpu.sp = 0xFF;
    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.set_flag(Flag::Decimal, true);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::Taken),
        cpu.execute(JSR, Abs)
    );
    assert_eq!(0x1234, cpu.pc);
    assert_eq!(Flag::Decimal as u8, cpu.status);
    assert_eq!(0xFD, cpu.sp);
    assert_eq!(0x01, cpu.read_bus_byte(0x1FE));
    assert_eq!(0x00, cpu.read_bus_byte(0x1FF));

    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(RTS, Imp)
    );
    assert_eq!(0x0002, cpu.pc);
    assert_eq!(0xFF, cpu.sp);
    assert_eq!(Flag::Decimal as u8, cpu.status);
}

#[test]
fn test_brk_rti() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();
    cpu.status = 0;

    cpu.sp = 0xFF;
    cpu.pc = 0;
    cpu.write_bus_byte(0xfffe, 0x34);
    cpu.write_bus_byte(0xffff, 0x12);
    cpu.set_flag(Flag::Decimal, true);
    cpu.set_flag(Flag::InterruptDisable, false);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(BRK, Imp)
    );
    assert_eq!(0x1234, cpu.pc);
    assert_eq!(Flag::Decimal | Flag::InterruptDisable, cpu.status);
    assert_eq!(0xFC, cpu.sp);

    assert_eq!(0x00, cpu.read_bus_byte(0x1FF));
    assert_eq!(0x01, cpu.read_bus_byte(0x1FE));

    assert_eq!(Flag::Decimal | Flag::Break, cpu.read_bus_byte(0x1FD));

    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(RTI, Imp)
    );
    assert_eq!(0x0001, cpu.pc);
    assert_eq!(0xFF, cpu.sp);
    assert_eq!(Flag::Decimal | Flag::Break | Flag::Unused, cpu.status);
}
