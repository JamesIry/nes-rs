use std::ops::BitAnd;
use std::ops::BitAndAssign;
use std::ops::BitOr;
use std::ops::BitOrAssign;
use std::ops::Not;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Flag {
    Carry = 0b00000001,
    Zero = 0b00000010,
    InterruptDisable = 0b00000100,
    Decimal = 0b00001000,
    Break = 0b00010000,
    Unused = 0b00100000,
    Overflow = 0b01000000,
    Negative = 0b10000000,
}

impl BitOr<Self> for Flag {
    type Output = u8;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as u8 | rhs as u8
    }
}

impl BitOr<u8> for Flag {
    type Output = u8;

    fn bitor(self, rhs: u8) -> Self::Output {
        self as u8 | rhs
    }
}

impl BitOr<Flag> for u8 {
    type Output = u8;

    fn bitor(self, rhs: Flag) -> Self::Output {
        self | rhs as u8
    }
}

impl BitOrAssign<Flag> for u8 {
    fn bitor_assign(&mut self, rhs: Flag) {
        *self |= rhs as u8
    }
}

impl BitAnd<Self> for Flag {
    type Output = u8;

    fn bitand(self, rhs: Self) -> Self::Output {
        self as u8 & rhs as u8
    }
}

impl BitAnd<u8> for Flag {
    type Output = u8;

    fn bitand(self, rhs: u8) -> Self::Output {
        self as u8 & rhs
    }
}

impl BitAnd<Flag> for u8 {
    type Output = u8;

    fn bitand(self, rhs: Flag) -> Self::Output {
        self & rhs as u8
    }
}

impl BitAndAssign<Flag> for u8 {
    fn bitand_assign(&mut self, rhs: Flag) {
        *self &= rhs as u8
    }
}

impl Not for Flag {
    type Output = u8;

    fn not(self) -> Self::Output {
        !(self as u8)
    }
}
