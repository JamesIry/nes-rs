use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use strum_macros::Display;

pub trait Controller {
    fn read_value(&self) -> u8;
}

pub struct NulController {}
impl NulController {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for NulController {
    fn default() -> Self {
        Self::new()
    }
}
impl Controller for NulController {
    fn read_value(&self) -> u8 {
        0
    }
}

pub struct JoyPad {
    current_buttons: u8,
}
impl JoyPad {
    pub fn new() -> Self {
        Self { current_buttons: 0 }
    }
    pub fn set_buttons(&mut self, buttons: u8) {
        self.current_buttons = buttons;
    }
}

impl Default for JoyPad {
    fn default() -> Self {
        Self::new()
    }
}

impl Controller for JoyPad {
    fn read_value(&self) -> u8 {
        self.current_buttons
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Display)]
#[repr(u8)]
pub enum JoyPadButton {
    A = 0b00000001,
    B = 0b00000010,
    Select = 0b00000100,
    Start = 0b00001000,
    Up = 0b00010000,
    Down = 0b00100000,
    Left = 0b01000000,
    Right = 0b10000000,
}

impl BitOr<Self> for JoyPadButton {
    type Output = u8;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as u8 | rhs as u8
    }
}

impl BitOr<u8> for JoyPadButton {
    type Output = u8;

    fn bitor(self, rhs: u8) -> Self::Output {
        self as u8 | rhs
    }
}

impl BitOr<JoyPadButton> for u8 {
    type Output = u8;

    fn bitor(self, rhs: JoyPadButton) -> Self::Output {
        self | rhs as u8
    }
}

impl BitOrAssign<JoyPadButton> for u8 {
    fn bitor_assign(&mut self, rhs: JoyPadButton) {
        *self |= rhs as u8
    }
}

impl BitAnd<Self> for JoyPadButton {
    type Output = u8;

    fn bitand(self, rhs: Self) -> Self::Output {
        self as u8 & rhs as u8
    }
}

impl BitAnd<u8> for JoyPadButton {
    type Output = u8;

    fn bitand(self, rhs: u8) -> Self::Output {
        self as u8 & rhs
    }
}

impl BitAnd<JoyPadButton> for u8 {
    type Output = u8;

    fn bitand(self, rhs: JoyPadButton) -> Self::Output {
        self & rhs as u8
    }
}

impl BitAndAssign<JoyPadButton> for u8 {
    fn bitand_assign(&mut self, rhs: JoyPadButton) {
        *self &= rhs as u8
    }
}

impl Not for JoyPadButton {
    type Output = u8;

    fn not(self) -> Self::Output {
        !(self as u8)
    }
}
