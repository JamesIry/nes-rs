mod address_converters;
mod mappers;

use anyhow::Result;
use std::{cell::RefCell, fs::File, io::BufReader, io::Read, rc::Rc};
use thiserror::Error;

use mappers::Mapper;

use crate::bus::{BusDevice, InterruptFlags};

use self::{
    address_converters::{AddressConverter, BankedConverter, MirroredConverter},
    mappers::NulMapper,
};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[allow(dead_code)]
pub enum MirrorType {
    Vertical,
    Horizontal,
    FourScreen,
    SingleScreen(u8),
}

pub struct Cartridge {}

static NES_TAG: [u8; 4] = [b'N', b'E', b'S', 0x1A];
static SRAM_PAGE_SIZE: usize = 0x2000;
static DEFAULT_SRAM_SIZE: usize = 0x8000;
static PRG_ROM_PAGE_SIZE: usize = 0x4000;
static CHR_ROM_PAGE_SIZE: usize = 0x2000;
// NES VRAM is actually half this, 0x0800, but by doubling we can use some 4 screen
// mappers without building more ram into the cart.
// mirror_vram() takes care of making sure we don't see memory outside the range we should
// be for Vertical, Horizontal, and Single
static VRAM_SIZE: usize = 0x2000;

impl Cartridge {
    pub fn load(file_name: &str) -> Result<Box<dyn Mapper>> {
        let file = File::open(file_name)?;

        let mut reader = BufReader::new(file);
        let mut header = [0; 16];

        reader.read_exact(&mut header)?;

        let nes_header = NesHeader::new(&header)?;

        let rom_expansion_size = 0x2000;
        let rom_expansion_vec = vec![0; rom_expansion_size];
        let rom_expansion = MemoryRegion::new(rom_expansion_vec, 0x4000, 0x5FFF, 1, true);

        let mut sram_vec = vec![0; nes_header.sram_size];
        if nes_header.has_trainer {
            let mut trainer_ram = vec![0; 512];
            reader.read_exact(&mut trainer_ram)?;
            sram_vec[0x7000..(0x7000 + 512)].copy_from_slice(&trainer_ram[..512]);
        }
        let sram = MemoryRegion::new(
            sram_vec,
            0x6000,
            0x7FFF,
            8.min(nes_header.sram_size / 1024) as u16,
            false,
        );

        let mut prg_rom_vec = vec![0; nes_header.prg_rom_size];
        reader.read_exact(&mut prg_rom_vec)?;
        let prg_rom = MemoryRegion::new(
            prg_rom_vec,
            0x8000,
            0xFFFF,
            32.min(nes_header.prg_rom_size / 1024) as u16,
            true,
        );

        let mut chr_rom_vec = vec![0; nes_header.chr_rom_size];
        if nes_header.chr_is_rom {
            reader.read_exact(&mut chr_rom_vec)?;
        }
        let chr_ram = MemoryRegion::new(
            chr_rom_vec,
            0x0000,
            0x1FFF,
            8.min(nes_header.chr_rom_size / 1024) as u16,
            nes_header.chr_is_rom,
        );

        // vram goes all the way to 0x3FFF even though palette ram occupies
        // 0x3F00-0x3FFF. That's because the PPU 'shadow' reads
        // vram even when reading palettes
        let vram_vec = vec![0; VRAM_SIZE];
        let vram =
            MemoryRegion::new_vram(vram_vec, 0x2000, 0x3FFF, 1, false, nes_header.mirror_type);

        let mapper_number = nes_header.mapper_number;

        let core = CartridgeCore {
            _nes_header: nes_header,
            rom_expansion,
            sram,
            prg_rom,
            chr_ram,
            vram,
        };

        mappers::get_mapper(mapper_number, core)
    }

    pub(crate) fn nul_cartridge() -> Box<dyn Mapper> {
        Box::new(NulMapper {})
    }
}

pub struct NesHeader {
    _ines_ver: u8,
    mirror_type: MirrorType,
    #[allow(dead_code)]
    sram_is_persistent: bool,
    chr_is_rom: bool,
    has_trainer: bool,
    prg_rom_size: usize,
    chr_rom_size: usize,
    sram_size: usize,
    mapper_number: u8,
}
impl NesHeader {
    fn new(header: &[u8; 16]) -> Result<NesHeader> {
        if header[0..4] != NES_TAG {
            Err(CartridgeError::UnrecognizedFileFormat)?;
        }

        let _ines_ver = (header[7] >> 2) & 0x03;
        if _ines_ver != 0 {
            Err(CartridgeError::UnsupportedInesVersion)?;
        }

        let four_screen = header[6] & 0x08 != 0;
        let has_trainer = header[6] & 0x04 != 0;
        let sram_is_persistent = header[6] & 0x02 != 0;
        let vertical_mirroring = header[6] & 0x01 != 0;

        let mirror_type = match (four_screen, vertical_mirroring) {
            (false, false) => MirrorType::Horizontal,
            (false, true) => MirrorType::Vertical,
            (true, false) => MirrorType::FourScreen,
            (true, true) => MirrorType::SingleScreen(0),
        };

        let prg_rom_size = (header[4] as usize) * PRG_ROM_PAGE_SIZE;
        let mut chr_rom_size = (header[5] as usize) * CHR_ROM_PAGE_SIZE;
        let chr_is_rom = if chr_rom_size == 0 {
            chr_rom_size = CHR_ROM_PAGE_SIZE;
            false
        } else {
            true
        };

        let mapper_number = (header[7] & 0xF0) | (header[6] >> 4);

        let mut sram_size = (header[8] as usize) * SRAM_PAGE_SIZE;
        if sram_size == 0 {
            sram_size = DEFAULT_SRAM_SIZE;
        }

        let chr_rom_ram = if chr_is_rom { "rom" } else { "ram" };
        println!("sram_size {:#06x} | peristence {} | trainer {} | prg rom size {:#06x} | chr {} size {:#06x} | screen mirroring {:?} | mapper {}",
        sram_size, sram_is_persistent, has_trainer, prg_rom_size, chr_rom_ram, chr_rom_size, mirror_type, mapper_number);

        Ok(Self {
            _ines_ver,
            mirror_type,
            sram_is_persistent,
            chr_is_rom,
            has_trainer,
            prg_rom_size,
            chr_rom_size,
            sram_size,
            mapper_number,
        })
    }
}

