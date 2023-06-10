extern crate bitflags;

bitflags::bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct CtrlFlags: u8 {
        const BaseNameTableLow = 0b00000001;
        const BaseNameTableHigh = 0b00000010;
        const IncrementAcross = 0b00000100;
        const SpriteTableHigh = 0b00001000;
        const BackgroundPatternHigh = 0b00010000;
        const SpriteSizeLarge = 0b00100000;
        const PpuMaster = 0b01000000;
        const NmiEnabled = 0b10000000;

    }
}

bitflags::bitflags! {
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct MaskFlags: u8 {
        const Greyscale = 0b00000001;
        const ShowLeft8BG = 0b00000010;
        const ShowLeft8Sprites = 0b00000100;
        const ShowBG = 0b00001000;
        const ShowSprites = 0b00010000;
        const EmphasizeRed = 0b00100000;
        const EmphasizeGreen = 0b01000000;
        const EmphasizeBlue = 0b10000000;
    }
}

bitflags::bitflags! {
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct StatusFlags: u8 {
        const SpriteOverflow = 0b00100000;
        const Sprite0Hit = 0b01000000;
        const VerticalBlank = 0b10000000;
    }
}
