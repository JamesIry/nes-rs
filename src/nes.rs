mod apu;
mod cartridge;
pub mod controllers;
mod ppu;

use std::{cell::RefCell, rc::Rc};

use crate::cpu::{CPUCycleType, CPUType, CPU};
use crate::ram::RAM;
use anyhow::Result;
use apu::APU;
use cartridge::{Cartridge, CartridgeCPUPort, CartridgePPUPort};
use ppu::PPU;

use self::controllers::{Controller, NulController};
#[cfg(test)]
mod integration_tests {
    mod nestest;
}

pub struct NES {
    cpu: Rc<RefCell<CPU>>,
    apu: Rc<RefCell<APU>>,
    ppu: Rc<RefCell<PPU>>,
    cartridge_cpu_port: Rc<RefCell<CartridgeCPUPort>>,
    cartridge_ppu_port: Rc<RefCell<CartridgePPUPort>>,
    tick: u8,
    controller1: Rc<RefCell<dyn Controller>>,
    controller2: Rc<RefCell<dyn Controller>>,
    last_cycle_type: CPUCycleType,
}

impl NES {
    pub fn new(renderer: Box<dyn FnMut(u16, u16, u8, u8, u8)>) -> Self {
        let cartridge = Rc::new(RefCell::new(Cartridge::nul_cartridge()));

        let cpu = Rc::new(RefCell::new(CPU::new(CPUType::RP2A03)));
        let ppu = Rc::new(RefCell::new(PPU::new(renderer)));
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
        let cartridge_cpu_port = Rc::new(RefCell::new(CartridgeCPUPort::new(cartridge.clone())));
        cpu.as_ref()
            .borrow_mut()
            .add_device(cartridge_cpu_port.clone());

        let cartridge_ppu_port = Rc::new(RefCell::new(CartridgePPUPort::new(cartridge)));

        ppu.as_ref()
            .borrow_mut()
            .add_device(cartridge_ppu_port.clone());

        let controller1 = Rc::new(RefCell::new(NulController::new()));
        let controller2 = Rc::new(RefCell::new(NulController::new()));

        Self {
            cpu,
            apu,
            ppu,
            cartridge_cpu_port,
            cartridge_ppu_port,
            tick: 0,
            controller1,
            controller2,
            last_cycle_type: CPUCycleType::Read,
        }
    }

    pub fn reset(&mut self) {
        self.cpu.as_ref().borrow_mut().reset();
        self.apu.as_ref().borrow_mut().reset();
        self.ppu.as_ref().borrow_mut().reset();
        self.tick = 0;
    }

    pub fn clock(&mut self) -> (bool, Option<f32>) {
        let mut audio_sample = None;
        if self.tick == 0 {
            {
                let input1 = self.controller1.borrow().read_value();
                let input2 = self.controller2.borrow().read_value();

                let mut apu_borrowed = self.apu.as_ref().borrow_mut();
                apu_borrowed.set_input_port1(input1);
                apu_borrowed.set_input_port2(input2);
                let (irq, sample) = apu_borrowed.clock(self.last_cycle_type);
                audio_sample = Some(sample);

                self.cpu.as_ref().borrow_mut().irq(irq);
            };
            self.last_cycle_type = self.cpu.as_ref().borrow_mut().clock();
        }

        let (end_of_frame, nmi) = self.ppu.as_ref().borrow_mut().clock();
        self.cpu.as_ref().borrow_mut().nmi(nmi);

        self.tick += 1;
        if self.tick == 3 {
            self.tick = 0;
        }

        (end_of_frame, audio_sample)
    }

    pub fn load_cartridge(&mut self, cartridge_name: String) -> Result<()> {
        let cartridge = Cartridge::load(&cartridge_name)?;
        let cart_ref = Rc::new(RefCell::new(cartridge));
        self.cartridge_cpu_port
            .replace(CartridgeCPUPort::new(cart_ref.clone()));
        self.cartridge_ppu_port
            .replace(CartridgePPUPort::new(cart_ref));

        Ok(())
    }

    pub fn plugin_controller1(&mut self, controller: Rc<RefCell<dyn Controller>>) {
        self.controller1 = controller;
    }

    pub fn plugin_controller2(&mut self, controller: Rc<RefCell<dyn Controller>>) {
        self.controller2 = controller;
    }
}