pub struct MemoryRegion<T>
where
    T: AddressConverter,
{
    memory: Vec<u8>,
    converter: T,
    write_protect: bool,
}

impl MemoryRegion<BankedConverter> {
    pub fn new(
        memory: Vec<u8>,
        start_address: u16,
        end_address: u16,
        bank_size_k: u16,
        write_protect: bool,
    ) -> MemoryRegion<BankedConverter> {
        let max_size = memory.len();
        MemoryRegion {
            memory,
            converter: BankedConverter::new(start_address, end_address, bank_size_k, max_size),
            write_protect,
        }
    }
}

impl MemoryRegion<MirroredConverter> {
    pub fn new_vram(
        memory: Vec<u8>,
        start_address: u16,
        end_address: u16,
        bank_size_k: u16,
        write_protect: bool,
        mirror_type: MirrorType,
    ) -> MemoryRegion<MirroredConverter> {
        let max_size = memory.len();
        MemoryRegion {
            memory,
            converter: MirroredConverter::new(
                mirror_type,
                start_address,
                end_address,
                bank_size_k,
                max_size,
            ),
            write_protect,
        }
    }
}

impl<T> MemoryRegion<T>
where
    T: AddressConverter,
{
    pub fn read(&self, addr: u16) -> u8 {
        self.memory[self.converter.convert(addr)]
    }

    pub fn read_from_bank(&self, bank: i16, addr: u16) -> u8 {
        let converted = self.converter.convert_from_bank(bank, addr);
        self.memory[converted]
    }

    pub fn write(&mut self, addr: u16, value: u8) -> u8 {
        let converted = self.converter.convert(addr);
        let old = self.memory[converted];
        if !self.write_protect {
            self.memory[converted] = value;
        }
        old
    }

    #[allow(dead_code)]
    pub fn write_to_bank(&mut self, bank: i16, addr: u16, value: u8) -> u8 {
        let converted = self.converter.convert_from_bank(bank, addr);
        let old = self.memory[converted];
        if !self.write_protect {
            self.memory[converted] = value;
        }
        old
    }
}

pub struct CartridgeCore {
    _nes_header: NesHeader,
    rom_expansion: MemoryRegion<BankedConverter>,
    sram: MemoryRegion<BankedConverter>,
    prg_rom: MemoryRegion<BankedConverter>,
    chr_ram: MemoryRegion<BankedConverter>,
    vram: MemoryRegion<MirroredConverter>,
}

impl CartridgeCore {
    fn read_cpu(&mut self, addr: u16) -> u8 {
        if self.sram.converter.contains_addr(addr) {
            self.sram.read(addr)
        } else if self.prg_rom.converter.contains_addr(addr) {
            self.prg_rom.read(addr)
        } else if self.rom_expansion.converter.contains_addr(addr) {
            self.rom_expansion.read(addr)
        } else {
            panic!("Unrecognized address {}", addr)
        }
    }
    fn write_cpu(&mut self, addr: u16, value: u8) -> u8 {
        if self.sram.converter.contains_addr(addr) {
            self.sram.write(addr, value)
        } else if self.prg_rom.converter.contains_addr(addr) {
            self.prg_rom.write(addr, value)
        } else if self.rom_expansion.converter.contains_addr(addr) {
            self.rom_expansion.write(addr, value)
        } else {
            panic!("Unrecognized address {}", addr)
        }
    }

    fn read_ppu(&mut self, addr: u16) -> u8 {
        if self.chr_ram.converter.contains_addr(addr) {
            self.chr_ram.read(addr)
        } else if self.vram.converter.contains_addr(addr) {
            self.vram.read(addr)
        } else {
            panic!("Unrecognized address {}", addr)
        }
    }
    fn write_ppu(&mut self, addr: u16, value: u8) -> u8 {
        if self.chr_ram.converter.contains_addr(addr) {
            self.chr_ram.write(addr, value)
        } else if self.vram.converter.contains_addr(addr) {
            self.vram.write(addr, value)
        } else {
            panic!("Unrecognized address {}", addr)
        }
    }
}

pub struct CartridgeCPUPort {
    cartridge: Rc<RefCell<Box<dyn Mapper>>>,
}

impl CartridgeCPUPort {
    pub fn new(cartridge: Rc<RefCell<Box<dyn Mapper>>>) -> Self {
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
        self.cartridge.borrow_mut().cpu_bus_clock()
    }
}

pub struct CartridgePPUPort {
    cartridge: Rc<RefCell<Box<dyn Mapper>>>,
}

impl CartridgePPUPort {
    pub fn new(cartridge: Rc<RefCell<Box<dyn Mapper>>>) -> Self {
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
        self.cartridge.borrow_mut().ppu_bus_clock();
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
