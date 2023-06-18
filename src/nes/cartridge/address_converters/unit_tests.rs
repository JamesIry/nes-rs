use crate::nes::cartridge::{
    address_converters::{AddressConverter, MirroredConverter},
    MirrorType,
};

#[test]
fn test_vertical_mirroring() {
    let c = MirroredConverter::new(MirrorType::Vertical, 0x2000, 0x3EFF, 0x1000, 0x1000, 0x2000);

    assert_eq!(0x0000, c.convert(0x2000));

    assert_eq!(0x0399, c.convert(0x2399));

    assert_eq!(0x0400, c.convert(0x2400));

    assert_eq!(0x0799, c.convert(0x2799));

    assert_eq!(0x0000, c.convert(0x2800));

    assert_eq!(0x0399, c.convert(0x2B99));

    assert_eq!(0x0400, c.convert(0x2C00));

    assert_eq!(0x0799, c.convert(0x2F99));
}

#[test]
fn test_horizontal_mirroring() {
    let c = MirroredConverter::new(MirrorType::Vertical, 0x2000, 0x3EFF, 0x1000, 0x1000, 0x2000);

    assert_eq!(0x0000, c.convert(0x2000));

    assert_eq!(0x0399, c.convert(0x2399));

    assert_eq!(0x0000, c.convert(0x2400));

    assert_eq!(0x0399, c.convert(0x2799));

    assert_eq!(0x0400, c.convert(0x2800));

    assert_eq!(0x0799, c.convert(0x2B99));

    assert_eq!(0x0400, c.convert(0x2C00));

    assert_eq!(0x0799, c.convert(0x2F99));
}
