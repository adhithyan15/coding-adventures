//! # `intel8051-encoder` — pure Intel 8051 (MCS-51) instruction encoder.
//!
//! Mirror of [`arm1_encoder`] / [`mips_r2000_encoder`] /
//! [`intel8008_encoder`] for the Intel 8051 (1980) — the most-
//! manufactured CPU architecture in history (20+ billion units).
//! Fourth lane of the 9-architecture expansion following the pattern
//! documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## What's inside
//!
//! 1. **Encoder re-exports** — `encode_mov_a_imm`/`encode_halt` come
//!    from [`intel8051_simulator::encoding`], the in-tree source of
//!    truth for 8051 bit-level encoding.  Re-exporting (rather than
//!    duplicating the byte-layout logic) means any future ISA-spec fix
//!    in the simulator propagates to `intel8051-backend` automatically
//!    — the same pattern `arm1-encoder` uses over `arm1-simulator`.
//! 2. **Opcode constants** — `MOV_A_IMM` (`0x74`) and `HALT_OPCODE`
//!    (`0xA5`), re-exported for callers that want the raw byte value
//!    rather than an `encode_*` call.
//! 3. **Capacity constant** — `IMM8_MAX` (255), the encodable range
//!    for `MOV A, #imm`'s 8-bit immediate operand.
//!
//! No IR knowledge lives here.  [`intel8051-backend`] is the consumer
//! that maps CIR ops onto these encoder calls.
//!
//! ## Why `MOV A, #imm` + the HALT sentinel, and not something else?
//!
//! The 8051's accumulator (`A`) is the implicit destination/source for
//! almost every arithmetic and data-transfer instruction — the same
//! "one working register" role the Intel 8008's `A` plays in
//! `intel8008-backend`.  `MOV A, #imm` (opcode `0x74`, 2 bytes: an
//! opcode byte plus an immediate byte) is the natural "materialise a
//! constant" instruction, exactly mirroring `intel8008-backend`'s
//! `MVI A, n`.
//!
//! There is genuinely no HALT instruction on the 8051 — see
//! `intel8051_simulator::opcodes::HALT_OPCODE`'s doc comment and
//! `code/specs/intel8051-backend.md` for the full derivation of why
//! this backend reuses the simulator's already-established `0xA5`
//! sentinel-opcode convention (ported from the Python reference,
//! `intel8051_simulator.state.HALT_OPCODE`, spec 07p) rather than
//! inventing self-jump (`SJMP $`) detection — the historically-
//! idiomatic alternative for a real, running 8051 program, but a worse
//! fit for a minimal-viable *emit-only* backend whose only job is to
//! mark "the trivial ROM is done" for a simulator to detect.
//!
//! ## Quick start
//!
//! ```
//! use intel8051_encoder::{encode_mov_a_imm, encode_halt, HALT_OPCODE};
//!
//! // const_i64 v=42 lowered to `MOV A, #42` — the first bytes
//! // `intel8051-backend` emits for the canonical Twig `42` program.
//! assert_eq!(encode_mov_a_imm(42), [0x74, 42]);
//!
//! // ret lowered to the HALT sentinel.
//! assert_eq!(encode_halt(), HALT_OPCODE);
//! assert_eq!(HALT_OPCODE, 0xA5);
//! ```

// ===========================================================================
// Encoder re-exports
// ===========================================================================

pub use intel8051_simulator::encoding::{encode_halt, encode_mov_a_imm, encode_mov_rn_imm};

// ===========================================================================
// Opcode / capacity constant re-exports
// ===========================================================================

/// `MOV A, #imm` opcode byte (`0x74`) — the instruction `const_*`
/// lowers to.
pub use intel8051_simulator::opcodes::MOV_A_IMM;

/// The HALT sentinel opcode (`0xA5`, reserved/undefined on real 8051
/// silicon) — the instruction `ret_*`/`ret_void` lowers to.  See the
/// crate-level doc comment for the full rationale.
pub use intel8051_simulator::opcodes::HALT_OPCODE;

/// Maximum unsigned 8-bit `MOV A, #imm` / `MOV Rn, #imm` immediate
/// (255).
pub use intel8051_simulator::opcodes::IMM8_MAX;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halt_opcode_value() {
        assert_eq!(HALT_OPCODE, 0xA5);
    }

    #[test]
    fn encode_halt_matches_constant() {
        assert_eq!(encode_halt(), HALT_OPCODE);
    }

    #[test]
    fn mov_a_imm_opcode_value() {
        assert_eq!(MOV_A_IMM, 0x74);
    }

    #[test]
    fn imm8_max_is_255() {
        assert_eq!(IMM8_MAX, 255);
    }

    #[test]
    fn canonical_const_42_bytes() {
        // First instruction of the Twig `42` lowering: MOV A, #42.
        assert_eq!(encode_mov_a_imm(42), [0x74, 42]);
    }

    #[test]
    fn canonical_const_42_then_halt_bytes() {
        let mut bytes = encode_mov_a_imm(42).to_vec();
        bytes.push(encode_halt());
        assert_eq!(bytes, vec![0x74, 0x2A, 0xA5]);
    }

    #[test]
    fn encode_mov_rn_imm_reexported() {
        assert_eq!(encode_mov_rn_imm(0, 7), [0x78, 7]);
    }
}
