#![allow(clippy::upper_case_acronyms)]

mod cpu;
mod device;
mod ram;

use cpu::*;

fn main() {
    let mut cpu = CPU::new(CPUType::RP2A03);
    let mem = Box::new(ram::RAM::new(0x0000, 0xFFFF, 0xFFFF));
    cpu.add_bus_device(mem);

    cpu.irq(); // just calling to avoid unused warning for now
    cpu.nmi(); // just calling to avoid unused warning for now

    // have to reset to put cpu in a good state
    cpu.reset();

    // now loop until trapped or halted
    while !cpu.stuck() {
        cpu.clock();
    }
}
