use crate::device::BusDevice;

static RANGE_START: u16 = 0x4000;
static RANGE_END: u16 = 0x401F;
static ADDR_MASK: u16 = 0x001F;

/* Not really a APU yet. Just some read/write registers */
pub struct APU {
    registers: [u8; 0x20],
}

impl APU {
    pub fn new() -> Self {
        Self {
            registers: [0xFF; 0x20],
        }
    }

    fn physical(addr: u16) -> usize {
        ((addr & ADDR_MASK) - (RANGE_START & ADDR_MASK)) as usize
    }
}

impl BusDevice for APU {
    fn read_from_cpu_bus(&mut self, addr: u16) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            Some(self.registers[APU::physical(addr)])
        } else {
            None
        }
    }

    fn write_to_cpu_bus(&mut self, addr: u16, data: u8) -> Option<u8> {
        if addr >= RANGE_START && addr <= RANGE_END {
            let old = self.registers[APU::physical(addr)];
            self.registers[APU::physical(addr)] = data;
            Some(old)
        } else {
            None
        }
    }
}
