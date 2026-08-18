//! # `intel8051-simulator::decode` — fetch + operand-length decoding.
//!
//! The 8051 opcode byte alone determines how many operand bytes follow
//! (0, 1, or 2 — see `code/specs/07p-intel-8051-simulator.md`'s
//! "Encoding format" section).  This module is the pure, side-effect-
//! free half of the fetch-decode-execute loop: given the code-memory
//! array and a program counter, it returns a [`DecodedInstr`] carrying
//! the opcode, up to two already-fetched operand bytes, and the PC
//! value after the whole instruction — with **no** interpretation of
//! what those bytes *mean*.  [`crate::execute::execute`] does the
//! semantic dispatch.
//!
//! This split mirrors the historical-arch simulators' `decode`/
//! `execute` module boundary (e.g. `arm1_simulator::decode`, which
//! turns a raw 32-bit word into a `DecodedInstruction` before
//! `ARM1::step` executes it) — decode never touches CPU state, so it's
//! trivially unit-testable in isolation.

/// One decoded instruction: the opcode byte, its (up to 2) operand
/// bytes, and the PC bookkeeping needed to execute it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstr {
    /// The opcode byte itself — `execute::execute` re-derives the
    /// mnemonic and addressing mode from this (plus `operands`), the
    /// same way `simulator.py::_execute_one` dispatches on the byte it
    /// just fetched.
    pub opcode: u8,
    /// PC value the opcode byte was fetched from (before any bytes of
    /// this instruction were consumed).
    pub pc_before: u16,
    /// Operand bytes, in code-memory order.  Only the first
    /// `operand_count` entries are meaningful.
    pub operands: [u8; 2],
    /// How many of `operands` are valid: 0, 1, or 2.
    pub operand_count: u8,
    /// PC value immediately after this instruction's last byte —
    /// i.e. where a non-branching instruction resumes, and the base
    /// PC that `SJMP`/`Jcc`/`AJMP`/`ACALL`'s relative or page-relative
    /// math is computed from (matching the real 8051's "PC already
    /// incremented past the instruction being executed" behaviour).
    pub next_pc: u16,
}

/// How many operand bytes follow `opcode` (0, 1, or 2).
///
/// Ported from the opcode table in `code/specs/07p-intel-8051-
/// simulator.md` — every fixed opcode and instruction-family base is
/// classified by how many bytes `simulator.py`'s `_execute_one` fetches
/// for it (via `_fetch8`/`_fetch16` calls before or interleaved with
/// dispatch).
///
/// `HALT_OPCODE` (`0xA5`) is not listed anywhere below and correctly
/// falls through to the 0-operand-byte default: it is reserved/
/// undefined on real silicon, and this simulator's HALT convention
/// treats it as a bare 1-byte instruction.
pub fn operand_len(opcode: u8) -> u8 {
    // -- 2 operand bytes (3-byte instructions) --------------------------
    match opcode {
        0x85 // MOV dir, dir2
        | 0x75 // MOV dir, #imm
        | 0x90 // MOV DPTR, #imm16
        | 0x53 // ANL dir, #imm
        | 0x43 // ORL dir, #imm
        | 0x63 // XRL dir, #imm
        | 0x02 // LJMP addr16
        | 0x20 // JB bit, rel
        | 0x30 // JNB bit, rel
        | 0x10 // JBC bit, rel
        | 0xB5 // CJNE A, dir, rel
        | 0xB4 // CJNE A, #imm, rel
        | 0xD5 // DJNZ dir, rel
        | 0x12 // LCALL addr16
        => return 2,
        0xB8..=0xBF => return 2, // CJNE Rn, #imm, rel
        0xB6..=0xB7 => return 2, // CJNE @Ri, #imm, rel
        _ => {}
    }

    // -- 1 operand byte (2-byte instructions) ---------------------------
    match opcode {
        0xE5 // MOV A, dir
        | 0x74 // MOV A, #imm
        | 0xF5 // MOV dir, A
        | 0xC0 // PUSH dir
        | 0xD0 // POP dir
        | 0xC5 // XCH A, dir
        | 0x25 // ADD A, dir
        | 0x24 // ADD A, #imm
        | 0x35 // ADDC A, dir
        | 0x34 // ADDC A, #imm
        | 0x95 // SUBB A, dir
        | 0x94 // SUBB A, #imm
        | 0x05 // INC dir
        | 0x15 // DEC dir
        | 0x55 // ANL A, dir
        | 0x54 // ANL A, #imm
        | 0x52 // ANL dir, A
        | 0x45 // ORL A, dir
        | 0x44 // ORL A, #imm
        | 0x42 // ORL dir, A
        | 0x65 // XRL A, dir
        | 0x64 // XRL A, #imm
        | 0x62 // XRL dir, A
        | 0xC2 // CLR bit
        | 0xD2 // SETB bit
        | 0xB2 // CPL bit
        | 0x82 // ANL C, bit
        | 0xB0 // ANL C, /bit
        | 0x72 // ORL C, bit
        | 0xA0 // ORL C, /bit
        | 0xA2 // MOV C, bit
        | 0x92 // MOV bit, C
        | 0x80 // SJMP rel
        | 0x60 // JZ rel
        | 0x70 // JNZ rel
        | 0x40 // JC rel
        | 0x50 // JNC rel
        => return 1,
        0xA8..=0xAF => return 1, // MOV Rn, dir
        0x78..=0x7F => return 1, // MOV Rn, #imm
        0x88..=0x8F => return 1, // MOV dir, Rn
        0x86..=0x87 => return 1, // MOV dir, @Ri
        0xA6..=0xA7 => return 1, // MOV @Ri, dir
        0x76..=0x77 => return 1, // MOV @Ri, #imm
        0xD8..=0xDF => return 1, // DJNZ Rn, rel
        _ => {}
    }

    // -- AJMP / ACALL: opcode's low 5 bits carry the family tag, the
    //    high 3 bits carry addr[10:8].  Checked last since the fixed
    //    values and ranges above take priority (and, per the ISA
    //    layout, never collide with these two bit patterns).
    if opcode & 0x1F == crate::opcodes::AJMP_PATTERN
        || opcode & 0x1F == crate::opcodes::ACALL_PATTERN
    {
        return 1;
    }

    // Every 1-byte (no-operand) instruction, HALT, NOP, and any
    // genuinely reserved/unimplemented opcode.
    0
}

