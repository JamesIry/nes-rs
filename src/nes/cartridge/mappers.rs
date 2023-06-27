use anyhow::Result;

use crate::bus::InterruptFlags;

use self::{
    axrom::AxRom, cnrom::CNRom, color_dreams::ColorDreams, hvc_un1rom::HvcUN1Rom, mmc1::MMC1,
    mmc3::MMC3, nes_event::NesEvent, nrom::NRom, uxrom::UxRom, uxrom_invert::UxRomInvert,
};

use super::{CartridgeCore, CartridgeError};

pub mod axrom;
pub mod cnrom;
pub mod color_dreams;
pub mod hvc_un1rom;
pub mod mmc1;
pub mod mmc3;
pub mod nes_event;
pub mod nrom;
pub mod uxrom;
pub mod uxrom_invert;

pub fn get_mapper(mapper_number: u8, core: CartridgeCore) -> Result<Box<dyn Mapper>> {
    let mapper: Box<dyn Mapper> = match mapper_number {
        0 => Box::new(NRom::new(core)),
        1 => Box::new(MMC1::new(core, false)),
        2 => Box::new(UxRom::new(core)),
        3 => Box::new(CNRom::new(core, false)),
        4 => Box::new(MMC3::new(core)),
        7 => Box::new(AxRom::new(core)),
        11 => Box::new(ColorDreams::new(core)),
        94 => Box::new(HvcUN1Rom::new(core)),
        105 => Box::new(NesEvent::new(core, 0b0100)),
        180 => Box::new(UxRomInvert::new(core)),
        185 => Box::new(CNRom::new(core, true)),
        _ => Err(CartridgeError::UnsupportedMapper(mapper_number))?,
    };
    Ok(mapper)
}
pub trait Mapper {
    fn cpu_bus_clock(&mut self) -> InterruptFlags;
    fn read_cpu(&mut self, addr: u16) -> u8;
    fn write_cpu(&mut self, addr: u16, value: u8) -> u8;

    fn ppu_bus_clock(&mut self);
    fn read_ppu(&mut self, addr: u16) -> u8;
    fn write_ppu(&mut self, addr: u16, value: u8) -> u8;

    fn core(&self) -> &CartridgeCore;
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

    fn core(&self) -> &CartridgeCore {
        unreachable!()
    }
}
