use crate::nes::ppu::{BGShiftRegister, BGShiftRegisterPair, BGShiftRegisterSet};

#[test]
fn test_bg_shift_register() {
    let mut reg = BGShiftRegister::new();

    assert_eq!(0, reg.data);
    assert_eq!(0, reg.prefetch);
    for i in 0..16 {
        assert_eq!(0, reg.bit(i));
    }

    reg.load(0x12);
    assert_eq!(0, reg.data);

    reg.shift();
    assert_eq!(0x0012, reg.data);
    for i in 0..8 {
        assert_eq!(0, reg.bit(i));
    }
    assert_eq!(0, reg.bit(8));
    assert_eq!(0, reg.bit(9));
    assert_eq!(0, reg.bit(10));
    assert_eq!(1, reg.bit(11));
    assert_eq!(0, reg.bit(12));
    assert_eq!(0, reg.bit(13));
    assert_eq!(1, reg.bit(14));
    assert_eq!(0, reg.bit(15));

    reg.shift();
    assert_eq!(0x1212, reg.data);
    assert_eq!(0, reg.bit(0));
    assert_eq!(0, reg.bit(1));
    assert_eq!(0, reg.bit(2));
    assert_eq!(1, reg.bit(3));
    assert_eq!(0, reg.bit(4));
    assert_eq!(0, reg.bit(5));
    assert_eq!(1, reg.bit(6));
    assert_eq!(0, reg.bit(7));

    reg.load(0x34);
    reg.shift();
    assert_eq!(0x1234, reg.data);
    assert_eq!(0, reg.bit(8));
    assert_eq!(0, reg.bit(9));
    assert_eq!(1, reg.bit(10));
    assert_eq!(1, reg.bit(11));
    assert_eq!(0, reg.bit(12));
    assert_eq!(1, reg.bit(13));
    assert_eq!(0, reg.bit(14));
    assert_eq!(0, reg.bit(15));
}

#[test]
fn test_bg_shift_register_pair() {
    let mut reg = BGShiftRegisterPair::new();

    assert_eq!(0, reg.high.prefetch);
    assert_eq!(0, reg.high.data);
    assert_eq!(0, reg.low.prefetch);
    assert_eq!(0, reg.low.data);
    assert_eq!(0, reg.bits(0, 0));

    reg.load_high(0x12);
    assert_eq!(0x12, reg.high.prefetch);
    assert_eq!(0, reg.high.data);
    assert_eq!(0, reg.low.prefetch);
    assert_eq!(0, reg.low.data);

    reg.load_low(0x56);
    assert_eq!(0x12, reg.high.prefetch);
    assert_eq!(0, reg.high.data);
    assert_eq!(0x56, reg.low.prefetch);
    assert_eq!(0, reg.low.data);
    assert_eq!(0, reg.bits(0, 0));

    reg.shift();
    assert_eq!(0x12, reg.high.prefetch);
    assert_eq!(0x12, reg.high.data);
    assert_eq!(0x56, reg.low.prefetch);
    assert_eq!(0x56, reg.low.data);
    assert_eq!(0, reg.bits(0, 0));

    reg.load_high(0x34);
    reg.load_low(0x78);
    reg.shift();

    assert_eq!(0x34, reg.high.prefetch);
    assert_eq!(0x1234, reg.high.data);
    assert_eq!(0x78, reg.low.prefetch);
    assert_eq!(0x5678, reg.low.data);
    //0001 0010 0011 0100
    //0101 0110 0111 1000
    assert_eq!(0b00, reg.bits(0, 0));
    assert_eq!(0b01, reg.bits(0, 1));
    assert_eq!(0b00, reg.bits(0, 2));
    assert_eq!(0b11, reg.bits(0, 3));

    assert_eq!(0b00, reg.bits(0, 4));
    assert_eq!(0b01, reg.bits(0, 5));
    assert_eq!(0b11, reg.bits(0, 6));
    assert_eq!(0b00, reg.bits(0, 7));

    assert_eq!(0b00, reg.bits(1, 7));
    assert_eq!(0b01, reg.bits(2, 7));
    assert_eq!(0b11, reg.bits(3, 7));
    assert_eq!(0b11, reg.bits(4, 7));

    assert_eq!(0b01, reg.bits(5, 7));
    assert_eq!(0b10, reg.bits(6, 7));
    assert_eq!(0b00, reg.bits(7, 7));

    assert_eq!(0b00, reg.bits(8, 7));
    assert_eq!(0b00, reg.bits(9, 7));
    assert_eq!(0b01, reg.bits(10, 7));
    assert_eq!(0b11, reg.bits(11, 7));
    assert_eq!(0b11, reg.bits(12, 7));
}

