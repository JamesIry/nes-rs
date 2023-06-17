use crate::nes::cartridge::{
    mappers::{CartridgePpuLocation, Mapper},
    MirrorType, NesHeader,
};

use super::NRom;

#[test]
fn test_mapping() {
    let mut nes_header = NesHeader::nul_header();
    nes_header.mirror_type = MirrorType::Vertical;
    let mut m = NRom::new(&nes_header);

    assert_eq!(
        CartridgePpuLocation::VRam(0x0000),
        m.translate_ppu_addr(0x2000)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0399),
        m.translate_ppu_addr(0x2399)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0400),
        m.translate_ppu_addr(0x2400)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0799),
        m.translate_ppu_addr(0x2799)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x800),
        m.translate_ppu_addr(0x2800)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0B99),
        m.translate_ppu_addr(0x2B99)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0C00),
        m.translate_ppu_addr(0x2C00)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x1E99),
        m.translate_ppu_addr(0x3E99)
    );
}
