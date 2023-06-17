use crate::nes::cartridge::{mappers::nrom::NRom, Cartridge, MirrorType};

#[test]
fn test_vertical_mirroring() {
    let mut c = Cartridge::nul_cartridge();
    let mapper = NRom::new(MirrorType::Vertical);
    c.mapper = Box::new(mapper);

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
    let mapper = NRom::new(MirrorType::Horizontal);
    c.mapper = Box::new(mapper);

    assert_eq!(0x0000, c.mirror_vram(0x2000));

    assert_eq!(0x0399, c.mirror_vram(0x2399));

    assert_eq!(0x0000, c.mirror_vram(0x2400));

    assert_eq!(0x0399, c.mirror_vram(0x2799));

    assert_eq!(0x0400, c.mirror_vram(0x2800));

    assert_eq!(0x0799, c.mirror_vram(0x2B99));

    assert_eq!(0x0400, c.mirror_vram(0x2C00));

    assert_eq!(0x0799, c.mirror_vram(0x2F99));
}
