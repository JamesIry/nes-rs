// run the functional test from https://github.com/Klaus2m5/6502_65C02_functional_tests

use std::{
    fs::File,
    io::{BufReader, Read},
};

use super::super::*;
use crate::ram::RAM;

#[test]
fn functional_test() {
    let mut cpu = CPU::default();

    let mut mem = Box::new(RAM::new(0x0000, 0xFFFF, 0xFFFF));
    // load the bin file into ram
    let file = File::open("resources/test/6502_functional_test.bin").unwrap();
    let mut reader = BufReader::new(file);
    mem.raw().truncate(0);
    reader.read_to_end(mem.raw()).unwrap();

    cpu.add_bus_device(mem);
    // cpu.monitor = Box::new(cpu::monitor::LoggingMonitor::new());

    // get the cpu ready to run the test code
    cpu.reset_to(0x0400);

    // now loop until trapped or halted
    while !cpu.stuck() {
        cpu.run_instruction();
    }

    #[allow(clippy::assertions_on_constants)]
    if cpu.pc != 0x3469 {
        println!(
            "Stuck [ a {:#04x} | x {:#04x} | y {:#04x} | sp {:#04x} | status {:#04x}  | pc {:#06x} ]",
            cpu.a, cpu.x, cpu.y, cpu.sp, cpu.status, cpu.pc);

        assert!(false);
    }
}
