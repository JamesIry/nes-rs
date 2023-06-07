use crate::nes::ppu::VramAddress;

#[test]
fn test_vram_address_addr() {
    let mut reg = VramAddress::new();

    assert_eq!(0, reg.get_address_high());
    assert_eq!(0, reg.get_address_low());
    assert_eq!(0, reg.register);

    reg.set_address_high(0x12);
    reg.set_address_low(0x34);

    assert_eq!(0x12, reg.get_address_high());
    assert_eq!(0x34, reg.get_address_low());
    assert_eq!(0x00001234, reg.register);

    reg.inc_address(0x01);
    assert_eq!(0x00001235, reg.register);
}

#[test]
fn test_vram_address_x_y() {
    let mut reg = VramAddress::new();

    assert_eq!(0, reg.get_coarse_x());
    assert_eq!(0, reg.get_coarse_y());
    assert_eq!(0, reg.get_fine_y());

    reg.set_x(0x37);
    reg.set_y(0x25);

    assert_eq!(0b00110111, reg.get_x());
    assert_eq!(0b00100101, reg.get_y());
    assert_eq!(0b101, reg.get_fine_y());
    assert_eq!(0b00110, reg.get_coarse_x());
    assert_eq!(0b00100, reg.get_coarse_y());
    assert_eq!(0b101000010000110, reg.register);

    reg.set_coarse_x(30);
    reg.increment_coarse_x();
    assert_eq!(31, reg.get_coarse_x());
    reg.increment_coarse_x();
    assert_eq!(0, reg.get_coarse_x());
    assert_eq!(0x25, reg.get_y());
    assert_eq!(0b101, reg.get_fine_y());

    reg.set_y(238);
    reg.increment_y();
    assert_eq!(239, reg.get_y());
    reg.increment_y();
    assert_eq!(0, reg.get_y());
    assert_eq!(0, reg.get_coarse_x());

    reg.set_y(254);
    reg.increment_y();
    assert_eq!(255, reg.get_y());
    reg.increment_y();
    assert_eq!(0, reg.get_y());

    let mut reg2 = VramAddress::new();
    reg.set_x(0x12);
    reg.set_y(0x34);
    reg.set_horizontal_nametable_selected(true);
    reg2.copy_x_from(&reg);
    assert_eq!(0x12, reg2.get_x());
    assert_eq!(0, reg2.get_y());
    assert_eq!(0b01, reg2.get_nametable_bits());

    reg2.set_vertical_nametable_selected(true);
    reg2.copy_y_from(&reg);
    assert_eq!(0x12, reg2.get_x());
    assert_eq!(0b01, reg2.get_nametable_bits());
    assert_eq!(0x12, reg2.get_x());
    assert_eq!(0x34, reg2.get_y());
    assert_eq!(0b01, reg2.get_nametable_bits());
}

#[test]
fn test_vram_address_nametable() {
    let mut reg = VramAddress::new();

    assert_eq!(0, reg.get_nametable_bits());
    assert_eq!(0x2000, reg.get_nametable_address());
    assert!(!reg.get_horizontal_nametable_selected());
    assert!(!reg.get_vertical_nametable_selected());

    reg.set_x(0x37);
    reg.set_y(0x25);

    reg.set_horizontal_nametable_selected(true);
    assert!(reg.get_horizontal_nametable_selected());
    assert_eq!(0b01, reg.get_nametable_bits());
    assert_eq!(0b0101010010000110, reg.register);

    reg.set_vertical_nametable_selected(true);
    assert!(reg.get_vertical_nametable_selected());
    assert_eq!(0b11, reg.get_nametable_bits());
    assert_eq!(0b0101110010000110, reg.register);

    assert_eq!(0b0010110010000110, reg.get_nametable_address());
}

#[test]
fn test_vram_address_attribute_table() {
    let mut reg = VramAddress::new();

    assert_eq!(0x23C0, reg.get_attribute_address());

    reg.set_x(0x37);
    reg.set_y(0x25);

    reg.set_vertical_nametable_selected(true);

    assert_eq!(0b0101100010000110, reg.register);
    assert_eq!(0b0010101111001001, reg.get_attribute_address());
}

#[test]
fn test_attribute_shift() {
    let mut addr = VramAddress::new();

    for y in 0..16 {
        addr.set_y(y);
        for x in 0..16 {
            addr.set_x(x);
            assert_eq!(0, addr.get_attribute_shift());
        }
    }

    for y in 0..16 {
        addr.set_y(y);
        for x in 16..32 {
            addr.set_x(x);
            assert_eq!(2, addr.get_attribute_shift());
        }
    }

    for y in 16..32 {
        addr.set_y(y);
        for x in 0..16 {
            addr.set_x(x);
            assert_eq!(4, addr.get_attribute_shift());
        }
    }

    for y in 16..32 {
        addr.set_y(y);
        for x in 16..32 {
            addr.set_x(x);
            assert_eq!(6, addr.get_attribute_shift());
        }
    }
}
