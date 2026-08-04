//! SPARC V8 instruction decoder.
//!
//! SPARC V8 has four instruction formats, distinguished by bits 31:30:
//!
//! ```text
//!  31 30  29          0
//!  ┌──┬──┬────────────────────────────────────────────────────────┐
//!  │ op│  │         ...                                           │
//!  └──┴──┴────────────────────────────────────────────────────────┘
//!
//!  op=00  Format 2: SETHI, Bicc, NOP
//!  op=01  Format 1: CALL (30-bit PC-relative displacement)
//!  op=10  Format 3: integer ALU (rd, op3, rs1, rs2/imm13)
//!  op=11  Format 3: load/store (rd, op3, rs1, rs2/imm13)
//! ```
//!
//! ## Format 2 (op=00)
//!
//! ```text
//!  31 30  29 25  24 22  21                    0
//!  ┌──┬──┬─────┬────┬──────────────────────────┐
//!  │00│  │ rd  │op2 │      imm22               │
//!  └──┴──┴─────┴────┴──────────────────────────┘
//!
//!  op2=100 → SETHI   (rd ≠ 0)
//!  op2=000 → NOP     (rd = 0, imm22 = 0)
//!  op2=010 → Bicc    (a=bit 29, cond=bits 28:25, disp22=bits 21:0)
//! ```
//!
//! ## Format 1 (op=01)
//!
//! ```text
//!  31 30  29                                   0
//!  ┌──┬──┬────────────────────────────────────┐
//!  │01│  │         disp30                     │
//!  └──┴──┴────────────────────────────────────┘
//! ```
//!
//! ## Format 3 (op=10 or op=11)
//!
//! ```text
//!  31 30  29 25  24 19  18 14  13  12      5   4  0
//!  ┌──┬──┬─────┬──────┬────┬──┬───────────┬──────┐
//!  │op│  │ rd  │ op3  │rs1 │i │  asi/shamt│  rs2 │  i=0: reg form
//!  ├──┴──┴─────┴──────┴────┴──┴───────────┴──────┤
//!  │op│  │ rd  │ op3  │rs1 │1 │      simm13       │  i=1: imm form
//!  └──┴──┴─────┴──────┴────┴──┴───────────────────┘
//! ```

/// Decoded instruction.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // ── Format 1 ─────────────────────────────────────────────────────────────
    Call { disp30: u32 },

    // ── Format 2 ─────────────────────────────────────────────────────────────
    Sethi { rd: u32, imm22: u32 },
    Nop,
    Bicc { cond: u8, disp22: u32, annul: bool },

    // ── Format 3 — integer ALU (op=10) ───────────────────────────────────────
    /// Most ALU ops share this shape.
    Alu { op3: u8, rd: u32, rs1: u32, src2: Src2 },

    // ── Format 3 — load/store (op=11) ────────────────────────────────────────
    Load  { op3: u8, rd: u32, rs1: u32, src2: Src2 },
    Store { op3: u8, rd: u32, rs1: u32, src2: Src2 },

    // ── Trap ─────────────────────────────────────────────────────────────────
    Ticc { cond: u8, rs1: u32, src2: Src2 },

    /// Any word we don't recognise.
    Illegal(u32),
}

/// Source operand 2 — either a register or a sign-extended 13-bit immediate.
#[derive(Debug, Clone, PartialEq)]
pub enum Src2 {
    Reg(u32),
    Imm(u32),
}

/// Decode a single 32-bit instruction word.
pub fn decode(word: u32) -> Instruction {
    let op = (word >> 30) & 0x3;
    match op {
        0b01 => {
            // Format 1: CALL
            let disp30 = word & 0x3FFF_FFFF;
            Instruction::Call { disp30 }
        }
        0b00 => decode_f2(word),
        0b10 => decode_f3_alu(word),
        0b11 => decode_f3_mem(word),
        _ => Instruction::Illegal(word),
    }
}