/// Decode the instruction at `pc` in `code` — fetch its opcode and
/// however many operand bytes `operand_len` says it needs, wrapping at
/// the 64 KiB code-memory boundary the same way a real PC wraps.
///
/// `code` must be at least [`crate::opcodes::CODE_SIZE`] bytes (the
/// simulator always hands over its full code array); this function
/// does not allocate or resize it.
pub fn decode(code: &[u8], pc: u16) -> DecodedInstr {
    let opcode = code[pc as usize];
    let len = operand_len(opcode);
    let mut operands = [0u8; 2];
    let mut p = pc.wrapping_add(1);
    for slot in operands.iter_mut().take(len as usize) {
        *slot = code[p as usize];
        p = p.wrapping_add(1);
    }
    DecodedInstr {
        opcode,
        pc_before: pc,
        operands,
        operand_count: len,
        next_pc: p,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcodes::HALT_OPCODE;

    fn code_with(bytes: &[u8]) -> Vec<u8> {
        let mut c = vec![0u8; crate::opcodes::CODE_SIZE];
        c[..bytes.len()].copy_from_slice(bytes);
        c
    }

    #[test]
    fn halt_has_zero_operand_bytes() {
        assert_eq!(operand_len(HALT_OPCODE), 0);
        let code = code_with(&[HALT_OPCODE]);
        let d = decode(&code, 0);
        assert_eq!(d.opcode, HALT_OPCODE);
        assert_eq!(d.operand_count, 0);
        assert_eq!(d.next_pc, 1);
    }

    #[test]
    fn mov_a_imm_has_one_operand_byte() {
        // MOV A, #42
        let code = code_with(&[0x74, 42]);
        let d = decode(&code, 0);
        assert_eq!(d.opcode, 0x74);
        assert_eq!(d.operand_count, 1);
        assert_eq!(d.operands[0], 42);
        assert_eq!(d.next_pc, 2);
    }

    #[test]
    fn ljmp_has_two_operand_bytes() {
        let code = code_with(&[0x02, 0x12, 0x34]);
        let d = decode(&code, 0);
        assert_eq!(d.operand_count, 2);
        assert_eq!(d.operands, [0x12, 0x34]);
        assert_eq!(d.next_pc, 3);
    }

    #[test]
    fn mov_a_rn_family_has_zero_operand_bytes() {
        for n in 0..8u8 {
            assert_eq!(operand_len(0xE8 + n), 0, "MOV A, R{n}");
        }
    }

    #[test]
    fn cjne_rn_imm_family_has_two_operand_bytes() {
        for n in 0..8u8 {
            assert_eq!(operand_len(0xB8 + n), 2, "CJNE R{n}, #imm, rel");
        }
    }

    #[test]
    fn ajmp_pattern_has_one_operand_byte() {
        for base in [0x01u8, 0x21, 0x41, 0x61, 0x81, 0xA1, 0xC1, 0xE1] {
            assert_eq!(operand_len(base), 1, "AJMP opcode 0x{base:02X}");
        }
    }

    #[test]
    fn acall_pattern_has_one_operand_byte() {
        for base in [0x11u8, 0x31, 0x51, 0x71, 0x91, 0xB1, 0xD1, 0xF1] {
            assert_eq!(operand_len(base), 1, "ACALL opcode 0x{base:02X}");
        }
    }

    #[test]
    fn decode_wraps_at_code_boundary() {
        let mut code = vec![0u8; crate::opcodes::CODE_SIZE];
        code[0xFFFF] = 0x74; // MOV A, #imm straddling the wrap
        code[0] = 99;
        let d = decode(&code, 0xFFFF);
        assert_eq!(d.opcode, 0x74);
        assert_eq!(d.operands[0], 99);
        assert_eq!(d.next_pc, 1);
    }
}
