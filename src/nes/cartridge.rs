mod mappers;

#[cfg(test)]
mod unit_tests;

use mappers::nrom::NRom;

use anyhow::Result;
use std::{cell::RefCell, fs::File, io::BufReader, io::Read, rc::Rc};
use thiserror::Error;

use mappers::{CartridgeCpuLocation, Mapper};

use crate::{
    bus::{BusDevice, InterruptFlags},
    nes::cartridge::mappers::{
        axrom::AxRom, cnrom::CNRom, color_dreams::ColorDreams, hvc_un1rom::HvcUN1Rom, uxrom::UxRom,
        uxrom_invert::UxRomInvert,
    },
};

use self::mappers::NulMapper;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[allow(dead_code)]
pub enum MirrorType {
    Vertical,
    Horizontal,
    FourScreen,
    SingleScreen(u8),
}

pub struct Cartridge {
    sram: Vec<u8>,
    sram_addr_mask: usize,
    #[allow(dead_code)]
    sram_is_persistent: bool,
    trainer: bool,
    trainer_ram: Vec<u8>,
    prg_rom: Vec<u8>,
    prg_rom_mask: usize,
    chr_rom: Vec<u8>,
    chr_rom_mask: usize,
    mapper: Box<dyn Mapper>,
    vram: Vec<u8>,
}

