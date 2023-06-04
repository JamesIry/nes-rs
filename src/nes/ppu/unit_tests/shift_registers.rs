use crate::nes::ppu::{self, BGShiftRegister};

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
