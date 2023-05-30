use crate::cartridge::{
    mappers::{CartridgePpuLocation, Mapper},
    MirrorType,
};

use super::Mapper0;

#[test]
fn test_vertical_mapping() {
    let mut m = Mapper0::new(MirrorType::Vertical);

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
        CartridgePpuLocation::VRam(0x0000),
        m.translate_ppu_addr(0x2800)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0399),
        m.translate_ppu_addr(0x2B99)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0400),
        m.translate_ppu_addr(0x2C00)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0799),
        m.translate_ppu_addr(0x3F99)
    );
}

#[test]
fn test_horizontal_mapping() {
    let mut m = Mapper0::new(MirrorType::Horizontal);

    assert_eq!(
        CartridgePpuLocation::VRam(0x0000),
        m.translate_ppu_addr(0x2000)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0399),
        m.translate_ppu_addr(0x2399)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0000),
        m.translate_ppu_addr(0x2400)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0399),
        m.translate_ppu_addr(0x2799)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0400),
        m.translate_ppu_addr(0x2800)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0799),
        m.translate_ppu_addr(0x2B99)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0400),
        m.translate_ppu_addr(0x2C00)
    );

    assert_eq!(
        CartridgePpuLocation::VRam(0x0799),
        m.translate_ppu_addr(0x3F99)
    );
}
