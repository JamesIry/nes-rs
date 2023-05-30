#![allow(clippy::upper_case_acronyms)]

mod apu;
mod bus;
mod cartridge;
mod cpu;
mod ppu;
mod ram;

use std::{cell::RefCell, rc::Rc};

use apu::APU;
use cartridge::{Cartridge, CartridgeCPUPort, CartridgePPUPort};
use cpu::{CPUType, CPU};
use ppu::PPU;
use ram::RAM;
#[cfg(test)]
mod integration_tests {
    mod nestest;
}

fn main() {
    let cartridge = Rc::new(RefCell::new(
        Cartridge::load("resources/test/nestest.nes").unwrap(),
    ));

    let cpu = Rc::new(RefCell::new(CPU::new(CPUType::RP2A03)));
    let ppu = Rc::new(RefCell::new(PPU::new()));
    let apu = Rc::new(RefCell::new(APU::new(cpu.clone())));

    // 0x0000 - 0x1FFFF "work" RAM (WRAM)
    // NES ram is physically only 0x0000 - 0x07FF, but it's then "mirrored" 3 more
    // times to 0x1FFF. "Mirroring" can be accomplished by masking off some bits
    cpu.as_ref()
        .borrow_mut()
        .add_device(Rc::new(RefCell::new(RAM::new(0x0000, 0x1FFF, 0x07FF))));
    //0x2000 - 0x3FFF  PPU Registers from 0x2000 to 0x2007 and then mirrored with mask 0x0007
    cpu.as_ref().borrow_mut().add_device(ppu.clone());
    //0x4000 - 0x4017  APU and IO registers
    //0x4018 - 0x401F  APU and IO functionality that is disabled
    cpu.as_ref().borrow_mut().add_device(apu.clone());
    //0x4020 - 0xFFFF  Cartridge space
    cpu.as_ref()
        .borrow_mut()
        .add_device(Rc::new(RefCell::new(CartridgeCPUPort::new(
            cartridge.clone(),
        ))));

    ppu.as_ref()
        .borrow_mut()
        .add_device(Rc::new(RefCell::new(CartridgePPUPort::new(cartridge))));

    cpu.as_ref().borrow_mut().irq(); // just calling to avoid unused warning for now

    cpu.as_ref().borrow_mut().reset();

    let mut t = 0;
    loop {
        if t == 0 {
            apu.as_ref().borrow_mut().clock();
            cpu.as_ref().borrow_mut().clock();
        }

        if ppu.as_ref().borrow_mut().clock() {
            cpu.as_ref().borrow_mut().nmi();
        }

        t += 1;
        if t == 3 {
            t = 0;
        }
    }
}