static NES_TAG: [u8; 4] = [b'N', b'E', b'S', 0x1A];
static SRAM_PAGE_SIZE: usize = 0x2000;
static PRG_ROM_PAGE_SIZE: usize = 0x4000;
static CHR_ROM_PAGE_SIZE: usize = 0x2000;
// NES VRAM is actually half this, 0x0800, but by doubling we can use some 4 screen
// mappers without building more ram into the cart.
// mirror_vram() takes care of making sure we don't see memory outside the range we should
// be for Vertical, Horizontal, and Single
static VRAM_SIZE: usize = 0x1000;

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
            (false, false) => MirrorType::Horizontal,
            (false, true) => MirrorType::Vertical,
            (true, false) => MirrorType::FourScreen,
            (true, true) => MirrorType::SingleScreen(0),
        };

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

        let prg_rom_mask = prg_rom_size - 1;
        let chr_rom_mask = chr_rom_size - 1;
        let sram_addr_mask = sram_size - 1;

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

        let vram = vec![0; VRAM_SIZE];

        let mapper_number = (header[7] & 0xF0) | (header[6] >> 4);
        let mapper: Box<dyn Mapper> = match mapper_number {
            0 => Box::new(NRom::new(screen_mirroring)),
            2 => Box::new(UxRom::new(screen_mirroring, prg_rom_size)),
            3 => Box::new(CNRom::new(screen_mirroring)),
            7 => Box::new(AxRom::new()),
            11 => Box::new(ColorDreams::new(screen_mirroring)),
            94 => Box::new(HvcUN1Rom::new(screen_mirroring, prg_rom_size)),
            180 => Box::new(UxRomInvert::new(screen_mirroring)),
            _ => Err(CartridgeError::UnsupportedMapper(mapper_number))?,
        };

        println!("sram_size {:#06x} | sram mask {:#06x} | peristence {} | trainer {} | prg size {:#06x} | prg mask {:#06x} | chr size {:#06x} | chr mask {:#06x} | screen mirroring {:?} | mapper {}",
        sram_size, sram_addr_mask, sram_is_persistent, trainer, prg_rom_size, prg_rom_mask, chr_rom_size, chr_rom_mask, screen_mirroring, mapper_number);

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

    fn mirror_vram(&self, addr: usize) -> usize {
        let raw_index = addr & 0xFFF; // mirror 0x2000-0x3FFF down to 0x0000 - 0x01FFF by turning off some bits

        let name_table_requested = (raw_index >> 10) & 0b11;

        let name_table_selected = match (self.mapper.mirror_type(), name_table_requested) {
            (MirrorType::Horizontal, 0) => 0,
            (MirrorType::Horizontal, 1) => 0,
            (MirrorType::Horizontal, 2) => 1,
            (MirrorType::Horizontal, 3) => 1,
            (MirrorType::Vertical, 0) => 0,
            (MirrorType::Vertical, 1) => 1,
            (MirrorType::Vertical, 2) => 0,
            (MirrorType::Vertical, 3) => 1,
            (MirrorType::SingleScreen(n), _) => n as usize,
            (MirrorType::FourScreen, n) => n,
            (m, n) => unreachable!("Invalid miror and nametable {:?} {}", m, n),
        };

        (raw_index & !0b00110000000000) | (name_table_selected << 10)
    }

    fn translate_cpu_addr(&mut self, addr: usize) -> CartridgeCpuLocation {
        if self.trainer && (0x7000..=0x71FF).contains(&addr) {
            CartridgeCpuLocation::Trainer(addr - 0x7000)
        } else {
            self.mapper.translate_cpu_addr(addr)
        }
    }

    pub fn read_cpu(&mut self, addr: u16) -> u8 {
        let location = self.translate_cpu_addr(addr as usize);
        match location {
            mappers::CartridgeCpuLocation::SRam(addr) => self.sram[addr & self.sram_addr_mask],
            mappers::CartridgeCpuLocation::Trainer(addr) => self.trainer_ram[addr & 0x01FF],
            mappers::CartridgeCpuLocation::PrgRom(addr) => self.prg_rom[addr & self.prg_rom_mask],
            mappers::CartridgeCpuLocation::None => {
                panic!("CPU address out of range in cart {}", addr)
            }
        }
    }

    pub fn write_cpu(&mut self, addr: u16, data: u8) -> u8 {
        let location = self.translate_cpu_addr(addr as usize);
        match location {
            mappers::CartridgeCpuLocation::SRam(addr) => {
                let old = self.sram[addr & self.sram_addr_mask];
                self.sram[addr & self.sram_addr_mask] = data;
                old
            }
            mappers::CartridgeCpuLocation::Trainer(addr) => {
                let old = self.trainer_ram[addr & 0x01FF];
                self.trainer_ram[addr & 0x01FF] = data;
                old
            }
            mappers::CartridgeCpuLocation::PrgRom(_) => self.mapper.configure(addr, data),
            mappers::CartridgeCpuLocation::None => {
                panic!("CPU address out of range in cart {}", addr)
            }
        }
    }

    pub fn read_ppu(&mut self, addr: u16) -> u8 {
        let location = self.mapper.translate_ppu_addr(addr as usize);
        match location {
            mappers::CartridgePpuLocation::ChrRom(addr) => {
                let physical = addr & self.chr_rom_mask;
                self.chr_rom[physical]
            }
            mappers::CartridgePpuLocation::VRam(addr) => {
                let physical = self.mirror_vram(addr);
                self.vram[physical]
            }
            mappers::CartridgePpuLocation::None => {
                panic!("PPU address out of range in cart {}", addr)
            }
        }
    }

    pub fn write_ppu(&mut self, addr: u16, data: u8) -> u8 {
        let location = self.mapper.translate_ppu_addr(addr as usize);
        match location {
            mappers::CartridgePpuLocation::ChrRom(addr) => {
                let physical = addr & self.chr_rom_mask;
                let old = self.chr_rom[physical];
                self.chr_rom[physical] = data;
                old
            }
            mappers::CartridgePpuLocation::VRam(addr) => {
                let physical = self.mirror_vram(addr);
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

    fn bus_clock(&mut self) -> InterruptFlags {
        self.cartridge.as_ref().borrow_mut().mapper.cpu_bus_clock()
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

    fn bus_clock(&mut self) -> crate::bus::InterruptFlags {
        self.cartridge.as_ref().borrow_mut().mapper.ppu_bus_clock();
        InterruptFlags::empty()
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
