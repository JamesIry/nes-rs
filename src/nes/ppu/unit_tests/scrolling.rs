use crate::{
    bus::BusDevice,
    nes::ppu,
    nes::ppu::flags::{CtrlFlags, MaskFlags, StatusFlags},
};

#[test]
fn test_x_scroll() {
    let (mut ppu, _mem) = ppu::create_test_configuration();

    ppu.set_ctrl_flag(CtrlFlags::NmiEnabled, true);
    ppu.set_mask_flag(MaskFlags::ShowBG, true);
    ppu.set_status_flag(StatusFlags::VerticalBlank, true);

    // set vram x, y to arbitrary values
    ppu.vram_address.set_coarse_x(13);
    ppu.vram_address.set_y(239);

    // set scroll_x, scroll_y to 0,0
    ppu.write(0x2004, 0);
    ppu.write(0x2004, 0);

    assert_eq!(0, ppu.vram_address.get_nametable_bits());
    assert_eq!(0, ppu.temporary_vram_address.get_nametable_bits());

    ppu.tick = 256;
    ppu.scan_line = -1;
    assert_eq!((false, false), ppu.clock());

    // x should be incremented. y should be incremented with a wrap
    assert_eq!(14, ppu.vram_address.get_coarse_x());
    assert_eq!(0, ppu.vram_address.get_y());
    assert_eq!(2, ppu.vram_address.get_nametable_bits());

    assert_eq!((false, false), ppu.clock());

    // now force y back to something weird to make sure it gets updated to 0 eventually
    ppu.vram_address.set_y(87);

    // coarse x should be updated, coarse y not
    for _ in 257..280 {
        assert_eq!(0, ppu.vram_address.get_coarse_x());
        assert_eq!(87, ppu.vram_address.get_y());
        assert_eq!(2, ppu.vram_address.get_nametable_bits());
        assert_eq!((false, false), ppu.clock());
    }

    // set temp_x to something new, should have no effect until end of next line
    ppu.temporary_vram_address.set_coarse_x(31);
    ppu.temporary_vram_address
        .set_horizontal_nametable_selected(true);

    // now y should be updated
    for _ in 280..328 {
        assert_eq!(0, ppu.vram_address.get_coarse_x());
        assert_eq!(0, ppu.vram_address.get_y());
        assert_eq!(0, ppu.vram_address.get_nametable_bits());
        assert_eq!((false, false), ppu.clock());
    }

    // set temp_y to something new, should have no effect for a bunch of ticks
    ppu.temporary_vram_address.set_y(12);
    ppu.temporary_vram_address
        .set_vertical_nametable_selected(true);

    // now x should be incremented
    for _ in 328..336 {
        assert_eq!(1, ppu.vram_address.get_coarse_x());
        assert_eq!(0, ppu.vram_address.get_y());
        assert_eq!(0, ppu.vram_address.get_nametable_bits());
        assert_eq!((false, false), ppu.clock());
    }

    // x = 2
    for _ in 336..=340 {
        assert_eq!(2, ppu.vram_address.get_coarse_x());
        assert_eq!(0, ppu.vram_address.get_y());
        assert_eq!(0, ppu.vram_address.get_nametable_bits());
        assert_eq!((false, false), ppu.clock());
    }

    // new scanline, tick 0 does nothing, so x still = 2
    assert_eq!(2, ppu.vram_address.get_coarse_x());
    assert_eq!(0, ppu.vram_address.get_y());
    assert_eq!(0, ppu.vram_address.get_nametable_bits());
    assert_eq!((false, false), ppu.clock());

    // just cruising for the rest of the visible line
    for i in 1..240 {
        assert_eq!((i / 8 + 2) as u8, ppu.vram_address.get_coarse_x(), "{}", i);
        assert_eq!(0, ppu.vram_address.get_y());
        assert_eq!(0, ppu.vram_address.get_nametable_bits());
        assert_eq!((false, false), ppu.clock());
    }

    // more cruising with a wrapped around x
    for i in 240..256 {
        assert_eq!(
            ((i - 240) / 8) as u8,
            ppu.vram_address.get_coarse_x(),
            "{}",
            i
        );
        assert_eq!(0, ppu.vram_address.get_y());
        assert_eq!(1, ppu.vram_address.get_nametable_bits());
        assert_eq!((false, false), ppu.clock());
    }

    // y should now be incremented
    assert_eq!(2, ppu.vram_address.get_coarse_x());
    assert_eq!(1, ppu.vram_address.get_y());
    assert_eq!(1, ppu.vram_address.get_nametable_bits());
    assert_eq!((false, false), ppu.clock());

    // x and nametable should be clobbered but then stay the same for a bunch of ticks
    for _ in 257..328 {
        assert_eq!(31, ppu.vram_address.get_coarse_x());
        assert_eq!(1, ppu.vram_address.get_y());
        assert_eq!(1, ppu.vram_address.get_nametable_bits());
        assert_eq!((false, false), ppu.clock());
    }

    // x inrementing should start again (with a wrap) and we're done with the line
    for i in 328..=340 {
        assert_eq!(((i - 328) as u8) / 8, ppu.vram_address.get_coarse_x());
        assert_eq!(1, ppu.vram_address.get_y());
        assert_eq!(0, ppu.vram_address.get_nametable_bits());
        assert_eq!((false, false), ppu.clock());
    }
}

#[test]
fn test_scroll_post_render() {
    // post render lines shouldn't d any scroll behavior
    let (mut ppu, _mem) = ppu::create_test_configuration();

    ppu.set_ctrl_flag(CtrlFlags::NmiEnabled, true);
    ppu.set_mask_flag(MaskFlags::ShowBG, true);
    ppu.set_status_flag(StatusFlags::VerticalBlank, true);

    // set vram x, y to arbitrary values
    ppu.vram_address.set_coarse_x(30);
    ppu.vram_address.set_y(87);
    ppu.vram_address.set_vertical_nametable_selected(true);

    for scanline in 240..=260 {
        ppu.scan_line = scanline;
        ppu.tick = 0;
        for _ in 0..=340 {
            assert_eq!(30, ppu.vram_address.get_coarse_x());
            assert_eq!(87, ppu.vram_address.get_y());
            assert_eq!(2, ppu.vram_address.get_nametable_bits());
            assert_eq!(
                ppu.clock(),
                (
                    ppu.scan_line == -1 && ppu.tick == 0,
                    false
                ),
                "{}, {}",
                ppu.tick,
                ppu.scan_line
            );
        }
    }
}
