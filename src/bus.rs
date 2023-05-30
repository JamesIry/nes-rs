use std::{cell::RefCell, rc::Rc};

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
}
