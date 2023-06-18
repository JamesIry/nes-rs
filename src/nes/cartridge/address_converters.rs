#[cfg(test)]
mod unit_tests;

use super::MirrorType;

pub trait AddressConverter {
    fn convert(&self, addr: u16) -> usize;
    fn convert_from_bank(&self, bank_number: i16, addr: u16) -> usize;
    fn contains_addr(&self, addr: u16) -> bool;
}

pub struct BankedConverter {
    pub bank: u8,
    pub start_address: u16,
    pub end_address: u16,
    pub bank_size: u16,
    pub window_size: u16,
    pub max_size: usize,
}

impl BankedConverter {
    pub fn new(
        start_address: u16,
        end_address: u16,
        bank_size: u16,
        window_size: u16,
        max_size: usize,
    ) -> Self {
        Self {
            bank: 0,
            start_address,
            end_address,
            bank_size,
            window_size,
            max_size,
        }
    }
}

impl AddressConverter for BankedConverter {
    fn convert(&self, addr: u16) -> usize {
        self.convert_from_bank(self.bank as i16, addr)
    }

    fn convert_from_bank(&self, bank_number: i16, addr: u16) -> usize {
        let base = if bank_number >= 0 {
            bank_number as usize * k_to_usize(self.bank_size)
        } else {
            self.max_size
                .wrapping_sub((-bank_number) as usize * k_to_usize(self.bank_size))
        };
        let offset = ((addr - self.start_address) as usize) % k_to_usize(self.window_size);

        base + offset
    }

    fn contains_addr(&self, addr: u16) -> bool {
        self.start_address <= addr && addr <= self.end_address
    }
}

pub struct MirroredConverter {
    pub mirror_type: MirrorType,
    pub start_address: u16,
    pub end_address: u16,
    pub bank_size: u16,
    pub window_size: u16,
    pub max_size: usize,
}

impl MirroredConverter {
    pub fn new(
        mirror_type: MirrorType,
        start_address: u16,
        end_address: u16,
        bank_size: u16,
        window_size: u16,
        max_size: usize,
    ) -> Self {
        Self {
            mirror_type,
            start_address,
            end_address,
            bank_size,
            window_size,
            max_size,
        }
    }
}

impl AddressConverter for MirroredConverter {
    fn convert(&self, addr: u16) -> usize {
        let name_table_requested = (addr >> 10) & 0b11;

        let name_table_selected = match (self.mirror_type, name_table_requested) {
            (MirrorType::Horizontal, 0) => 0,
            (MirrorType::Horizontal, 1) => 0,
            (MirrorType::Horizontal, 2) => 1,
            (MirrorType::Horizontal, 3) => 1,
            (MirrorType::Vertical, 0) => 0,
            (MirrorType::Vertical, 1) => 1,
            (MirrorType::Vertical, 2) => 0,
            (MirrorType::Vertical, 3) => 1,
            (MirrorType::SingleScreen(n), _) => n as u16,
            (MirrorType::FourScreen, n) => n,
            (m, n) => unreachable!("Invalid miror and nametable {:?} {}", m, n),
        };

        self.convert_from_bank(name_table_selected as i16, addr)
    }

    fn convert_from_bank(&self, bank_number: i16, addr: u16) -> usize {
        let base = if bank_number >= 0 {
            bank_number as usize * k_to_usize(self.bank_size)
        } else {
            self.max_size
                .wrapping_sub((-bank_number) as usize * k_to_usize(self.bank_size))
        };
        let offset = ((addr - self.start_address) as usize) % k_to_usize(self.window_size);

        base + offset
    }

    fn contains_addr(&self, addr: u16) -> bool {
        self.start_address <= addr && addr <= self.end_address
    }
}

fn k_to_usize(k: u16) -> usize {
    (k as usize) * 1024
}
