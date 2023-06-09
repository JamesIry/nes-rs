use crate::bus::{BusDevice, InterruptFlags};

pub struct RAM {
    start_addr: u16,
    end_addr: u16,
    addr_mask: u16,
    memory: Vec<u8>,
}

impl RAM {
    pub fn new(start_addr: u16, end_addr: u16, addr_mask: u16) -> Self {
        Self {
            start_addr,
            end_addr,
            addr_mask,
            memory: vec![
                0;
                (end_addr & addr_mask) as usize - (start_addr & addr_mask) as usize + 1
            ],
        }
    }

    #[cfg(test)]
    pub fn raw(&mut self) -> &mut Vec<u8> {
        &mut self.memory
    }

    fn physical(&self, addr: u16) -> usize {
        ((addr & self.addr_mask) - (self.start_addr & self.addr_mask)) as usize
    }
}

impl BusDevice for RAM {
    fn get_address_range(&self) -> (u16, u16) {
        (self.start_addr, self.end_addr)
    }

    fn read(&mut self, addr: u16) -> u8 {
        if addr >= self.start_addr && addr <= self.end_addr {
            self.memory[self.physical(addr)]
        } else {
            panic!("Address out of range in RAM {}", addr)
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> u8 {
        if addr >= self.start_addr && addr <= self.end_addr {
            let physical = self.physical(addr);
            let old = self.memory[physical];
            self.memory[physical] = data;
            old
        } else {
            panic!("Address out of range in RAM {}", addr)
        }
    }

    fn bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }
}
