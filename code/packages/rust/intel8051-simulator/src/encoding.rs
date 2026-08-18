//! # `intel8051-simulator::encoding` — pure `encode_*` helpers.
//!
//! This is the module `intel8051-encoder` re-exports from — analogous
//! to `arm1_simulator::encode_mov_imm`/`encode_halt` being the source
//! of truth `arm1-encoder` re-exports rather than duplicates.  Keeping
//! the encoders here (next to `opcodes`/`decode`/`execute`, which
//! together are the actual bit-layout authority) means any future ISA
//! fix propagates to `intel8051-encoder` — and therefore
//! `intel8051-backend` — automatically.
//!
//! Only the handful of encodings the minimal-viable
//! `intel8051-backend` needs (`MOV A, #imm` + the HALT sentinel) plus
//! a couple of closely related ones (`MOV Rn, #imm`, `NOP`) are
//! exposed — this is an encoder for the backend's trivial-ROM scope,
//! not a full 8051 assembler.

use crate::opcodes::{HALT_OPCODE, MOV_A_IMM, MOV_RN_IMM_BASE, NOP};

/// Encode `MOV A, #n` — 2 bytes: `[0x74, n]`.  The instruction
/// `intel8051-backend` lowers every `const_*` CIR op to.
#[inline]
pub fn encode_mov_a_imm(n: u8) -> [u8; 2] {
    [MOV_A_IMM, n]
}

/// Encode `MOV Rn, #imm` — 2 bytes: `[0x78 + (n & 7), imm]`.
#[inline]
pub fn encode_mov_rn_imm(n: u8, imm: u8) -> [u8; 2] {
    [MOV_RN_IMM_BASE + (n & 7), imm]
}

/// Encode the HALT sentinel — 1 byte: `0xA5`.  See
/// `crate::opcodes::HALT_OPCODE`'s doc comment for why this reserved
/// opcode (not a real 8051 instruction) is the chosen "program is
/// done" convention.
#[inline]
pub fn encode_halt() -> u8 {
    HALT_OPCODE
}

/// Encode `NOP` — 1 byte: `0x00`.
#[inline]
pub fn encode_nop() -> u8 {
    NOP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_mov_a_imm_canonical_42() {
        assert_eq!(encode_mov_a_imm(42), [0x74, 42]);
    }

    #[test]
    fn encode_mov_a_imm_zero_and_max() {
        assert_eq!(encode_mov_a_imm(0), [0x74, 0x00]);
        assert_eq!(encode_mov_a_imm(255), [0x74, 0xFF]);
    }

    #[test]
    fn encode_mov_rn_imm_selects_register() {
        assert_eq!(encode_mov_rn_imm(0, 5), [0x78, 5]);
        assert_eq!(encode_mov_rn_imm(7, 5), [0x7F, 5]);
        // Register index masks to 3 bits.
        assert_eq!(encode_mov_rn_imm(8, 5), [0x78, 5]);
    }

    #[test]
    fn encode_halt_is_reserved_a5() {
        assert_eq!(encode_halt(), 0xA5);
    }

    #[test]
    fn encode_nop_is_zero() {
        assert_eq!(encode_nop(), 0x00);
    }
}
