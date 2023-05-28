use crate::device::BusDevice;

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
    fn read_from_cpu_bus(&mut self, addr: u16) -> Option<u8> {
        if addr >= self.start_addr && addr <= self.end_addr {
            let data = self.memory[self.physical(addr)];
            Some(data)
        } else {
            None
        }
    }

    fn write_to_cpu_bus(&mut self, addr: u16, data: u8) -> Option<u8> {
        if addr >= self.start_addr && addr <= self.end_addr {
            let physical = self.physical(addr);
            let old = self.memory[physical];
            self.memory[physical] = data;
            Some(old)
        } else {
            None
        }
    }
}
