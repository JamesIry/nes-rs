use crate::bus::InterruptFlags;

pub mod axrom;
pub mod cnrom;
pub mod color_dreams;
pub mod hvc_un1rom;
pub mod nrom;
pub mod uxrom;
pub mod uxrom_invert;

pub trait Mapper {
    fn cpu_bus_clock(&mut self) -> InterruptFlags;
    fn read_cpu(&mut self, addr: u16) -> u8;
    fn write_cpu(&mut self, addr: u16, value: u8) -> u8;

    fn ppu_bus_clock(&mut self);
    fn read_ppu(&mut self, addr: u16) -> u8;
    fn write_ppu(&mut self, addr: u16, value: u8) -> u8;
}
pub struct NulMapper {}

impl Mapper for NulMapper {
    fn read_cpu(&mut self, _addr: u16) -> u8 {
        0
    }
    fn write_cpu(&mut self, _addr: u16, _value: u8) -> u8 {
        0
    }

    fn read_ppu(&mut self, _addr: u16) -> u8 {
        0
    }
    fn write_ppu(&mut self, _addr: u16, _value: u8) -> u8 {
        0
    }

    fn cpu_bus_clock(&mut self) -> InterruptFlags {
        InterruptFlags::empty()
    }

    fn ppu_bus_clock(&mut self) {}
}