#[test]
fn test_bg_shift_register_set_basics() {
    let mut set = BGShiftRegisterSet::new();

    assert_eq!(0, set.attribute_data.low.data);
    assert_eq!(0, set.attribute_data.high.data);
    assert_eq!(0, set.name_table_data);
    assert_eq!(0, set.pattern_data.low.data);
    assert_eq!(0, set.pattern_data.high.data);

    set.load_attribute_data(0b00011011, 2);
    set.load_name_table_data(0x34);
    set.load_pattern_data_low(0x56);
    set.load_pattern_data_high(0x78);
    set.shift();

    assert_eq!(0x00FF, set.attribute_data.high.data);
    assert_eq!(0x0000, set.attribute_data.low.data);
    assert_eq!(0x34, set.name_table_data);
    assert_eq!(0x0056, set.pattern_data.low.data);
    assert_eq!(0x0078, set.pattern_data.high.data);

    set.shift();

    assert_eq!(0xFFFF, set.attribute_data.high.data);
    assert_eq!(0x0000, set.attribute_data.low.data);
    assert_eq!(0x34, set.name_table_data);
    assert_eq!(0x5656, set.pattern_data.low.data);
    assert_eq!(0x7878, set.pattern_data.high.data);

    set.load_attribute_data(0b00011011, 4);
    set.load_name_table_data(0xBC);
    set.load_pattern_data_low(0xDE);
    set.load_pattern_data_high(0xF0);
    set.shift();

    assert_eq!(0xFF00, set.attribute_data.high.data);
    assert_eq!(0x00FF, set.attribute_data.low.data);
    assert_eq!(0xBC, set.name_table_data);
    assert_eq!(0x56DE, set.pattern_data.low.data);
    assert_eq!(0x78F0, set.pattern_data.high.data);
}

#[test]
fn test_bg_shift_register_set_pattern_address() {
    let mut set = BGShiftRegisterSet::new();

    set.load_name_table_data(0x12);
    assert_eq!(0x0127, set.get_pattern_address(false, 7));
    assert_eq!(0x1125, set.get_pattern_address(true, 5));

    set.load_name_table_data(0xCA);
    assert_eq!(0x0CA7, set.get_pattern_address(false, 0xF));
    assert_eq!(0x1CA4, set.get_pattern_address(true, 0xC));
}

#[test]
fn test_bg_shift_register_set_pixel_color() {
    let mut set = BGShiftRegisterSet::new();

    set.load_pattern_data_high(0b00001111);
    set.load_pattern_data_low(0b01010101);
    set.shift();
    set.load_pattern_data_high(0b11110000);
    set.load_pattern_data_low(0b10101010);
    set.shift();

    assert_eq!(0b00, set.get_pixel_color_number(0, 0));
    assert_eq!(0b01, set.get_pixel_color_number(0, 1));
    assert_eq!(0b00, set.get_pixel_color_number(0, 2));
    assert_eq!(0b01, set.get_pixel_color_number(0, 3));
    assert_eq!(0b10, set.get_pixel_color_number(0, 4));
    assert_eq!(0b11, set.get_pixel_color_number(0, 5));
    assert_eq!(0b10, set.get_pixel_color_number(0, 6));
    assert_eq!(0b11, set.get_pixel_color_number(0, 7));

    assert_eq!(0b11, set.get_pixel_color_number(1, 7));
    assert_eq!(0b10, set.get_pixel_color_number(2, 7));
    assert_eq!(0b11, set.get_pixel_color_number(3, 7));
    assert_eq!(0b10, set.get_pixel_color_number(4, 7));
    assert_eq!(0b01, set.get_pixel_color_number(5, 7));
    assert_eq!(0b00, set.get_pixel_color_number(6, 7));
    assert_eq!(0b01, set.get_pixel_color_number(7, 7));

    assert_eq!(0b11, set.get_pixel_color_number(8, 7));
    assert_eq!(0b11, set.get_pixel_color_number(9, 7));
    assert_eq!(0b10, set.get_pixel_color_number(10, 7));
    assert_eq!(0b11, set.get_pixel_color_number(11, 7));
    assert_eq!(0b10, set.get_pixel_color_number(12, 7));
    assert_eq!(0b01, set.get_pixel_color_number(13, 7));
    assert_eq!(0b00, set.get_pixel_color_number(14, 7));
    assert_eq!(0b01, set.get_pixel_color_number(15, 7));
}

#[test]
fn test_bg_shift_register_get_palette_address() {
    let mut set = BGShiftRegisterSet::new();

    set.load_pattern_data_high(0b00001111);
    set.load_pattern_data_low(0b01010101);
    set.load_attribute_data(0b00011011, 4); // 01
    set.shift();
    set.load_pattern_data_high(0b11110000);
    set.load_pattern_data_low(0b10101010);
    set.load_attribute_data(0b00010110, 0); // 10
    set.shift();

    assert_eq!(0b0011111100000000, set.get_palette_address(false, 0, 0));
    assert_eq!(0b0011111100010000, set.get_palette_address(true, 0, 0));

    assert_eq!(0b0011111100000101, set.get_palette_address(false, 0, 1));
    assert_eq!(0b0011111100000111, set.get_palette_address(false, 0, 5));
    assert_eq!(0b0011111100000111, set.get_palette_address(false, 5, 0));
    assert_eq!(0b0011111100000111, set.get_palette_address(false, 21, 0));
}
