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
}

impl BusDevice for RAM {
    fn read(&self, addr: u16) -> Option<u8> {
        if addr >= self.start_addr && addr <= self.end_addr {
            let target = addr & self.addr_mask;
            let data = self.memory[(target - self.start_addr) as usize];
            Some(data)
        } else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> bool {
        if addr >= self.start_addr && addr <= self.end_addr {
            let target = addr & self.addr_mask;
            self.memory[(target - self.start_addr) as usize] = data;
            true
        } else {
            false
        }
    }
}
