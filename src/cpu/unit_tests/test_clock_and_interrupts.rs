use crate::{bus::Interruptor, cpu::*};

#[test]
fn test_reset() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();

    cpu.write_bus_byte(0xFFFC, 0x34);
    cpu.write_bus_byte(0xFFFD, 0x12);
    cpu.write_bus_byte(0x1234, 0xEA); //NOP

    cpu.reset();
    let cycles = cpu.run_instruction();
    assert_eq!(7, cycles);
    assert_eq!(0x1234, cpu.pc);
    assert_eq!(0xFD, cpu.sp);
    cpu.clock(); // start a 2 cycle nop
    cpu.reset(); // reset should clobber the ongoing nop
    let cycles = cpu.run_instruction();
    assert_eq!(7, cycles);
    assert_eq!(0x1234, cpu.pc);
    assert_eq!(0xFA, cpu.sp);
}

#[test]
fn test_nmi() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();
    let interruptor = Rc::new(RefCell::new(Interruptor::new()));
    cpu.add_device(interruptor.clone());

    cpu.write_bus_byte(0xFFFC, 0x34);
    cpu.write_bus_byte(0xFFFD, 0x12);
    cpu.write_bus_byte(0xFFFA, 0x67);
    cpu.write_bus_byte(0xFFFB, 0x45);

    cpu.reset();
    cpu.run_instruction();

    interruptor.borrow_mut().flags = InterruptFlags::NMI;
    cpu.clock_bus();
    cpu.poll_interrupts();
    interruptor.borrow_mut().flags = InterruptFlags::empty();
    cpu.clock_bus();
    cpu.poll_interrupts();
    let cycles = cpu.run_instruction();
    assert_eq!(7, cycles);
    assert_eq!(0x4567, cpu.pc);
    assert_eq!(0x12, cpu.read_bus_byte(0x1FD));
    assert_eq!(0x34, cpu.read_bus_byte(0x1FC));
    let pushed_status = cpu.read_bus_byte(0x1FB);
    assert!((pushed_status & StatusFlags::Break.bits()) == 0);
    assert!((pushed_status & StatusFlags::Unused.bits()) != 0);
    assert!(cpu.read_flag(StatusFlags::InterruptDisable));
    assert_eq!(0xFA, cpu.sp);
}

#[test]
fn test_irq() {
    let (mut cpu, _mem) = crate::cpu::create_test_configuration();
    let interruptor = Rc::new(RefCell::new(Interruptor::new()));
    cpu.add_device(interruptor.clone());

    cpu.write_bus_byte(0xFFFC, 0x34);
    cpu.write_bus_byte(0xFFFD, 0x12);
    cpu.write_bus_byte(0x1234, 0x78); // SEI
    cpu.write_bus_byte(0x1235, 0x58); // CLI
    cpu.write_bus_byte(0xFFFE, 0xAB);
    cpu.write_bus_byte(0xFFFF, 0x89);

    cpu.reset();
    cpu.run_instruction(); // run the reset
    cpu.run_instruction(); // SEI
    assert!(cpu.read_flag(StatusFlags::InterruptDisable));
    interruptor.borrow_mut().flags = InterruptFlags::IRQ; // should be masked
    cpu.run_instruction(); // CLI
    assert_eq!(0x1236, cpu.pc);
    assert!(!cpu.read_flag(StatusFlags::InterruptDisable));
    cpu.run_instruction();
    assert_eq!(0x89AB, cpu.pc);
    assert_eq!(0xFA, cpu.sp);
    assert_eq!(0x12, cpu.read_bus_byte(0x1FD));
    assert_eq!(0x36, cpu.read_bus_byte(0x1FC));
    let pushed_status = cpu.read_bus_byte(0x1FB);
    assert!((pushed_status & StatusFlags::Break.bits()) == 0);
    assert!((pushed_status & StatusFlags::Unused.bits()) != 0);
    assert!((pushed_status & StatusFlags::InterruptDisable.bits()) == 0);
    assert!(cpu.read_flag(StatusFlags::InterruptDisable));
    assert_eq!(0xFA, cpu.sp);
}
