mod mappers;

use mappers::mapper0::Mapper0;

use anyhow::Result;
use std::{fs::File, io::BufReader, io::Read};
use thiserror::Error;

use mappers::{CartridgeCpuLocation, Mapper};

use crate::device::BusDevice;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum MirrorType {
    Vertical,
    Horizontal,
    FourScreen,
}

pub struct Cartridge {
    ram: Vec<u8>,
    ram_addr_mask: u16,
    #[allow(dead_code)]
    persistent_ram: bool,
    trainer: bool,
    trainer_ram: Vec<u8>,
    prg_rom: Vec<u8>,
    prg_rom_mask: u16,
    #[allow(dead_code)]
    chr_rom: Vec<u8>,
    #[allow(dead_code)]
    chr_rom_mask: u16,
    #[allow(dead_code)]
    screen_mirroring: MirrorType,
    mapper: Box<dyn Mapper>,
}

static NES_TAG: [u8; 4] = [b'N', b'E', b'S', 0x1A];
static RAM_PAGE_SIZE: usize = 0x2000;
static PRG_ROM_PAGE_SIZE: usize = 0x4000;
static CHR_ROM_PAGE_SIZE: usize = 0x2000;
static MAX_PRG_ROM_ADDRESSIBLE: usize = 0x8000;
static MAX_CHR_ROM_ADDRESSIBLE: usize = 0x4000;
static MAX_RAM_ADDRESSIBLE: usize = 0x2000;

impl Cartridge {
    pub fn load(file_name: &str) -> Result<Self> {
        let file = File::open(file_name)?;

        let mut reader = BufReader::new(file);
        let mut header = [0; 16];

        reader.read_exact(&mut header)?;

        if header[0..4] != NES_TAG {
            Err(CartridgeError::UnrecognizedFileFormat)?;
        }

        let ines_ver = (header[7] >> 2) & 0x03;
        if ines_ver != 0 {
            Err(CartridgeError::UnsupportedInesVersion)?;
        }

        let four_screen = header[6] & 0x08 != 0;
        let trainer = header[6] & 0x04 != 0;
        let persistent_ram = header[6] & 0x02 != 0;
        let vertical_mirroring = header[6] & 0x01 != 0;

        let screen_mirroring = match (four_screen, vertical_mirroring) {
            (true, _) => MirrorType::FourScreen,
            (false, true) => MirrorType::Vertical,
            (false, false) => MirrorType::Horizontal,
        };

        let mapper_number = (header[7] & 0xF0) | (header[6] >> 4);
        let mapper = Box::new(match mapper_number {
            0 => Mapper0::new(),
            _ => Err(CartridgeError::UnsupportedMapper(mapper_number))?,
        });

        let prg_rom_size = (header[4] as usize) * PRG_ROM_PAGE_SIZE;
        let chr_rom_size = (header[5] as usize) * CHR_ROM_PAGE_SIZE;
        let mut ram_size = (header[8] as usize) * RAM_PAGE_SIZE;
        if ram_size == 0 {
            ram_size = RAM_PAGE_SIZE;
        }

        let prg_rom_mask = (prg_rom_size.min(MAX_PRG_ROM_ADDRESSIBLE) - 1) as u16;
        let chr_rom_mask = (chr_rom_size.min(MAX_CHR_ROM_ADDRESSIBLE) - 1) as u16;
        let ram_addr_mask = (ram_size.min(MAX_RAM_ADDRESSIBLE) - 1) as u16;

        let trainer_ram = if trainer {
            let mut trainer_ram = vec![0; 512];
            reader.read_exact(&mut trainer_ram)?;
            trainer_ram
        } else {
            Vec::new()
        };

        let mut prg_rom = vec![0; prg_rom_size];
        reader.read_exact(&mut prg_rom)?;

        let mut chr_rom = vec![0; chr_rom_size];
        reader.read_exact(&mut chr_rom)?;

        let ram = vec![0; ram_size];

        //println!("ram_size {:#06x} | ram mask {:#06x} | peristence {} | trainer {} | prg size {:#06x} | prg mask {:#06x} | chr size {:#06x} | chr mask {:#06x} | screen mirroring {:?} ",
        //ram_size, ram_addr_mask, persistent_ram, trainer, prg_rom_size, prg_rom_mask, chr_rom_size, chr_rom_mask, screen_mirroring);

        Ok(Cartridge {
            ram,
            ram_addr_mask,
            persistent_ram,
            trainer,
            trainer_ram,
            prg_rom,
            prg_rom_mask,
            chr_rom,
            chr_rom_mask,
            screen_mirroring,
            mapper,
        })
    }

    fn translate_cpu_addr(&mut self, addr: u16) -> CartridgeCpuLocation {
        if self.trainer && (0x7000..=0x71FF).contains(&addr) {
            CartridgeCpuLocation::Trainer(addr - 0x7000)
        } else {
            self.mapper.translate_cpu_addr(addr)
        }
    }
}

impl BusDevice for Cartridge {
    fn read_from_cpu_bus(&mut self, addr: u16) -> Option<u8> {
        let location = self.translate_cpu_addr(addr);
        match location {
            mappers::CartridgeCpuLocation::None => None,
            mappers::CartridgeCpuLocation::Ram(addr) => {
                Some(self.ram[(addr & self.ram_addr_mask) as usize])
            }
            mappers::CartridgeCpuLocation::Trainer(addr) => {
                Some(self.trainer_ram[(addr & 0x01FF) as usize])
            }
            mappers::CartridgeCpuLocation::PrgRom(addr) => {
                Some(self.prg_rom[(addr & self.prg_rom_mask) as usize])
            }
        }
    }

    fn write_to_cpu_bus(&mut self, addr: u16, data: u8) -> Option<u8> {
        let location = self.translate_cpu_addr(addr);
        match location {
            mappers::CartridgeCpuLocation::None => None,
            mappers::CartridgeCpuLocation::Ram(addr) => {
                let old = self.ram[(addr & self.ram_addr_mask) as usize];
                self.ram[(addr & self.ram_addr_mask) as usize] = data;
                Some(old)
            }
            mappers::CartridgeCpuLocation::Trainer(addr) => {
                let old = self.trainer_ram[(addr & 0x01FF) as usize];
                self.trainer_ram[(addr & 0x01FF) as usize] = data;
                Some(old)
            }
            mappers::CartridgeCpuLocation::PrgRom(_) => None,
        }
    }
}

#[derive(Error, Debug)]
pub enum CartridgeError {
    #[error("The file loaded in the cartridge wasn't recognized as having NES format")]
    UnrecognizedFileFormat,
    #[error("The file loaded in the cartridge has an unsupported NES version")]
    UnsupportedInesVersion,
    #[error("The file loaded requires  mapper {0} which isn't supported yet")]
    UnsupportedMapper(u8),
}