fn decode_f2(word: u32) -> Instruction {
    let op2 = (word >> 22) & 0x7;
    match op2 {
        0b100 => {
            let rd = (word >> 25) & 0x1F;
            let imm22 = word & 0x003F_FFFF;
            if rd == 0 && imm22 == 0 {
                Instruction::Nop
            } else {
                Instruction::Sethi { rd, imm22 }
            }
        }
        0b000 => {
            // NOP: rd=0, imm22=0; also covers the raw NOP encoding.
            Instruction::Nop
        }
        0b010 => {
            // Bicc
            let annul = (word >> 29) & 1 == 1;
            let cond = ((word >> 25) & 0xF) as u8;
            let disp22 = word & 0x003F_FFFF;
            Instruction::Bicc { cond, disp22, annul }
        }
        _ => Instruction::Illegal(word),
    }
}

fn src2(word: u32) -> Src2 {
    let i = (word >> 13) & 1;
    if i == 0 {
        Src2::Reg(word & 0x1F)
    } else {
        use crate::bits::sext13;
        Src2::Imm(sext13(word))
    }
}

fn decode_f3_alu(word: u32) -> Instruction {
    let op3 = ((word >> 19) & 0x3F) as u8;
    let rd = (word >> 25) & 0x1F;
    let rs1 = (word >> 14) & 0x1F;
    let s2 = src2(word);

    match op3 {
        // Ticc (op3 = 0x3A)
        0x3A => Instruction::Ticc { cond: ((word >> 25) & 0xF) as u8, rs1, src2: s2 },
        _ => Instruction::Alu { op3, rd, rs1, src2: s2 },
    }
}

fn decode_f3_mem(word: u32) -> Instruction {
    let op3 = ((word >> 19) & 0x3F) as u8;
    let rd = (word >> 25) & 0x1F;
    let rs1 = (word >> 14) & 0x1F;
    let s2 = src2(word);

    // op3 bit 2 set → store; otherwise load.
    if op3 & 0x04 != 0 {
        Instruction::Store { op3, rd, rs1, src2: s2 }
    } else {
        Instruction::Load { op3, rd, rs1, src2: s2 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_nop() {
        // NOP = SETHI 0, %g0 = 0x0100_0000
        let word = 0x0100_0000u32;
        assert_eq!(decode(word), Instruction::Nop);
    }

    #[test]
    fn decode_sethi() {
        // SETHI 1, %o0  (rd=%o0=8, imm22=1)
        let word = (8 << 25) | (0b100 << 22) | 1;
        match decode(word) {
            Instruction::Sethi { rd, imm22 } => {
                assert_eq!(rd, 8);
                assert_eq!(imm22, 1);
            }
            other => panic!("expected Sethi, got {:?}", other),
        }
    }

    #[test]
    fn decode_call() {
        let word = (0b01u32 << 30) | 0x42;
        match decode(word) {
            Instruction::Call { disp30 } => assert_eq!(disp30, 0x42),
            other => panic!("expected Call, got {:?}", other),
        }
    }

    #[test]
    fn decode_bicc_ba() {
        // BA = cond=1000(8), op2=010, op=00; annul=0, disp22=4
        let word = (0b1000 << 25) | (0b010 << 22) | 4;
        match decode(word) {
            Instruction::Bicc { cond, disp22, annul } => {
                assert_eq!(cond, 8);
                assert_eq!(disp22, 4);
                assert!(!annul);
            }
            other => panic!("expected Bicc, got {:?}", other),
        }
    }

    #[test]
    fn decode_alu_reg_form() {
        // ADD %o0, %o1, %o0 — op=10, rd=8, op3=0, rs1=8, i=0, rs2=9
        let word = ((0b10u32 << 30) | (8 << 25)) | (8 << 14) | 9;
        match decode(word) {
            Instruction::Alu { op3, rd, rs1, src2 } => {
                assert_eq!(op3, 0);
                assert_eq!(rd, 8);
                assert_eq!(rs1, 8);
                assert_eq!(src2, Src2::Reg(9));
            }
            other => panic!("expected Alu, got {:?}", other),
        }
    }

    #[test]
    fn decode_alu_imm_form() {
        // ADD %o0, 5, %o0 — op=10, rd=8, op3=0, rs1=8, i=1, simm13=5
        let word = ((0b10u32 << 30) | (8 << 25)) | (8 << 14) | (1 << 13) | 5;
        match decode(word) {
            Instruction::Alu { src2, .. } => assert_eq!(src2, Src2::Imm(5)),
            other => panic!("expected Alu, got {:?}", other),
        }
    }
}
