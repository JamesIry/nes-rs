#[cfg(test)]
mod unit_tests;

use super::MirrorType;

const MAX_BANKS: usize = 8;
pub struct MemoryRegion {
    pub memory_type: MemoryType,
    pub memory: Vec<u8>,

    pub start_address: u16,
    pub end_address: u16,

    bank_size: usize,
    page_count: usize,
    bank_count: usize,
    bank_map: [i16; MAX_BANKS],

    write_protect: bool,
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
            start_address,
            end_address,
            write_protect,

            bank_size: 0,
            page_count: 0,
            bank_count: 0,
            bank_map: [0; MAX_BANKS],
        };
        result.set_bank_size(result.get_address_size().min(result.memory.len()));
        result
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.memory[self.convert(addr)]
    }

    pub fn write(&mut self, addr: u16, value: u8) -> u8 {
        let converted = self.convert(addr);
        let old = self.memory[converted];
        if !self.write_protect {
            self.memory[converted] = value;
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
    }

    pub fn set_mirror_type(&mut self, mirror_type: MirrorType) {
        match mirror_type {
            MirrorType::Vertical => {
                self.bank_map[0] = 0;
                self.bank_map[1] = 1;
                self.bank_map[2] = 0;
                self.bank_map[3] = 1;
            }
            MirrorType::Horizontal => {
                self.bank_map[0] = 0;
                self.bank_map[1] = 0;
                self.bank_map[2] = 1;
                self.bank_map[3] = 1;
            }
            MirrorType::FourScreen => {
                self.bank_map[0] = 0;
                self.bank_map[1] = 1;
                self.bank_map[2] = 2;
                self.bank_map[3] = 3;
            }
            MirrorType::SingleScreen(n) => {
                let bank = n as i16;
                self.bank_map[0] = bank;
                self.bank_map[1] = bank;
                self.bank_map[2] = bank;
                self.bank_map[3] = bank;
            }
        }
        self.bank_map[4] = self.bank_map[0];
        self.bank_map[5] = self.bank_map[1];
        self.bank_map[6] = self.bank_map[2];
        self.bank_map[7] = self.bank_map[3];
    }

    pub fn set_bank(&mut self, page: usize, bank: i16) {
        self.bank_map[page] = bank;
    }

    pub fn get_bank(&mut self, page: usize) -> i16 {
        self.bank_map[page]
    }

    fn convert(&self, addr: u16) -> usize {
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

        let base = if bank >= 0 {
            (bank as usize % self.bank_count) * self.bank_size
        } else {
            self.memory
                .len()
                .wrapping_sub((-bank) as usize * self.bank_size)
        };
        let offset = raw_index % self.bank_size;

        (base + offset) % self.memory.len()
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
