use super::super::*;
use crate::ram::RAM;
use instructions::Instruction::*;
use instructions::Mode::*;

#[test]
fn test_read_modes() {
    let mut cpu = CPU::default();
    let mem = Box::new(RAM::new(0x0000, 0xFFFF, 0xFFFF));
    cpu.add_bus_device(mem);
    cpu.status = 0;

    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x42);
    cpu.execute(LDA, Imm);
    assert_eq!(cpu.a, 0x42);
    assert_eq!(cpu.pc, 1);

    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.write_bus_byte(0x1234, 0x43);
    cpu.execute(LDA, Abs);
    assert_eq!(cpu.a, 0x43);
    assert_eq!(cpu.pc, 2);

    cpu.pc = 0;
    cpu.x = 3;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.write_bus_byte(0x1237, 0x44);
    cpu.execute(LDA, AbsX);
    assert_eq!(cpu.a, 0x44);
    assert_eq!(cpu.pc, 2);

    cpu.pc = 0;
    cpu.y = 4;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.write_bus_byte(0x1238, 0x45);
    cpu.execute(LDA, AbsY);
    assert_eq!(cpu.a, 0x45);
    assert_eq!(cpu.pc, 2);

    cpu.pc = 0;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(0x34, 0x46);
    cpu.execute(LDA, Zp);
    assert_eq!(cpu.a, 0x46);
    assert_eq!(cpu.pc, 1);

    cpu.pc = 0;
    cpu.x = 5;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(0x39, 0x47);
    cpu.execute(LDA, Zpx);
    assert_eq!(cpu.a, 0x47);
    assert_eq!(cpu.pc, 1);

    cpu.pc = 0;
    cpu.y = 6;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(0x3A, 0x69);
    cpu.execute(LDA, Zpy);
    assert_eq!(cpu.a, 0x69);
    assert_eq!(cpu.pc, 1);

    cpu.pc = 0;
    cpu.x = 6;
    cpu.write_bus_byte(0x00, 0x12);
    cpu.write_bus_byte(0x18, 0x3B);
    cpu.write_bus_byte(0x19, 0x12);
    cpu.write_bus_byte(0x123B, 0x48);
    cpu.execute(LDA, IndX);
    assert_eq!(cpu.a, 0x48);
    assert_eq!(cpu.pc, 1);

    cpu.pc = 0;
    cpu.y = 7;
    cpu.write_bus_byte(0, 0x35);
    cpu.write_bus_byte(0x35, 0x34);
    cpu.write_bus_byte(0x36, 0x12);
    cpu.write_bus_byte(0x123B, 0x49);
    cpu.execute(LDA, IndY);
    assert_eq!(cpu.a, 0x49);
    assert_eq!(cpu.pc, 1);
}

#[test]
fn test_write_modes() {
    let mut cpu = CPU::default();
    let mem = Box::new(RAM::new(0x0000, 0xFFFF, 0xFFFF));
    cpu.add_bus_device(mem);
    cpu.status = 0;

    cpu.pc = 0;
    cpu.a = 0x42;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.execute(STA, Abs);
    assert_eq!(0x42, cpu.read_bus_byte(0x1234));
    assert_eq!(cpu.pc, 2);

    cpu.pc = 0;
    cpu.a = 0x43;
    cpu.x = 2;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.execute(STA, AbsX);
    assert_eq!(0x43, cpu.read_bus_byte(0x1236));
    assert_eq!(cpu.pc, 2);

    cpu.pc = 0;
    cpu.a = 0x44;
    cpu.y = 3;
    cpu.write_bus_byte(0, 0x34);
    cpu.write_bus_byte(1, 0x12);
    cpu.execute(STA, AbsY);
    assert_eq!(0x44, cpu.read_bus_byte(0x1237));
    assert_eq!(cpu.pc, 2);

    cpu.pc = 0;
    cpu.a = 0x45;
    cpu.write_bus_byte(0, 0x34);
    cpu.execute(STA, Zp);
    assert_eq!(0x45, cpu.read_bus_byte(0x0034));
    assert_eq!(cpu.pc, 1);

    cpu.pc = 0;
    cpu.a = 0x46;
    cpu.x = 4;
    cpu.write_bus_byte(0, 0x34);
    cpu.execute(STA, Zpx);
    assert_eq!(0x46, cpu.read_bus_byte(0x0038));
    assert_eq!(cpu.pc, 1);

    cpu.pc = 0;
    cpu.a = 0x47;
    cpu.y = 5;
    cpu.write_bus_byte(0, 0x34);
    cpu.execute(STA, Zpy);
    assert_eq!(0x47, cpu.read_bus_byte(0x0039));
    assert_eq!(cpu.pc, 1);

    cpu.pc = 0;
    cpu.a = 0x47;
    cpu.y = 5;
    cpu.write_bus_byte(0, 0x34);
    cpu.execute(STA, Zpy);
    assert_eq!(0x47, cpu.read_bus_byte(0x0039));
    assert_eq!(cpu.pc, 1);

    cpu.pc = 0;
    cpu.a = 0x48;
    cpu.x = 6;
    cpu.write_bus_byte(0x00, 0x12);
    cpu.write_bus_byte(0x18, 0x3B);
    cpu.write_bus_byte(0x19, 0x12);
    cpu.execute(STA, IndX);
    assert_eq!(0x48, cpu.read_bus_byte(0x123B));

    assert_eq!(cpu.pc, 1);

    cpu.pc = 0;
    cpu.y = 7;
    cpu.a = 0x49;
    cpu.write_bus_byte(0, 0x35);
    cpu.write_bus_byte(0x35, 0x34);
    cpu.write_bus_byte(0x36, 0x12);
    cpu.execute(STA, IndY);
    assert_eq!(0x49, cpu.read_bus_byte(0x123B));
    assert_eq!(cpu.pc, 1);
}
