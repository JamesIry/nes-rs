use crate::bus::BusDevice;
use crate::ppu::flags::{CtrlFlag, MaskFlag, StatusFlag};
use crate::ppu::PPU;

#[test]
fn test_ctrl_register() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    assert_eq!(0, ppu.ppu_ctrl);

    ppu.write(
        0x2000,
        CtrlFlag::IncrementAcross | CtrlFlag::SpriteSizeLarge,
    );

    assert_eq!(
        CtrlFlag::IncrementAcross | CtrlFlag::SpriteSizeLarge,
        ppu.ppu_ctrl
    );
}

#[test]
fn test_mask_register() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    assert_eq!(0, ppu.ppu_mask);

    ppu.write(0x2001, MaskFlag::EmphasizeBlue | MaskFlag::ShowBG);

    assert_eq!(MaskFlag::EmphasizeBlue | MaskFlag::ShowBG, ppu.ppu_mask);
}

#[test]
fn test_status_regiter() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    assert_eq!(Some(0), ppu.read(0x2002));

    ppu.set_status_flag(StatusFlag::VerticalBlank, true);
    ppu.set_status_flag(StatusFlag::Sprite0Hit, true);

    assert_eq!(
        Some(StatusFlag::VerticalBlank | StatusFlag::Sprite0Hit),
        ppu.read(0x2002)
    );

    // first ready should have cleared the vertical blank flag
    assert_eq!(Some(0 | StatusFlag::Sprite0Hit), ppu.read(0x2002));

    check_read_from_write(&mut ppu, 0 | StatusFlag::Sprite0Hit);
}

#[cfg(test)]
fn check_read_from_write(ppu: &mut PPU, expected: u8) {
    // read from write only should return last read

    assert_eq!(Some(expected), ppu.read(0x2000));
    assert_eq!(Some(expected), ppu.read(0x2001));
    assert_eq!(Some(expected), ppu.read(0x2003));
    assert_eq!(Some(expected), ppu.read(0x2005));
    assert_eq!(Some(expected), ppu.read(0x2006));
}

#[test]
fn test_oam_registers() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    assert_eq!(ppu.oam_table, [0; 256]);

    ppu.write(0x2003, 0x42);
    ppu.write(0x2004, 0x12);
    ppu.write(0x2004, 0x34);

    assert_eq!(0x00, ppu.oam_table[0x41]);
    assert_eq!(0x12, ppu.oam_table[0x42]);
    assert_eq!(0x34, ppu.oam_table[0x43]);
    assert_eq!(0x00, ppu.oam_table[0x44]);

    ppu.write(0x2003, 0x42);
    assert_eq!(Some(0x12), ppu.read(0x2004));
    assert_eq!(Some(0x12), ppu.read(0x2004));
    ppu.write(0x2003, 0x43);
    assert_eq!(Some(0x34), ppu.read(0x2004));

    check_read_from_write(&mut ppu, 0x34);
}

#[test]
fn test_scroll_registers() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    assert_eq!(0, ppu.ppu_scroll_x);
    assert_eq!(0, ppu.ppu_scroll_y);

    ppu.write(0x2005, 0x12);
    ppu.write(0x2005, 0x34);

    assert_eq!(0x12, ppu.ppu_scroll_x);
    assert_eq!(0x34, ppu.ppu_scroll_y);

    // should reset the latch
    ppu.read(0x2002);

    // allowing new writes
    ppu.write(0x2005, 0x45);
    ppu.write(0x2005, 0x67);

    assert_eq!(0x45, ppu.ppu_scroll_x);
    assert_eq!(0x67, ppu.ppu_scroll_y);
}

#[test]
fn test_data_registers_small_stride() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    ppu.set_ctrl_flag(CtrlFlag::IncrementAcross, false);

    assert_eq!(0, ppu.read_ppu_bus(0x1234));
    assert_eq!(0, ppu.read_ppu_bus(0x1235));

    ppu.write(0x2006, 0x12);
    ppu.write(0x2006, 0x34);

    ppu.write(0x2007, 0x42);
    assert_eq!(0, ppu.read_ppu_bus(0x1234));
    assert!(!ppu.clock());
    assert_eq!(0x42, ppu.read_ppu_bus(0x1234));
    ppu.write(0x2007, 0x43);
    assert_eq!(0, ppu.read_ppu_bus(0x1235));
    assert!(!ppu.clock());
    assert_eq!(0x43, ppu.read_ppu_bus(0x1235));

    ppu.write(0x2006, 0x12);
    ppu.write(0x2006, 0x34);

    // after writing the address, the first read is bogus
    assert_eq!(Some(0x43), ppu.read(0x2007));

    // but once clock has ticked the reads will be good
    assert!(!ppu.clock());
    assert_eq!(Some(0x42), ppu.read(0x2007));
    assert!(!ppu.clock());
    assert_eq!(Some(0x43), ppu.read(0x2007));

    check_read_from_write(&mut ppu, 0x43);
}

#[test]
fn test_data_registers_large_stride() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    ppu.set_ctrl_flag(CtrlFlag::IncrementAcross, true);

    assert_eq!(0, ppu.read_ppu_bus(0x1234));
    assert_eq!(0, ppu.read_ppu_bus(0x1254));

    ppu.write(0x2006, 0x12);
    ppu.write(0x2006, 0x34);

    ppu.write(0x2007, 0x42);
    assert_eq!(0, ppu.read_ppu_bus(0x1234));
    assert!(!ppu.clock());
    assert_eq!(0x42, ppu.read_ppu_bus(0x1234));
    ppu.write(0x2007, 0x43);
    assert_eq!(0, ppu.read_ppu_bus(0x1254));
    assert!(!ppu.clock());
    assert_eq!(0x43, ppu.read_ppu_bus(0x1254));

    ppu.write(0x2006, 0x12);
    ppu.write(0x2006, 0x34);

    // after writing the address, the first read is bogus
    assert_eq!(Some(0x43), ppu.read(0x2007));

    // but once clock has ticked the reads will be good
    assert!(!ppu.clock());
    assert_eq!(Some(0x42), ppu.read(0x2007));
    assert!(!ppu.clock());
    assert_eq!(Some(0x43), ppu.read(0x2007));

    check_read_from_write(&mut ppu, 0x43);
}
