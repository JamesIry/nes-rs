// run the functional test from https://github.com/Klaus2m5/6502_65C02_functional_tests

use std::{
    fs::File,
    io::{BufReader, Read},
};

#[test]
fn functional_test() {
    let (mut cpu, mem) = crate::cpu::create_test_configuration();

    // load the bin file into ram
    let file = File::open("resources/test/6502_functional_test.bin").unwrap();
    let mut reader = BufReader::new(file);
    mem.borrow_mut().raw().truncate(0);
    reader.read_to_end(mem.borrow_mut().raw()).unwrap();

    // cpu.monitor = Box::new(cpu::monitor::LoggingMonitor::new());

    // get the cpu ready to run the test code
    cpu.reset_to(0x0400);

    // now loop until trapped or halted
    while !cpu.stuck() {
        cpu.run_instruction();
    }

    assert_eq!(
        0x3469, cpu.pc,
        "Stuck [ a {:#04x} | x {:#04x} | y {:#04x} | sp {:#04x} | status {:#04x}  | pc {:#06x} ]",
        cpu.a, cpu.x, cpu.y, cpu.sp, cpu.status, cpu.pc,
    );
}
