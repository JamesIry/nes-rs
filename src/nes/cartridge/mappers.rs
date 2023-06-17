use crate::bus::InterruptFlags;

use super::MirrorType;

pub mod axrom;
pub mod cnrom;
pub mod color_dreams;
pub mod hvc_un1rom;
pub mod nrom;
pub mod uxrom;
pub mod uxrom_invert;

pub trait Mapper {
    fn translate_cpu_addr(&mut self, addr: usize) -> CartridgeCpuLocation;
    fn translate_ppu_addr(&mut self, addr: usize) -> CartridgePpuLocation;
    fn mirror_type(&self) -> MirrorType;
    fn configure(&mut self, addr: u16, value: u8) -> u8;
    fn cpu_bus_clock(&mut self) -> InterruptFlags;
    fn ppu_bus_clock(&mut self);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CartridgeCpuLocation {
    None,
    SRam(usize),
    Trainer(usize),
    PrgRom(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CartridgePpuLocation {
    None,
    ChrRom(usize),
    VRam(usize),
}

pub struct NulMapper {}

impl Mapper for NulMapper {
    fn translate_cpu_addr(&mut self, _addr: usize) -> CartridgeCpuLocation {
        CartridgeCpuLocation::None
    }

    fn translate_ppu_addr(&mut self, _addr: usize) -> CartridgePpuLocation {
        CartridgePpuLocation::None
    }

    fn mirror_type(&self) -> MirrorType {
        MirrorType::FourScreen
    }

    fn configure(&mut self, _addr: u16, _value: u8) -> u8 {
        0
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }

    fn ppu_bus_clock(&mut self) {}
}

struct AddressConverter {
    bank: u8,
    base: usize,
    bank_size: u16,
    window_size: u16,
    max_size: Option<usize>,
}

impl AddressConverter {
    fn new(base: usize, bank_size: u16, window_size: u16, max_size: Option<usize>) -> Self {
        Self {
            bank: 0,
            base,
            bank_size,
            window_size,
            max_size,
        }
    }

    fn convert(&self, addr: usize) -> usize {
        self.convert_from_bank(self.bank as i16, addr)
    }

    fn convert_from_bank(&self, bank_number: i16, addr: usize) -> usize {
        let base = if bank_number >= 0 {
            bank_number as usize * k_to_usize(self.bank_size)
        } else {
            self.max_size
                .expect("Attempt to use negative bank number with no max size")
                .wrapping_sub((-bank_number) as usize * k_to_usize(self.bank_size))
        };
        let offset = (addr - self.base) % k_to_usize(self.window_size);

        base + offset
    }
}

fn k_to_usize(k: u16) -> usize {
    (k as usize) * 1024
}
