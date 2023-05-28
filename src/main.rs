#![allow(clippy::upper_case_acronyms)]

mod apu;
mod cartridge;
mod cpu;
mod device;
mod ppu;
mod ram;

use apu::APU;
use cartridge::Cartridge;
use cpu::{CPUType, CPU};
use ppu::PPU;

#[cfg(test)]
mod integration_tests {
    mod nestest;
}

fn main() {
    let mut cpu = CPU::new(CPUType::RP2A03);

    // 0x0000 - 0x1FFFF RAM
    // NES ram is physically only 0x0000 - 0x07FF, but it's then "mirrored" 3 more
    // times to 0x1FFF. "Mirroring" can be accomplished by masking off some bits
    cpu.add_bus_device(Box::new(ram::RAM::new(0x0000, 0x1FFF, 0x07FF)));
    //0x2000 - 0x3FFF  PPU Registers from 0x2000 to 0x2007 and then mirrored with mask 0x0007
    cpu.add_bus_device(Box::new(PPU::new()));
    //0x4000 - 0x4017  APU and IO registers
    //0x4018 - 0x401F  APU and IO functionality that is disabled
    cpu.add_bus_device(Box::new(APU::new()));
    //0x4020 - 0xFFFF  Cartridge space
    cpu.add_bus_device(Box::new(
        Cartridge::load("resources/test/nestest.nes").unwrap(),
    ));

    cpu.irq(); // just calling to avoid unused warning for now
    cpu.nmi(); // just calling to avoid unused warning for now

    cpu.reset();

    // now loop until trapped or halted
    while !cpu.stuck() {
        cpu.clock();
    }
}
