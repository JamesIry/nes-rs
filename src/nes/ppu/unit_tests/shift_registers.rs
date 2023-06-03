use crate::nes::ppu::{self, BGShiftRegister, BGShiftRegisterPair};

#[test]
fn test_bg_shift_register() {
    let (mut _ppu, _mem) = ppu::create_test_configuration();

    let mut reg = BGShiftRegister::new();

    assert_eq!(0, reg.data);
    assert_eq!(0, reg.current_byte());
    for i in 0..16 {
        assert_eq!(0, reg.bit(i));
    }

    reg.load(0x12);
    assert_eq!(0x0012, reg.data);
    assert_eq!(0, reg.current_byte());
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
    assert_eq!(0x12, reg.current_byte());
    assert_eq!(0, reg.bit(0));
    assert_eq!(0, reg.bit(1));
    assert_eq!(0, reg.bit(2));
    assert_eq!(1, reg.bit(3));
    assert_eq!(0, reg.bit(4));
    assert_eq!(0, reg.bit(5));
    assert_eq!(1, reg.bit(6));
    assert_eq!(0, reg.bit(7));

    reg.load(0x34);
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
    let (mut _ppu, _mem) = ppu::create_test_configuration();

    let mut reg = BGShiftRegisterPair::new();
    assert_eq!(0, reg.high.data);
    assert_eq!(0, reg.low.data);
    for i in 0..16 {
        assert_eq!(0, reg.bits(i));
    }

    reg.load_low(0x12);
    assert_eq!(0x0012, reg.low.data);
    assert_eq!(0, reg.high.data);
    reg.load_high(0x56);
    assert_eq!(0x0012, reg.low.data);
    assert_eq!(0x0056, reg.high.data);

    reg.shift();
    assert_eq!(0x1212, reg.low.data);
    assert_eq!(0x5656, reg.high.data);

    reg.load_low(0x34);
    reg.load_high(0x78);
    assert_eq!(0x1234, reg.low.data);
    assert_eq!(0x5678, reg.high.data);
    //01010110 01111000
    //00010010 00110100
    assert_eq!(0b00, reg.bits(0));
    assert_eq!(0b10, reg.bits(1));
    assert_eq!(0b00, reg.bits(2));
    assert_eq!(0b11, reg.bits(3));
    assert_eq!(0b00, reg.bits(4));
    assert_eq!(0b10, reg.bits(5));
    assert_eq!(0b11, reg.bits(6));
    assert_eq!(0b00, reg.bits(7));

    assert_eq!(0b00, reg.bits(8));
    assert_eq!(0b10, reg.bits(9));
    assert_eq!(0b11, reg.bits(10));
    assert_eq!(0b11, reg.bits(11));
    assert_eq!(0b10, reg.bits(12));
    assert_eq!(0b01, reg.bits(13));
    assert_eq!(0b00, reg.bits(14));
    assert_eq!(0b00, reg.bits(15));
}
