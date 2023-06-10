extern crate bitflags;

bitflags::bitflags! {
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct StatusFlags: u8 {
        const Carry = 0b00000001;
        const Zero = 0b00000010;
        const  InterruptDisable = 0b00000100;
        const Decimal = 0b00001000;
        const Break = 0b00010000;
        const Unused = 0b00100000;
        const Overflow = 0b01000000;
        const Negative = 0b10000000;
    }
}
