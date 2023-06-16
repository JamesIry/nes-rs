mod mappers;

use mappers::mapper0::Mapper0;

use anyhow::Result;
use std::{cell::RefCell, fs::File, io::BufReader, io::Read, rc::Rc};
use thiserror::Error;

use mappers::{CartridgeCpuLocation, Mapper};

use crate::bus::BusDevice;

use self::mappers::NulMapper;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum MirrorType {
    Vertical,
    Horizontal,
    FourScreen,
}

pub struct Cartridge {
    sram: Vec<u8>,
    sram_addr_mask: u16,
    #[allow(dead_code)]
    sram_is_persistent: bool,
    trainer: bool,
    trainer_ram: Vec<u8>,
    prg_rom: Vec<u8>,
    prg_rom_mask: u16,
    chr_rom: Vec<u8>,
    chr_rom_mask: u16,
    mapper: Box<dyn Mapper>,
    vram: Vec<u8>,
}

static NES_TAG: [u8; 4] = [b'N', b'E', b'S', 0x1A];
static SRAM_PAGE_SIZE: usize = 0x2000;
static PRG_ROM_PAGE_SIZE: usize = 0x4000;
static CHR_ROM_PAGE_SIZE: usize = 0x2000;
static MAX_PRG_ROM_ADDRESSIBLE: usize = 0x8000;
static MAX_CHR_ROM_ADDRESSIBLE: usize = 0x4000;
static MAX_SRAM_ADDRESSIBLE: usize = 0x2000;
static VRAM_SIZE: usize = 0x0800;

impl Cartridge {
    pub fn nul_cartridge() -> Self {
        Self {
            sram: Vec::new(),
            sram_addr_mask: 0,
            sram_is_persistent: false,
            trainer: false,
            trainer_ram: Vec::new(),
            prg_rom: Vec::new(),
            prg_rom_mask: 0,
            chr_rom: Vec::new(),
            chr_rom_mask: 0,
            mapper: Box::new(NulMapper {}),
            vram: Vec::new(),
        }
    }

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
        let sram_is_persistent = header[6] & 0x02 != 0;
        let vertical_mirroring = header[6] & 0x01 != 0;

        let screen_mirroring = match (four_screen, vertical_mirroring) {
            (true, _) => MirrorType::FourScreen,
            (false, true) => MirrorType::Vertical,
            (false, false) => MirrorType::Horizontal,
        };

        let mapper_number = (header[7] & 0xF0) | (header[6] >> 4);
        let mapper = Box::new(match mapper_number {
            0 => Mapper0::new(screen_mirroring),
            _ => Err(CartridgeError::UnsupportedMapper(mapper_number))?,
        });

        let prg_rom_size = (header[4] as usize) * PRG_ROM_PAGE_SIZE;
        let mut chr_rom_size = (header[5] as usize) * CHR_ROM_PAGE_SIZE;
        let chr_ram = if chr_rom_size == 0 {
            chr_rom_size = CHR_ROM_PAGE_SIZE;
            true
        } else {
            false
        };
        let mut sram_size = (header[8] as usize) * SRAM_PAGE_SIZE;
        if sram_size == 0 {
            sram_size = SRAM_PAGE_SIZE;
        }

        let prg_rom_mask = (prg_rom_size.min(MAX_PRG_ROM_ADDRESSIBLE) - 1) as u16;
        let chr_rom_mask = (chr_rom_size.min(MAX_CHR_ROM_ADDRESSIBLE) - 1) as u16;
        let sram_addr_mask = (sram_size.min(MAX_SRAM_ADDRESSIBLE) - 1) as u16;

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
        if !chr_ram {
            reader.read_exact(&mut chr_rom)?;
        }

        let sram = vec![0; sram_size];

        //println!("sram_size {:#06x} | sram mask {:#06x} | peristence {} | trainer {} | prg size {:#06x} | prg mask {:#06x} | chr size {:#06x} | chr mask {:#06x} | screen mirroring {:?} ",
        //sram_size, sram_addr_mask, sram_is_persistent, trainer, prg_rom_size, prg_rom_mask, chr_rom_size, chr_rom_mask, screen_mirroring);

        let vram = vec![0; VRAM_SIZE];

