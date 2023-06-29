use crate::nes::cartridge::{
    memory_region::{MemoryRegion, MemoryType},
    MirrorType,
};

#[test]
fn test_vertical_mirroring() {
    let vram_vec = vec![0; 0x1000];
    let mut vram = MemoryRegion::new(MemoryType::VRAM, vram_vec, 0x2000, 0x3FFF, false);
    vram.set_bank_size_k(1);
    vram.set_mirror_type(MirrorType::Vertical);

    assert_eq!(0x0000, vram.convert(0x2000));
    assert_eq!(0x0399, vram.convert(0x2399));
    assert_eq!(0x0400, vram.convert(0x2400));
    assert_eq!(0x0799, vram.convert(0x2799));
    assert_eq!(0x0000, vram.convert(0x2800));
    assert_eq!(0x0399, vram.convert(0x2B99));
    assert_eq!(0x0400, vram.convert(0x2C00));
    assert_eq!(0x0799, vram.convert(0x2F99));
    for i in 0..=0x0EFF {
        assert_eq!(vram.convert(0x2000 + i), vram.convert(0x3000 + i));
    }
}

#[test]
fn test_horizontal_mirroring() {
    let vram_vec = vec![0; 0x1000];
    let mut vram = MemoryRegion::new(MemoryType::VRAM, vram_vec, 0x2000, 0x3FFF, false);
    vram.set_bank_size_k(1);
    vram.set_mirror_type(MirrorType::Horizontal);

    assert_eq!(0x0000, vram.convert(0x2000));
    assert_eq!(0x0399, vram.convert(0x2399));
    assert_eq!(0x0000, vram.convert(0x2400));
    assert_eq!(0x0399, vram.convert(0x2799));
    assert_eq!(0x0400, vram.convert(0x2800));
    assert_eq!(0x0799, vram.convert(0x2B99));
    assert_eq!(0x0400, vram.convert(0x2C00));
    assert_eq!(0x0799, vram.convert(0x2F99));

    for i in 0..=0x0EFF {
        assert_eq!(vram.convert(0x2000 + i), vram.convert(0x3000 + i));
    }
}

#[test]
fn test_with_bank_set() {
    let vec = vec![0; 0x4000];
    let mut ram = MemoryRegion::new(MemoryType::SRAM, vec, 0x3000, 0x4FFF, false);
    ram.set_bank_size_k(2);

    assert_eq!(0x0000, ram.convert(0x3000));
    assert_eq!(0x07FF, ram.convert(0x37FF));
    assert_eq!(0x0000, ram.convert(0x3800));
    assert_eq!(0x07FF, ram.convert(0x3FFF));

    ram.set_bank(0, 1);

    assert_eq!(0x0800, ram.convert(0x3000));
    assert_eq!(0x0FFF, ram.convert(0x37FF));
    assert_eq!(0x0000, ram.convert(0x3800));
    assert_eq!(0x07FF, ram.convert(0x3FFF));

    ram.set_bank(1, 2);

    assert_eq!(0x0800, ram.convert(0x3000));
    assert_eq!(0x0FFF, ram.convert(0x37FF));
    assert_eq!(0x1000, ram.convert(0x3800));
    assert_eq!(0x17FF, ram.convert(0x3FFF));

    ram.set_bank(1, -2);

    assert_eq!(0x0800, ram.convert(0x3000));
    assert_eq!(0x0FFF, ram.convert(0x37FF));
    assert_eq!(0x3000, ram.convert(0x3800));
    assert_eq!(0x37FF, ram.convert(0x3FFF));
}
