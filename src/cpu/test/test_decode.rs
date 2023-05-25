use super::super::*;
use Instruction::*;
use Mode::*;

#[test]
fn test_decode() {
    use crate::cpu::decode::decode;
    // source http://www.6502.org/tutorials/6502opcodes.html

    assert_eq!(decode(0x69), (ADC, Imm, 2, false));
    assert_eq!(decode(0x65), (ADC, Zp, 3, false));
    assert_eq!(decode(0x75), (ADC, Zpx, 4, false));
    assert_eq!(decode(0x6D), (ADC, Abs, 4, false));
    assert_eq!(decode(0x7D), (ADC, AbsX, 4, true));
    assert_eq!(decode(0x79), (ADC, AbsY, 4, true));
    assert_eq!(decode(0x61), (ADC, IndX, 6, false));
    assert_eq!(decode(0x71), (ADC, IndY, 5, true));

    assert_eq!(decode(0x29), (AND, Imm, 2, false));
    assert_eq!(decode(0x25), (AND, Zp, 3, false));
    assert_eq!(decode(0x35), (AND, Zpx, 4, false));
    assert_eq!(decode(0x2D), (AND, Abs, 4, false));
    assert_eq!(decode(0x3D), (AND, AbsX, 4, true));
    assert_eq!(decode(0x39), (AND, AbsY, 4, true));
    assert_eq!(decode(0x21), (AND, IndX, 6, false));
    assert_eq!(decode(0x31), (AND, IndY, 5, true));

    assert_eq!(decode(0x0A), (ASL, A, 2, false));
    assert_eq!(decode(0x06), (ASL, Zp, 5, false));
    assert_eq!(decode(0x16), (ASL, Zpx, 6, false));
    assert_eq!(decode(0x0E), (ASL, Abs, 6, false));
    assert_eq!(decode(0x1E), (ASL, AbsX, 7, false));

    assert_eq!(decode(0x24), (BIT, Zp, 3, false));
    assert_eq!(decode(0x2C), (BIT, Abs, 4, false));

    assert_eq!(decode(0x10), (BPL, Rel, 2, true));
    assert_eq!(decode(0x30), (BMI, Rel, 2, true));
    assert_eq!(decode(0x50), (BVC, Rel, 2, true));
    assert_eq!(decode(0x70), (BVS, Rel, 2, true));
    assert_eq!(decode(0x90), (BCC, Rel, 2, true));
    assert_eq!(decode(0xB0), (BCS, Rel, 2, true));
    assert_eq!(decode(0xD0), (BNE, Rel, 2, true));
    assert_eq!(decode(0xF0), (BEQ, Rel, 2, true));

    assert_eq!(decode(0x00), (BRK, Imp, 7, false));

    assert_eq!(decode(0xC9), (CMP, Imm, 2, false));
    assert_eq!(decode(0xC5), (CMP, Zp, 3, false));
    assert_eq!(decode(0xD5), (CMP, Zpx, 4, false));
    assert_eq!(decode(0xCD), (CMP, Abs, 4, false));
    assert_eq!(decode(0xDD), (CMP, AbsX, 4, true));
    assert_eq!(decode(0xD9), (CMP, AbsY, 4, true));
    assert_eq!(decode(0xC1), (CMP, IndX, 6, false));
    assert_eq!(decode(0xD1), (CMP, IndY, 5, true));

    assert_eq!(decode(0xE0), (CPX, Imm, 2, false));
    assert_eq!(decode(0xE4), (CPX, Zp, 3, false));
    assert_eq!(decode(0xEC), (CPX, Abs, 4, false));

    assert_eq!(decode(0xC0), (CPY, Imm, 2, false));
    assert_eq!(decode(0xC4), (CPY, Zp, 3, false));
    assert_eq!(decode(0xCC), (CPY, Abs, 4, false));

    assert_eq!(decode(0xC6), (DEC, Zp, 5, false));
    assert_eq!(decode(0xD6), (DEC, Zpx, 6, false));
    assert_eq!(decode(0xCE), (DEC, Abs, 6, false));
    assert_eq!(decode(0xDE), (DEC, AbsX, 7, false));

    assert_eq!(decode(0x49), (EOR, Imm, 2, false));
    assert_eq!(decode(0x45), (EOR, Zp, 3, false));
    assert_eq!(decode(0x55), (EOR, Zpx, 4, false));
    assert_eq!(decode(0x4D), (EOR, Abs, 4, false));
    assert_eq!(decode(0x5D), (EOR, AbsX, 4, true));
    assert_eq!(decode(0x59), (EOR, AbsY, 4, true));
    assert_eq!(decode(0x41), (EOR, IndX, 6, false));
    assert_eq!(decode(0x51), (EOR, IndY, 5, true));

    assert_eq!(decode(0x18), (CLC, Imp, 2, false));
    assert_eq!(decode(0x38), (SEC, Imp, 2, false));
    assert_eq!(decode(0x58), (CLI, Imp, 2, false));
    assert_eq!(decode(0x78), (SEI, Imp, 2, false));
    assert_eq!(decode(0xB8), (CLV, Imp, 2, false));
    assert_eq!(decode(0xD8), (CLD, Imp, 2, false));
    assert_eq!(decode(0xF8), (SED, Imp, 2, false));

    assert_eq!(decode(0xE6), (INC, Zp, 5, false));
    assert_eq!(decode(0xF6), (INC, Zpx, 6, false));
    assert_eq!(decode(0xEE), (INC, Abs, 6, false));
    assert_eq!(decode(0xFE), (INC, AbsX, 7, false));

    assert_eq!(decode(0x4C), (JMP, Abs, 3, false));
    assert_eq!(decode(0x6C), (JMP, AbsInd, 5, false));

    assert_eq!(decode(0x20), (JSR, Abs, 6, false));

    assert_eq!(decode(0xA9), (LDA, Imm, 2, false));
    assert_eq!(decode(0xA5), (LDA, Zp, 3, false));
    assert_eq!(decode(0xB5), (LDA, Zpx, 4, false));
    assert_eq!(decode(0xAD), (LDA, Abs, 4, false));
    assert_eq!(decode(0xBD), (LDA, AbsX, 4, true));
    assert_eq!(decode(0xB9), (LDA, AbsY, 4, true));
    assert_eq!(decode(0xA1), (LDA, IndX, 6, false));
    assert_eq!(decode(0xB1), (LDA, IndY, 5, true));

    assert_eq!(decode(0xA2), (LDX, Imm, 2, false));
    assert_eq!(decode(0xA6), (LDX, Zp, 3, false));
    assert_eq!(decode(0xB6), (LDX, Zpy, 4, false));
    assert_eq!(decode(0xAE), (LDX, Abs, 4, false));
    assert_eq!(decode(0xBE), (LDX, AbsY, 4, true));

    assert_eq!(decode(0xA0), (LDY, Imm, 2, false));
    assert_eq!(decode(0xA4), (LDY, Zp, 3, false));
    assert_eq!(decode(0xB4), (LDY, Zpx, 4, false));
    assert_eq!(decode(0xAC), (LDY, Abs, 4, false));
    assert_eq!(decode(0xBC), (LDY, AbsX, 4, true));

    assert_eq!(decode(0x4A), (LSR, A, 2, false));
    assert_eq!(decode(0x46), (LSR, Zp, 5, false));
    assert_eq!(decode(0x56), (LSR, Zpx, 6, false));
    assert_eq!(decode(0x4E), (LSR, Abs, 6, false));
    assert_eq!(decode(0x5E), (LSR, AbsX, 7, false));

    assert_eq!(decode(0xEA), (NOP, Imp, 2, false));

    assert_eq!(decode(0x09), (ORA, Imm, 2, false));
    assert_eq!(decode(0x05), (ORA, Zp, 3, false));
    assert_eq!(decode(0x15), (ORA, Zpx, 4, false));
    assert_eq!(decode(0x0D), (ORA, Abs, 4, false));
    assert_eq!(decode(0x1D), (ORA, AbsX, 4, true));
    assert_eq!(decode(0x19), (ORA, AbsY, 4, true));
    assert_eq!(decode(0x01), (ORA, IndX, 6, false));
    assert_eq!(decode(0x11), (ORA, IndY, 5, true));

    assert_eq!(decode(0xAA), (TAX, Imp, 2, false));
    assert_eq!(decode(0x8A), (TXA, Imp, 2, false));
    assert_eq!(decode(0xCA), (DEX, Imp, 2, false));
    assert_eq!(decode(0xE8), (INX, Imp, 2, false));
    assert_eq!(decode(0xA8), (TAY, Imp, 2, false));
    assert_eq!(decode(0x98), (TYA, Imp, 2, false));
    assert_eq!(decode(0x88), (DEY, Imp, 2, false));
    assert_eq!(decode(0xC8), (INY, Imp, 2, false));

    assert_eq!(decode(0x2A), (ROL, A, 2, false));
    assert_eq!(decode(0x26), (ROL, Zp, 5, false));
    assert_eq!(decode(0x36), (ROL, Zpx, 6, false));
    assert_eq!(decode(0x2E), (ROL, Abs, 6, false));
    assert_eq!(decode(0x3E), (ROL, AbsX, 7, false));

    assert_eq!(decode(0x6A), (ROR, A, 2, false));
    assert_eq!(decode(0x66), (ROR, Zp, 5, false));
    assert_eq!(decode(0x76), (ROR, Zpx, 6, false));
    assert_eq!(decode(0x6E), (ROR, Abs, 6, false));
    assert_eq!(decode(0x7E), (ROR, AbsX, 7, false));

    assert_eq!(decode(0x40), (RTI, Imp, 6, false));
    assert_eq!(decode(0x60), (RTS, Imp, 6, false));

    assert_eq!(decode(0xE9), (SBC, Imm, 2, false));
    assert_eq!(decode(0xE5), (SBC, Zp, 3, false));
    assert_eq!(decode(0xF5), (SBC, Zpx, 4, false));
    assert_eq!(decode(0xED), (SBC, Abs, 4, false));
    assert_eq!(decode(0xFD), (SBC, AbsX, 4, true));
    assert_eq!(decode(0xF9), (SBC, AbsY, 4, true));
    assert_eq!(decode(0xE1), (SBC, IndX, 6, false));
    assert_eq!(decode(0xF1), (SBC, IndY, 5, true));

    assert_eq!(decode(0x85), (STA, Zp, 3, false));
    assert_eq!(decode(0x95), (STA, Zpx, 4, false));
    assert_eq!(decode(0x8D), (STA, Abs, 4, false));
    assert_eq!(decode(0x9D), (STA, AbsX, 5, false));
    assert_eq!(decode(0x99), (STA, AbsY, 5, false));
    assert_eq!(decode(0x81), (STA, IndX, 6, false));
    assert_eq!(decode(0x91), (STA, IndY, 6, false));

    assert_eq!(decode(0x9A), (TXS, Imp, 2, false));
    assert_eq!(decode(0xBA), (TSX, Imp, 2, false));
    assert_eq!(decode(0x48), (PHA, Imp, 3, false));
    assert_eq!(decode(0x68), (PLA, Imp, 4, false));
    assert_eq!(decode(0x08), (PHP, Imp, 3, false));
    assert_eq!(decode(0x28), (PLP, Imp, 4, false));

    assert_eq!(decode(0x86), (STX, Zp, 3, false));
    assert_eq!(decode(0x96), (STX, Zpy, 4, false));
    assert_eq!(decode(0x8E), (STX, Abs, 4, false));

    assert_eq!(decode(0x84), (STY, Zp, 3, false));
    assert_eq!(decode(0x94), (STY, Zpx, 4, false));
    assert_eq!(decode(0x8C), (STY, Abs, 4, false));
}
