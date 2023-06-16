use std::{cell::RefCell, rc::Rc};

use super::PPU;

use crate::{
    bus::BusDevice,
    cpu::{CPUCycleType, CPUType, CPU},
    nes::apu::{APUCycleType, APU},
    nes::cartridge::{Cartridge, CartridgeCPUPort},
    ram::RAM,
};

#[test]
fn test_dma() {
    for alignment in 0..=1 {
        let cartridge = Rc::new(RefCell::new(
            Cartridge::load("resources/test/nestest.nes").unwrap(),
        ));

        let cpu = Rc::new(RefCell::new(CPU::new(CPUType::RP2A03)));
        let ppu = Rc::new(RefCell::new(PPU::new(PPU::nul_renderer())));
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
        apu.as_ref().borrow_mut().write(0x4014, 0x03);
        let _ = apu.as_ref().borrow_mut().clock(CPUCycleType::Write); // process the write
                                                                      // make sure rdy didn't get cleared with that tick, because it was a write cycle
        assert!(cpu.borrow().is_rdy());
        if alignment == 1 {
            apu.as_ref().borrow_mut().cycle_type = APUCycleType::Put;
            _ = apu.as_ref().borrow_mut().clock(CPUCycleType::Read); // need extra cycle to align
        }

        for _ in 0..512 {
            let _ = apu.as_ref().borrow_mut().clock(CPUCycleType::Read);
            cpu.as_ref().borrow_mut().clock();
            assert!(!(cpu.borrow().is_rdy()));
            assert_eq!(cycles, cpu.as_ref().borrow_mut().cycles());
            assert_eq!((false, false), ppu.as_ref().borrow_mut().clock());
            assert_eq!((false, false), ppu.as_ref().borrow_mut().clock());
            assert_eq!((false, false), ppu.as_ref().borrow_mut().clock());
        }

        let _ = apu.as_ref().borrow_mut().clock(CPUCycleType::Read);
        assert!(cpu.borrow().is_rdy());

        for address in 0..0x0100 {
            let data = (address as u8).wrapping_sub(0x02) as usize;
            assert_eq!(
                data as u8,
                ppu.as_ref().borrow_mut().primary_oam.table[address]
            );
        }
    }
}
