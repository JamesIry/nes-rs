use std::{cell::RefCell, rc::Rc};

use super::PPU;

use crate::{
    apu::APU,
    bus::BusDevice,
    cartridge::{Cartridge, CartridgeCPUPort},
    cpu::{CPUType, CPU},
    ppu::nul_renderer,
    ram::RAM,
};

#[test]
fn test_dma() {
    let cartridge = Rc::new(RefCell::new(
        Cartridge::load("resources/test/nestest.nes").unwrap(),
    ));

    let cpu = Rc::new(RefCell::new(CPU::new(CPUType::RP2A03)));
    let ppu = Rc::new(RefCell::new(PPU::new(nul_renderer)));
    let ram = Rc::new(RefCell::new(RAM::new(0x0000, 0x1FFF, 0x07FF)));
    let apu = Rc::new(RefCell::new(APU::new(cpu.clone())));
    cpu.as_ref().borrow_mut().add_device(ram.clone());
    cpu.as_ref().borrow_mut().add_device(ppu.clone());
    cpu.as_ref().borrow_mut().add_device(apu.clone());
    cpu.as_ref()
        .borrow_mut()
        .add_device(Rc::new(RefCell::new(CartridgeCPUPort::new(cartridge))));

    // just to ensure the CPU is merrily chirping away on the test instructions
    // if it starts ticking
    cpu.as_ref().borrow_mut().reset_to(0xC000);

    for i in 0..0x0100 {
        ram.as_ref().borrow_mut().raw()[i + 0x0300] = i as u8;
    }

    // make sure the scanline is in a range that allows oam writes
    ppu.as_ref().borrow_mut().scan_line = 240;

    let cycles = cpu.as_ref().borrow_mut().cycles();

    ppu.as_ref().borrow_mut().write(0x2003, 0x02);
    apu.as_ref().borrow_mut().write(0x4015, 0x03);
    apu.as_ref().borrow_mut().clock(); // process the write and clear cpu.rdy

    for _ in 0..513 {
        apu.as_ref().borrow_mut().clock();
        cpu.as_ref().borrow_mut().clock();
        assert!(!(cpu.borrow().is_rdy()));
        assert_eq!(cycles, cpu.as_ref().borrow_mut().cycles());
        assert!(!ppu.as_ref().borrow_mut().clock());
        assert!(!ppu.as_ref().borrow_mut().clock());
        assert!(!ppu.as_ref().borrow_mut().clock());
    }

    apu.as_ref().borrow_mut().clock();
    assert!(cpu.borrow().is_rdy());

    for address in 0..0x0100 {
        let data = (address as u8).wrapping_sub(0x02) as usize;
        assert_eq!(data as u8, ppu.as_ref().borrow_mut().oam_table[address]);
    }
}
