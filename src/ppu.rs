use crate::bus::BusDevice;

static RANGE_START: u16 = 0x2000;
static RANGE_END: u16 = 0x3FFF;
static ADDR_MASK: u16 = 0x0007;

/* Not really a PPU yet. Just some read/write registers */
pub struct PPU {
    registers: [u8; 8],
}

impl PPU {
    pub fn new() -> Self {
        Self { registers: [0; 8] }
    }

    fn physical(addr: u16) -> usize {
        ((addr & ADDR_MASK) - (RANGE_START & ADDR_MASK)) as usize
    }
}

impl BusDevice for PPU {
    fn read(&mut self, addr: u16) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            Some(self.registers[PPU::physical(addr)])
        } else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            let old = self.registers[PPU::physical(addr)];
            self.registers[PPU::physical(addr)] = data;
            Some(old)
        } else {
            None
        }
    }
}
