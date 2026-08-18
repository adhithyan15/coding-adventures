//! # `intel8080-encoder` — pure Intel 8080 instruction encoder.
//!
//! Mirror of [`mips_r2000_encoder`] / [`intel8008_encoder`] /
//! [`armv7_encoder`] for the Intel 8080 (1974) — direct successor to the
//! 8008, and the CPU inside the Altair 8800.
//!
//! Third lane of the 9-architecture expansion.
//!
//! ## ISA quick reference (subset used here)
//!
//! | Mnemonic | Opcode | Bytes | Effect |
//! |----------|--------|-------|--------|
//! | `HLT` | `0x76` | 1 | halt — `01_110_110` |
//! | `MVI A, n` | `0x3E nn` | 2 | A ← 8-bit immediate `n` |
//! | `RET` | `0xC9` | 1 | return from subroutine |
//!
//! ## Quick start
//!
//! ```
//! use intel8080_encoder::{encode_mvi_a, HLT, RET};
//!
//! // MVI A, 42 → [0x3E, 0x2A]
//! assert_eq!(encode_mvi_a(42), vec![0x3E, 0x2A]);
//! assert_eq!(HLT, 0x76);
//! assert_eq!(RET, 0xC9);
//! ```

// ===========================================================================
// Encoder re-exports
// ===========================================================================
//
// `intel8080-simulator::encoding` / `::opcodes` are the in-tree source of
// truth for the Intel 8080 bit layout — it's the only place the opcode
// constants and the instruction-byte-sequence packing logic lives.  We
// re-export the subset `intel8080-backend` actually uses.

pub use intel8080_simulator::encoding::{assemble, encode_mvi_a};
pub use intel8080_simulator::opcodes::{HLT, RET};

// ===========================================================================
// Register-role constants
// ===========================================================================
//
// The 8080 names its working registers rather than numbering them (A, B,
// C, D, E, H, L — not R0..R7 like MIPS/ARM), so `intel8080-backend`
// addresses the accumulator directly rather than through an indexed
// register file.  Re-exported here as the 3-bit register-field encoding
// `intel8080-simulator::opcodes` already defines, so a future increment
// that needs to name other registers (for a real allocator) can do so
// without duplicating the encoding.

pub use intel8080_simulator::opcodes::{REG_A, REG_B, REG_C, REG_D, REG_E, REG_H, REG_L, REG_M};

// ===========================================================================
// Capacity constants
// ===========================================================================

/// Maximum unsigned 8-bit `MVI A` immediate (= 255).
pub const MVI_MAX: u8 = 255;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ret_byte_value() {
        assert_eq!(RET, 0xC9);
    }

    #[test]
    fn hlt_byte_value() {
        assert_eq!(HLT, 0x76);
    }

    #[test]
    fn register_constants_match_convention() {
        assert_eq!(REG_A, 7);
        assert_eq!(REG_B, 0);
        assert_eq!(REG_M, 6);
    }

    #[test]
    fn canonical_const_42_bytes() {
        // First (and only) instruction of the Twig `42` lowering:
        // MVI A, 42
        assert_eq!(encode_mvi_a(42), vec![0x3E, 0x2A]);
    }

    #[test]
    fn assemble_then_hlt() {
        assert_eq!(assemble(&[encode_mvi_a(42), vec![HLT]]), vec![0x3E, 0x2A, 0x76]);
    }

    #[test]
    fn mvi_max_is_255() {
        assert_eq!(MVI_MAX, 255);
    }
}