        Ok(Cartridge {
            sram,
            sram_addr_mask,
            sram_is_persistent,
            trainer,
            trainer_ram,
            prg_rom,
            prg_rom_mask,
            chr_rom,
            chr_rom_mask,
            mapper,
            vram,
        })
    }

    fn translate_cpu_addr(&mut self, addr: u16) -> CartridgeCpuLocation {
        if self.trainer && (0x7000..=0x71FF).contains(&addr) {
            CartridgeCpuLocation::Trainer(addr - 0x7000)
        } else {
            self.mapper.translate_cpu_addr(addr)
        }
    }

    pub fn read_cpu(&mut self, addr: u16) -> u8 {
        let location = self.translate_cpu_addr(addr);
        match location {
            mappers::CartridgeCpuLocation::None => {
                panic!("CPU address out of range in cart {}", addr)
            }
            mappers::CartridgeCpuLocation::SRam(addr) => {
                self.sram[(addr & self.sram_addr_mask) as usize]
            }
            mappers::CartridgeCpuLocation::Trainer(addr) => {
                self.trainer_ram[(addr & 0x01FF) as usize]
            }
            mappers::CartridgeCpuLocation::PrgRom(addr) => {
                self.prg_rom[(addr & self.prg_rom_mask) as usize]
            }
        }
    }

    pub fn write_cpu(&mut self, addr: u16, data: u8) -> u8 {
        let location = self.translate_cpu_addr(addr);
        match location {
            mappers::CartridgeCpuLocation::None => {
                panic!("CPU address out of range in cart {}", addr)
            }
            mappers::CartridgeCpuLocation::SRam(addr) => {
                let old = self.sram[(addr & self.sram_addr_mask) as usize];
                self.sram[(addr & self.sram_addr_mask) as usize] = data;
                old
            }
            mappers::CartridgeCpuLocation::Trainer(addr) => {
                let old = self.trainer_ram[(addr & 0x01FF) as usize];
                self.trainer_ram[(addr & 0x01FF) as usize] = data;
                old
            }
            mappers::CartridgeCpuLocation::PrgRom(_) => 0,
        }
    }

    pub fn read_ppu(&mut self, addr: u16) -> u8 {
        let location = self.mapper.translate_ppu_addr(addr);
        match location {
            mappers::CartridgePpuLocation::ChrRom(addr) => {
                let physical = (addr & self.chr_rom_mask) as usize;
                self.chr_rom[physical]
            }
            mappers::CartridgePpuLocation::VRam(addr) => self.vram[addr as usize],
            mappers::CartridgePpuLocation::None => {
                panic!("PPU address out of range in cart {}", addr)
            }
        }
    }

    pub fn write_ppu(&mut self, addr: u16, data: u8) -> u8 {
        let location = self.mapper.translate_ppu_addr(addr);
        match location {
            mappers::CartridgePpuLocation::ChrRom(addr) => {
                let physical = (addr & self.chr_rom_mask) as usize;
                let old = self.chr_rom[physical];
                self.chr_rom[physical] = data;
                old
            }
            mappers::CartridgePpuLocation::VRam(addr) => {
                let physical = addr as usize;
                let old = self.vram[physical];
                self.vram[physical] = data;
                old
            }
            mappers::CartridgePpuLocation::None => {
                panic!("PPU address out of range in cart {}", addr)
            }
        }
    }
}

pub struct CartridgeCPUPort {
    cartridge: Rc<RefCell<Cartridge>>,
}

impl CartridgeCPUPort {
    pub fn new(cartridge: Rc<RefCell<Cartridge>>) -> Self {
        Self { cartridge }
    }
}

impl BusDevice for CartridgeCPUPort {
    fn read(&mut self, addr: u16) -> u8 {
        self.cartridge.borrow_mut().read_cpu(addr)
    }

    fn write(&mut self, addr: u16, data: u8) -> u8 {
        self.cartridge.borrow_mut().write_cpu(addr, data)
    }

    fn get_address_range(&self) -> (u16, u16) {
        (0x4020, 0xFFFF)
    }
}

pub struct CartridgePPUPort {
    cartridge: Rc<RefCell<Cartridge>>,
}

impl CartridgePPUPort {
    pub fn new(cartridge: Rc<RefCell<Cartridge>>) -> Self {
        Self { cartridge }
    }
}

impl BusDevice for CartridgePPUPort {
    fn read(&mut self, addr: u16) -> u8 {
        self.cartridge.borrow_mut().read_ppu(addr)
    }

    fn write(&mut self, addr: u16, data: u8) -> u8 {
        self.cartridge.borrow_mut().write_ppu(addr, data)
    }

    fn get_address_range(&self) -> (u16, u16) {
        (0x0000, 0x3EFF)
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
