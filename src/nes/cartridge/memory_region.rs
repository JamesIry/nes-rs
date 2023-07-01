#[cfg(test)]
mod unit_tests;

use super::MirrorType;

const MAX_BANKS: usize = 8;
pub struct MemoryRegion {
    pub memory_type: MemoryType,
    pub memory: Vec<u8>,
    pub alternate_memory: Vec<u8>,

    pub start_address: u16,
    pub end_address: u16,

    bank_size: usize,
    page_count: usize,
    bank_count: usize,
    alternate_bank_count: usize,
    bank_map: [(i16, bool); MAX_BANKS],

    write_protect: bool,
    alternate_write_protect: bool,
}

impl MemoryRegion {
    pub fn new(
        memory_type: MemoryType,
        memory: Vec<u8>,
        start_address: u16,
        end_address: u16,
        write_protect: bool,
    ) -> MemoryRegion {
        let mut result = Self {
            memory_type,
            memory,
            alternate_memory: Vec::new(),
            start_address,
            end_address,
            write_protect,
            alternate_write_protect: true,

            bank_size: 0,
            page_count: 0,
            bank_count: 0,
            alternate_bank_count: 0,
            bank_map: [(0, false); MAX_BANKS],
        };
        result.set_bank_size(result.get_address_size().min(result.memory.len()));
        result
    }

    pub fn read(&self, addr: u16) -> u8 {
        let converted = self.convert(addr);
        if converted.1 {
            self.alternate_memory[converted.0]
        } else {
            self.memory[converted.0]
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) -> u8 {
        let converted = self.convert(addr);
        let (memory, write_protect) = if converted.1 {
            (&mut self.alternate_memory, self.alternate_write_protect)
        } else {
            (&mut self.memory, self.write_protect)
        };

        let old = memory[converted.0];
        if !write_protect {
            memory[converted.0] = value;
        }
        old
    }

    fn get_address_size(&self) -> usize {
        self.end_address as usize - self.start_address as usize + 1
    }

    pub fn get_memory_size_k(&mut self) -> u16 {
        (self.memory.len() / 1024) as u16
    }

    pub fn set_bank_size_k(&mut self, bank_size_k: u16) {
        self.set_bank_size(k_to_usize(bank_size_k));
    }

    fn set_bank_size(&mut self, bank_size: usize) {
        self.bank_size = bank_size;
        self.page_count = self.get_address_size() / self.bank_size;

        self.bank_count = self.memory.len() / self.bank_size;
        self.alternate_bank_count = self.alternate_memory.len() / self.bank_size;
    }

    pub fn set_alternate_memory(&mut self, alternate_memory: Vec<u8>, write_protect: bool) {
        self.alternate_memory = alternate_memory;
        self.alternate_write_protect = write_protect;
        self.set_bank_size(self.bank_size);
    }

    pub fn set_mirror_type(&mut self, mirror_type: MirrorType) {
        match mirror_type {
            MirrorType::Vertical => {
                self.bank_map[0] = (0, false);
                self.bank_map[1] = (1, false);
                self.bank_map[2] = (0, false);
                self.bank_map[3] = (1, false);
            }
            MirrorType::Horizontal => {
                self.bank_map[0] = (0, false);
                self.bank_map[1] = (0, false);
                self.bank_map[2] = (1, false);
                self.bank_map[3] = (1, false);
            }
            MirrorType::FourScreen => {
                self.bank_map[0] = (0, false);
                self.bank_map[1] = (1, false);
                self.bank_map[2] = (2, false);
                self.bank_map[3] = (3, false);
            }
            MirrorType::SingleScreen(n) => {
                let bank = n as i16;
                self.bank_map[0] = (bank, false);
                self.bank_map[1] = (bank, false);
                self.bank_map[2] = (bank, false);
                self.bank_map[3] = (bank, false);
            }
        }
        self.bank_map[4] = self.bank_map[0];
        self.bank_map[5] = self.bank_map[1];
        self.bank_map[6] = self.bank_map[2];
        self.bank_map[7] = self.bank_map[3];
    }

    pub fn set_bank(&mut self, page: usize, bank: i16) {
        self.bank_map[page] = (bank, self.bank_map[page].1);
    }

    pub fn select_alternate_memory(&mut self, page: usize, alternate: bool) {
        self.bank_map[page] = (self.bank_map[page].0, alternate);
    }

    pub fn get_bank(&mut self, page: usize) -> i16 {
        self.bank_map[page].0
    }

    fn convert(&self, addr: u16) -> (usize, bool) {
        let raw_index = (addr - self.start_address) as usize;
        let page = raw_index / self.bank_size;
        assert!(
            page < self.page_count,
            "{:?} page too big addr {:#0x} (index {:#0x}) {} >= {}. Bank size is {}",
            self.memory_type,
            addr,
            raw_index,
            page,
            self.page_count,
            self.bank_size,
        );
        let bank = self.bank_map[page];

        let base = if bank.0 >= 0 {
            let bank_count = if bank.1 {
                self.alternate_bank_count
            } else {
                self.bank_count
            };
            (bank.0 as usize % bank_count) * self.bank_size
        } else {
            self.memory
                .len()
                .wrapping_sub((-bank.0) as usize * self.bank_size)
        };
        let offset = raw_index % self.bank_size;

        if bank.1 {
            ((base + offset) % self.alternate_memory.len(), true)
        } else {
            ((base + offset) % self.memory.len(), false)
        }
    }

    pub fn contains_addr(&self, addr: u16) -> bool {
        self.start_address <= addr && addr <= self.end_address
    }
}

fn k_to_usize(k: u16) -> usize {
    (k as usize) * 1024
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MemoryType {
    SRAM,
    VRAM,
    CHR_RAM,
    CHR_ROM,
    PRG_ROM,
    ROM_EXPANSION,
}
