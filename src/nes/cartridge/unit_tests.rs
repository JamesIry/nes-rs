use crate::nes::cartridge::{mappers::nrom::NRom, Cartridge, MirrorType, NesHeader};

#[test]
fn test_vertical_mirroring() {
    let mut c = Cartridge::nul_cartridge();
    let mut nes_header = NesHeader::nul_header();
    nes_header.mirror_type = MirrorType::Vertical;
    let m = NRom::new(&nes_header);
    c.mapper = Box::new(m);

    assert_eq!(0x0000, c.mirror_vram(0x2000));

    assert_eq!(0x0399, c.mirror_vram(0x2399));

    assert_eq!(0x0400, c.mirror_vram(0x2400));

    assert_eq!(0x0799, c.mirror_vram(0x2799));

    assert_eq!(0x0000, c.mirror_vram(0x2800));

    assert_eq!(0x0399, c.mirror_vram(0x2B99));

    assert_eq!(0x0400, c.mirror_vram(0x2C00));

    assert_eq!(0x0799, c.mirror_vram(0x2F99));
}

#[test]
fn test_horizontal_mirroring() {
    let mut c = Cartridge::nul_cartridge();
    let mut nes_header = NesHeader::nul_header();
    nes_header.mirror_type = MirrorType::Horizontal;
    let m = NRom::new(&nes_header);
    c.mapper = Box::new(m);

    assert_eq!(0x0000, c.mirror_vram(0x2000));

    assert_eq!(0x0399, c.mirror_vram(0x2399));

    assert_eq!(0x0000, c.mirror_vram(0x2400));

    assert_eq!(0x0399, c.mirror_vram(0x2799));

    assert_eq!(0x0400, c.mirror_vram(0x2800));

    assert_eq!(0x0799, c.mirror_vram(0x2B99));

    assert_eq!(0x0400, c.mirror_vram(0x2C00));

    assert_eq!(0x0799, c.mirror_vram(0x2F99));
}
