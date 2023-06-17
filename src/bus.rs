use std::{cell::RefCell, rc::Rc};

pub trait BusDevice {
    fn get_address_range(&self) -> (u16, u16);
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8) -> u8;
    fn bus_clock(&mut self) -> InterruptFlags;
}

type AddrRange = (u16, u16);
pub struct Bus {
    bus_devices: Vec<(AddrRange, Rc<RefCell<dyn BusDevice>>)>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            bus_devices: Vec::new(),
        }
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn BusDevice>>) {
        let addr_range = device.as_ref().borrow_mut().get_address_range();
        self.bus_devices.push((addr_range, device));
    }

    pub fn read(&self, addr: u16) -> u8 {
        for device in &self.bus_devices {
            if device.0 .0 <= addr && addr <= device.0 .1 {
                return device.1.borrow_mut().read(addr);
            }
        }
        0
    }

    pub fn write(&self, addr: u16, data: u8) -> u8 {
        for device in &self.bus_devices {
            if device.0 .0 <= addr && addr <= device.0 .1 {
                return device.1.borrow_mut().write(addr, data);
            }
        }
        0
    }

    pub fn clock(&self) -> InterruptFlags {
        let mut flags = InterruptFlags::empty();
        for device in &self.bus_devices {
            flags |= device.1.borrow_mut().bus_clock();
        }
        flags
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    pub struct InterruptFlags: u8 {
        const IRQ = 1;
        const NMI = 2;
    }
}

#[cfg(test)]
pub struct Interruptor {
    pub flags: InterruptFlags,
}
#[cfg(test)]
impl Interruptor {
    pub fn new() -> Self {
        Self {
            flags: InterruptFlags::empty(),
        }
    }
}
#[cfg(test)]
impl Default for Interruptor {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
impl BusDevice for Interruptor {
    fn get_address_range(&self) -> (u16, u16) {
        (0xFFFF, 0x0000)
    }

    fn read(&mut self, _addr: u16) -> u8 {
        unreachable!()
    }

    fn write(&mut self, _addr: u16, _data: u8) -> u8 {
        unreachable!()
    }

    fn bus_clock(&mut self) -> InterruptFlags {
        self.flags
    }
}
