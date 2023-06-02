use crate::bus::BusDevice;
use crate::ppu::flags::{CtrlFlag, MaskFlag, StatusFlag};

#[test]
fn test_ctrl_register() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    assert_eq!(0, ppu.get_ctrl_flags());

    ppu.write(
        0x2000,
        CtrlFlag::IncrementAcross | CtrlFlag::SpriteSizeLarge,
    );

    assert_eq!(0, ppu.temporary_vram_address.register);

    assert_eq!(
        CtrlFlag::IncrementAcross | CtrlFlag::SpriteSizeLarge,
        ppu.get_ctrl_flags()
    );

    ppu.write(0x2000, 0 | CtrlFlag::BaseNameTableHigh);

    assert_eq!(0b0000100000000000, ppu.temporary_vram_address.register);

    ppu.write(0x2000, 0 | CtrlFlag::BaseNameTableLow);

    assert_eq!(0b0000010000000000, ppu.temporary_vram_address.register);
    assert_eq!(0, ppu.vram_address.register);
}

#[test]
fn test_mask_register() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    assert_eq!(0, ppu.mask_register);

    ppu.write(0x2001, MaskFlag::EmphasizeBlue | MaskFlag::ShowBG);

    assert_eq!(
        MaskFlag::EmphasizeBlue | MaskFlag::ShowBG,
        ppu.mask_register
    );
    assert_eq!(0, ppu.vram_address.register);
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

    assert_eq!(0, ppu.vram_address.register);
}

#[test]
fn test_read_from_write() {
    // read from write only should return last read
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    ppu.data_buffer = 0x42;

    assert_eq!(Some(0x42), ppu.read(0x2000));
    assert_eq!(Some(0x42), ppu.read(0x2001));
    assert_eq!(Some(0x42), ppu.read(0x2003));
    assert_eq!(Some(0x42), ppu.read(0x2005));
    assert_eq!(Some(0x42), ppu.read(0x2006));
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

    assert_eq!(0, ppu.vram_address.register);
}

#[test]
fn test_scroll_registers() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    assert_eq!(0, ppu.temporary_vram_address.get_x());
    assert_eq!(0, ppu.temporary_vram_address.get_y());

    ppu.write(0x2005, 0x42);
    assert_eq!(0b0000000000001000, ppu.temporary_vram_address.register);
    assert_eq!(0b00000010, ppu.temporary_vram_address.fine_x);
    ppu.write(0x2005, 0x34);
    assert_eq!(0b0100000011001000, ppu.temporary_vram_address.register);

    assert_eq!(0x42, ppu.temporary_vram_address.get_x());
    assert_eq!(0x34, ppu.temporary_vram_address.get_y());

    // should reset the latch
    ppu.read(0x2002);

    // allowing new writes
    ppu.write(0x2005, 0x45);
    ppu.write(0x2005, 0x67);

    assert_eq!(0x45, ppu.temporary_vram_address.get_x());
    assert_eq!(0x67, ppu.temporary_vram_address.get_y());

    assert_eq!(0, ppu.vram_address.register);
}

#[test]
fn test_data_registers_small_stride() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    ppu.set_ctrl_flag(CtrlFlag::IncrementAcross, false);

    assert_eq!(0, ppu.bus.read(0x1234));
    assert_eq!(0, ppu.bus.read(0x1235));

    ppu.write(0x2006, 0x12);

    assert_eq!(0x1200, ppu.temporary_vram_address.register);
    assert_eq!(0, ppu.vram_address.register);

    ppu.write(0x2006, 0x34);
    assert_eq!(0x1234, ppu.temporary_vram_address.register);
    assert_eq!(0x1234, ppu.vram_address.register);

    ppu.write(0x2007, 0x42);
    assert!(!ppu.clock());
    assert_eq!(0x42, ppu.bus.read(0x1234));
    ppu.write(0x2007, 0x43);
    assert!(!ppu.clock());
    assert_eq!(0x43, ppu.bus.read(0x1235));

    ppu.write(0x2006, 0x12);
    ppu.write(0x2006, 0x34);

    // after writing the address, the first read is bogus
    ppu.read(0x2007);

    // but once clock has ticked the reads will be good
    assert!(!ppu.clock());
    assert_eq!(Some(0x42), ppu.read(0x2007));
    assert!(!ppu.clock());
    assert_eq!(Some(0x43), ppu.read(0x2007));
}

#[test]
fn test_data_registers_large_stride() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    ppu.set_ctrl_flag(CtrlFlag::IncrementAcross, true);

    assert_eq!(0, ppu.bus.read(0x1234));
    assert_eq!(0, ppu.bus.read(0x1254));

    ppu.write(0x2006, 0x12);
    ppu.write(0x2006, 0x34);

    ppu.write(0x2007, 0x42);
    assert!(!ppu.clock());
    assert_eq!(0x42, ppu.bus.read(0x1234));
    ppu.write(0x2007, 0x43);
    assert!(!ppu.clock());
    assert_eq!(0x43, ppu.bus.read(0x1254));

    ppu.write(0x2006, 0x12);
    ppu.write(0x2006, 0x34);

    // after writing the address, the first read is bogus
    ppu.read(0x2007);

    // but once clock has ticked the reads will be good
    assert!(!ppu.clock());
    assert_eq!(Some(0x42), ppu.read(0x2007));
    assert!(!ppu.clock());
    assert_eq!(Some(0x43), ppu.read(0x2007));
}

#[test]
fn test_automatic_status() {
    let (mut ppu, _mem) = crate::ppu::create_test_configuration();

    assert_eq!(-1, ppu.scan_line);
    assert_eq!(0, ppu.tick);

    ppu.set_ctrl_flag(CtrlFlag::NmiEnabled, true);

    ppu.status_register =
        StatusFlag::VerticalBlank | StatusFlag::SpriteOverflow | StatusFlag::Sprite0Hit;
    assert!(!ppu.clock());
    assert_eq!(
        StatusFlag::VerticalBlank | StatusFlag::SpriteOverflow | StatusFlag::Sprite0Hit,
        ppu.status_register
    );
    assert!(!ppu.clock());
    assert_eq!(0, ppu.status_register);

    ppu.scan_line = 241;
    ppu.tick = 0;
    assert!(!ppu.clock());
    assert_eq!(0, ppu.status_register);
    assert!(ppu.clock()); // nmi should happen here
    assert!(ppu.read_status_flag(StatusFlag::VerticalBlank));
    assert_eq!(Some(StatusFlag::VerticalBlank | 0), ppu.read(0x2002)); // should clear VB flag
    assert_eq!(Some(0), ppu.read(0x2002));

    ppu.set_ctrl_flag(CtrlFlag::NmiEnabled, false);
    ppu.status_register = 0;
    ppu.scan_line = 241;
    ppu.tick = 0;
    assert!(!ppu.clock());
    assert_eq!(0, ppu.status_register);
    assert!(!ppu.clock()); // nmi should not happen here because disabled
    assert_eq!(StatusFlag::VerticalBlank | 0, ppu.status_register);
}
