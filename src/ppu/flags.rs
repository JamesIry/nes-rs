use std::ops::BitAnd;
use std::ops::BitAndAssign;
use std::ops::BitOr;
use std::ops::BitOrAssign;
use std::ops::Not;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum CtrlFlag {
    BaseNameTableLow = 0b00000001,
    BaseNameTableHigh = 0b00000010,
    IncrementAcross = 0b00000100,
    SpriteTableHigh = 0b00001000,
    BackgroundPatternHigh = 0b00010000,
    SpriteSizeLarge = 0b00100000,
    PpuMaster = 0b01000000,
    NmiEnabled = 0b10000000,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MaskFlag {
    Greyscale = 0b00000001,
    Left8BG = 0b00000010,
    Left8Sprites = 0b00000100,
    ShowBG = 0b00001000,
    ShowSprites = 0b00010000,
    EmphasizeRed = 0b00100000,
    EmphasizeGreen = 0b01000000,
    EmphasizeBlue = 0b10000000,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum StatusFlag {
    SpriteOverflow = 0b00100000,
    Sprite0Hit = 0b01000000,
    VerticalBlank = 0b10000000,
}

impl BitOr<Self> for CtrlFlag {
    type Output = u8;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as u8 | rhs as u8
    }
}

impl BitOr<u8> for CtrlFlag {
    type Output = u8;

    fn bitor(self, rhs: u8) -> Self::Output {
        self as u8 | rhs
    }
}

impl BitOr<CtrlFlag> for u8 {
    type Output = u8;

    fn bitor(self, rhs: CtrlFlag) -> Self::Output {
        self | rhs as u8
    }
}

impl BitOrAssign<CtrlFlag> for u8 {
    fn bitor_assign(&mut self, rhs: CtrlFlag) {
        *self |= rhs as u8
    }
}

impl BitAnd<Self> for CtrlFlag {
    type Output = u8;

    fn bitand(self, rhs: Self) -> Self::Output {
        self as u8 & rhs as u8
    }
}

impl BitAnd<u8> for CtrlFlag {
    type Output = u8;

    fn bitand(self, rhs: u8) -> Self::Output {
        self as u8 & rhs
    }
}

impl BitAnd<CtrlFlag> for u8 {
    type Output = u8;

    fn bitand(self, rhs: CtrlFlag) -> Self::Output {
        self & rhs as u8
    }
}

impl BitAndAssign<CtrlFlag> for u8 {
    fn bitand_assign(&mut self, rhs: CtrlFlag) {
        *self &= rhs as u8
    }
}

impl Not for CtrlFlag {
    type Output = u8;

    fn not(self) -> Self::Output {
        !(self as u8)
    }
}

impl BitOr<Self> for MaskFlag {
    type Output = u8;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as u8 | rhs as u8
    }
}

impl BitOr<u8> for MaskFlag {
    type Output = u8;

    fn bitor(self, rhs: u8) -> Self::Output {
        self as u8 | rhs
    }
}

impl BitOr<MaskFlag> for u8 {
    type Output = u8;

    fn bitor(self, rhs: MaskFlag) -> Self::Output {
        self | rhs as u8
    }
}

impl BitOrAssign<MaskFlag> for u8 {
    fn bitor_assign(&mut self, rhs: MaskFlag) {
        *self |= rhs as u8
    }
}

impl BitAnd<Self> for MaskFlag {
    type Output = u8;

    fn bitand(self, rhs: Self) -> Self::Output {
        self as u8 & rhs as u8
    }
}

impl BitAnd<u8> for MaskFlag {
    type Output = u8;

    fn bitand(self, rhs: u8) -> Self::Output {
        self as u8 & rhs
    }
}

impl BitAnd<MaskFlag> for u8 {
    type Output = u8;

    fn bitand(self, rhs: MaskFlag) -> Self::Output {
        self & rhs as u8
    }
}

impl BitAndAssign<MaskFlag> for u8 {
    fn bitand_assign(&mut self, rhs: MaskFlag) {
        *self &= rhs as u8
    }
}

impl Not for MaskFlag {
    type Output = u8;

    fn not(self) -> Self::Output {
        !(self as u8)
    }
}

impl BitOr<Self> for StatusFlag {
    type Output = u8;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as u8 | rhs as u8
    }
}

impl BitOr<u8> for StatusFlag {
    type Output = u8;

    fn bitor(self, rhs: u8) -> Self::Output {
        self as u8 | rhs
    }
}

impl BitOr<StatusFlag> for u8 {
    type Output = u8;

    fn bitor(self, rhs: StatusFlag) -> Self::Output {
        self | rhs as u8
    }
}

impl BitOrAssign<StatusFlag> for u8 {
    fn bitor_assign(&mut self, rhs: StatusFlag) {
        *self |= rhs as u8
    }
}

impl BitAnd<StatusFlag> for u8 {
    type Output = u8;

    fn bitand(self, rhs: StatusFlag) -> Self::Output {
        self & rhs as u8
    }
}

impl BitAndAssign<StatusFlag> for u8 {
    fn bitand_assign(&mut self, rhs: StatusFlag) {
        *self &= rhs as u8
    }
}

impl Not for StatusFlag {
    type Output = u8;

    fn not(self) -> Self::Output {
        !(self as u8)
    }
}
