use std::{cell::RefCell, rc::Rc};

#[cfg(test)]
use crate::{cpu::CPU, ram::RAM};

pub trait BusDevice {
    fn read(&mut self, addr: u16) -> Option<u8>;
    fn write(&mut self, addr: u16, data: u8) -> Option<u8>;
}

pub struct Bus {
    bus_devices: Vec<Rc<RefCell<dyn BusDevice>>>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            bus_devices: Vec::new(),
        }
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn BusDevice>>) {
        self.bus_devices.push(device);
    }

    pub fn read(&self, addr: u16) -> u8 {
        for device in &self.bus_devices {
            if let Some(data) = device.borrow_mut().read(addr) {
                return data;
            }
        }
        0
    }

    pub fn write(&self, addr: u16, data: u8) -> u8 {
        for device in &self.bus_devices {
            if let Some(data) = device.borrow_mut().write(addr, data) {
                return data;
            }
        }
        0
    }

    #[allow(clippy::type_complexity)]
    #[cfg(test)]
    pub fn configure_generic() -> (Rc<RefCell<CPU>>, Rc<RefCell<RAM>>) {
        let cpu = Rc::new(RefCell::new(CPU::default()));
        let mem = Rc::new(RefCell::new(RAM::new(0x0000, 0xFFFF, 0xFFFF)));
        cpu.borrow_mut().add_device(mem.clone());
        (cpu, mem)
    }
}
