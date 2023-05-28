use std::{cell::RefCell, rc::Rc, rc::Weak};

#[cfg(test)]
use crate::{cpu::CPU, ram::RAM};
pub trait Processor {
    fn clock(&mut self);
    fn reset(&mut self);
    fn nmi(&mut self);
    fn irq(&mut self);
    fn stuck(&self) -> bool;

    fn set_bus(&mut self, bus: Weak<RefCell<Bus>>);
}

pub trait BusDevice {
    fn read(&mut self, addr: u16) -> Option<u8>;
    fn write(&mut self, addr: u16, data: u8) -> Option<u8>;
}

pub struct Bus {
    processor: Rc<RefCell<dyn Processor>>,
    bus_devices: Vec<Rc<RefCell<dyn BusDevice>>>,
}

impl Bus {
    pub fn new(processor: Rc<RefCell<dyn Processor>>) -> Rc<RefCell<Self>> {
        let result = Rc::new(RefCell::new(Self {
            processor: processor.clone(),
            bus_devices: Vec::new(),
        }));
        let weak = Rc::downgrade(&result);
        processor.borrow_mut().set_bus(weak);
        result
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

    pub fn clock(&self) {
        self.processor.borrow_mut().clock();
    }

    pub fn reset(&self) {
        self.processor.borrow_mut().reset();
    }

    pub fn nmi(&self) {
        self.processor.borrow_mut().nmi();
    }

    pub fn irq(&self) {
        self.processor.borrow_mut().irq();
    }

    pub fn stuck(&self) -> bool {
        self.processor.borrow().stuck()
    }

    #[allow(clippy::type_complexity)]
    #[cfg(test)]
    pub fn configure_generic() -> (Rc<RefCell<CPU>>, Rc<RefCell<RAM>>, Rc<RefCell<Bus>>) {
        let cpu = Rc::new(RefCell::new(CPU::default()));
        let mem = Rc::new(RefCell::new(RAM::new(0x0000, 0xFFFF, 0xFFFF)));
        let bus = Bus::new(cpu.clone());
        bus.borrow_mut().add_device(mem.clone());
        (cpu, mem, bus)
    }
}
