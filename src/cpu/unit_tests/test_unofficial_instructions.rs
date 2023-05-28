#![allow(clippy::field_reassign_with_default)]

use crate::cpu::flags::*;
use crate::cpu::instructions::Instruction::*;
use crate::cpu::instructions::Mode::*;
use crate::cpu::*;
use crate::ram::RAM;

#[test]
fn test_load_store_unofficial() {
    let mut cpu = CPU::default();
    cpu.status = 0;

    let mem = Box::new(RAM::new(0x0000, 0xFFFF, 0xFFFF));
    cpu.add_bus_device(mem);

    cpu.pc = 0;
    cpu.y = 4;
    cpu.sp = 0x41;
    cpu.a = 0xFF;
    cpu.x = 0xFF;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.write_bus_byte(0x1238, 0x45);
    cpu.execute(LAS, AbsY);
    assert_eq!(cpu.a, 0x41 & 0x45);
    assert_eq!(cpu.x, 0x41 & 0x45);
    assert_eq!(cpu.sp, 0x41 & 0x45);
    assert_eq!(cpu.pc, 2);

    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x42);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(LAX, Imm)
    );
    assert_eq!(0x01, cpu.pc);
    assert_eq!(0x42, cpu.a);
    assert_eq!(0x42, cpu.x);

    cpu.pc = 0;
    cpu.a = 0x45;
    cpu.x = 0x43;
    cpu.y = 3;
    cpu.write_bus_byte(0x1237, 0xFF);
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.execute(SHA, AbsY);
    assert_eq!(0x45 & 0x43 & 0x13, cpu.read_bus_byte(0x1237));
    assert_eq!(cpu.pc, 2);

    cpu.pc = 0;
    cpu.y = 7;
    cpu.a = 0x79;
    cpu.x = 0x34;
    cpu.write_bus_byte(0x123B, 0xFF);
    cpu.write_bus_byte(0, 0x35);
    cpu.write_bus_byte(0x35, 0x34);
    cpu.write_bus_byte(0x36, 0x12);
    cpu.execute(SHA, IndY);
    assert_eq!(0x79 & 0x34 & 0x13, cpu.read_bus_byte(0x123B));
    assert_eq!(cpu.pc, 1);

    cpu.pc = 0;
    cpu.a = 0x45;
    cpu.x = 0x43;
    cpu.y = 3;
    cpu.sp = 0;
    cpu.write_bus_byte(0x1237, 0xFF);
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.execute(SHS, AbsY);
    assert_eq!(0x45 & 0x43, cpu.sp);
    assert_eq!(0x45 & 0x43 & 0x13, cpu.read_bus_byte(0x1237));
    assert_eq!(cpu.pc, 2);

    cpu.pc = 0;
    cpu.x = 0x43;
    cpu.y = 3;
    cpu.write_bus_byte(0x1237, 0xFF);
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.execute(SHX, AbsY);
    assert_eq!(0x43 & 0x13, cpu.read_bus_byte(0x1237));
    assert_eq!(cpu.pc, 2);

    cpu.pc = 0;
    cpu.y = 3;
    cpu.write_bus_byte(0x1237, 0xFF);
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.execute(SHY, AbsY);
    assert_eq!(0x3 & 0x13, cpu.read_bus_byte(0x1237));
    assert_eq!(cpu.pc, 2);
}

#[test]
fn test_logic_unofficial() {
    let mut cpu = CPU::default();
    let mem = Box::new(RAM::new(0x0000, 0xFFFF, 0xFFFF));
    cpu.add_bus_device(mem);

    cpu.pc = 0;
    cpu.a = 0b10010110;
    cpu.write_bus_byte(0, 0x0F);
    cpu.set_flag(Flag::Carry, true);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ANC, Imm)
    );
    assert_eq!(0b00000110, cpu.a);
    assert!(!cpu.read_flag(Flag::Carry));

    cpu.pc = 0;
    cpu.a = 0b10010110;
    cpu.write_bus_byte(0, 0xFF);
    cpu.set_flag(Flag::Carry, false);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ANC, Imm)
    );
    assert_eq!(0b10010110, cpu.a);
    assert!(cpu.read_flag(Flag::Carry));

    cpu.set_flag(Flag::Decimal, false);

    cpu.pc = 0;
    cpu.a = 0b10010110;
    cpu.write_bus_byte(0, 0b11111100);
    cpu.set_flag(Flag::Carry, false);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ARR, Imm)
    );
    assert_eq!(0b01001010, cpu.a);
    assert!(cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));

    cpu.pc = 0;
    cpu.a = 0b01010110;
    cpu.write_bus_byte(0, 0b11111100);
    cpu.set_flag(Flag::Carry, true);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ARR, Imm)
    );
    assert_eq!(0b10101010, cpu.a);
    assert!(!cpu.read_flag(Flag::Carry));
    assert!(cpu.read_flag(Flag::Negative));

    cpu.pc = 0;
    cpu.a = 0b01010111;
    cpu.write_bus_byte(0, 0b11111101);
    cpu.set_flag(Flag::Carry, false);
    cpu.set_flag(Flag::Negative, true);
    assert_eq!(
        (PageBoundary::NotCrossed, Branch::NotTaken),
        cpu.execute(ASR, Imm)
    );
    assert_eq!(0b00101010, cpu.a);
    assert!(cpu.read_flag(Flag::Carry));
    assert!(!cpu.read_flag(Flag::Negative));
}

// DCP
// ISC
// RLA
// RRA
// SBX
// SLO
// SRE
// XAA
