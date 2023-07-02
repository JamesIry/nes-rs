use anyhow::Result;

use crate::bus::InterruptFlags;

use self::{
    axrom::AxRom, cnrom::CNRom, color_dreams::ColorDreams, hvc_un1rom::HvcUN1Rom, mmc1::MMC1,
    mmc3::MMC3, mmc3_tqrom::MMC3TQRom, mmc3_tsxrom::MMC3TxSRom, namcot_108::Namcot108,
    namcot_3425::Namcot3425, namcot_3443::Namcot3443, namcot_3446::Namcot3446,
    namcot_3453::Namcot3453, nes_event::NesEvent, nrom::NRom, uxrom::UxRom,
    uxrom_invert::UxRomInvert,
};

use super::{CartridgeCore, CartridgeError};

pub mod axrom;
pub mod cnrom;
pub mod color_dreams;
pub mod hvc_un1rom;
pub mod mmc1;
pub mod mmc3;
pub mod mmc3_irq;
pub mod mmc3_tqrom;
pub mod mmc3_tsxrom;
pub mod namcot_108;
pub mod namcot_3425;
pub mod namcot_3443;
pub mod namcot_3446;
pub mod namcot_3453;
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
        76 => Box::new(Namcot3446::new(core)),
        88 => Box::new(Namcot3443::new(core)),
        94 => Box::new(HvcUN1Rom::new(core)),
        95 => Box::new(Namcot3425::new(core)),
        105 => Box::new(NesEvent::new(core, 0b0100)),
        118 => Box::new(MMC3TxSRom::new(core)),
        119 => Box::new(MMC3TQRom::new(core)),
        154 => Box::new(Namcot3453::new(core)),
        155 => Box::new(MMC1::new(core, true)),
        180 => Box::new(UxRomInvert::new(core)),
        185 => Box::new(CNRom::new(core, true)),
        206 => Box::new(Namcot108::new(core)),
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
