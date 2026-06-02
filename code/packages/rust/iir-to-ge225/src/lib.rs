//! # iir-to-ge225 — IIR → GE-225 machine code backend (v0.1.0 skeleton).
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `Vec<u8>` of encoded
//! 20-bit GE-225 instruction words (packed 3 bytes per word, big-
//! endian, with the top 4 bits of byte 0 always zero).
//!
//! ## Why a GE-225 backend?
//!
//! The **GE-225** (1959) was the General Electric mainframe at
//! Dartmouth College where **John Kemeny and Thomas Kurtz designed
//! Dartmouth BASIC in 1964**.  BASIC ran on this very machine — the
//! 1.7 µs cycle time and 20-bit word size shaped the language's
//! defaults in ways still visible 60 years later.
//!
//! In this codebase the GE-225 is primarily a **BASIC fit** per
//! MULTILANG-ARCHITECTURE-BACKENDS.md §A5.  Compiling BASIC source
//! to GE-225 bytes is a small piece of computing history made
//! queryable through the LANG VM pipeline.
//!
//! Adding the GE-225 gives the LANG VM matrix its fifth and most
//! exotic architecture backend:
//!
//! | | RV32I (A1) | 8008 (A2) | ARMv7 (A3) | 4004 (A4) | **GE-225 (A5)** |
//! |---|---|---|---|---|---|
//! | Width | 32-bit | 8-bit | 32-bit | 4-bit | **20-bit** |
//! | Year shipped | 2015 | 1972 | 2005 | 1971 | **1959** |
//! | Style | Modern RISC | Accumulator CISC | RISC + cond | Tiny MCS-4 | **Mainframe accumulator** |
//! | LANG VM fit | Generic | Oct's native | Phone-class | Brainfuck | **BASIC's birthplace** |
//!
//! ## Scope of v0.1.0 (A5)
//!
//! This release is a **skeleton**: any IIR module lowers to a
//! single `HLT` — the all-zeros 20-bit word, packed as
//! `[0x00, 0x00, 0x00]`.  No instruction selection yet; that
//! arrives in A5+ and beyond.
//!
//! ## Why `Vec<u8>` output for a 20-bit-word machine?
//!
//! - **Cross-backend uniformity.**  Every other backend emits
//!   `Vec<u8>` or a `Vec<u32>` that's trivially flattened to bytes.
//!   Bytes round-trip through every host filesystem without
//!   alignment surprises.
//! - **3-byte word packing.**  Each 20-bit GE-225 word is emitted
//!   as 3 bytes (24 bits total), big-endian, with the top 4 bits
//!   of byte 0 always zero.  This wastes 4 bits per word (~17 %
//!   overhead) but means a downstream simulator can read 3 bytes,
//!   mask off the top 4 bits, and recover the original 20-bit
//!   word.
//!
//! ## The halt sentinel — all-zeros HLT
//!
//! The GE-225's `HLT` instruction is the all-zeros 20-bit word.
//! Emitted at the start of a program ROM, this halts the machine.
//!
//! ```text
//! 20-bit word: 0000_0000_0000_0000_0000
//!    ↓ packed into 3 big-endian bytes
//! [0x00, 0x00, 0x00]
//! ```
//!
//! This is the documented HLT encoding in the GE-225 reference
//! manual.  Alternative halt idioms (e.g. unconditional branch to
//! self, `BR $.` in mnemonics) would also work but produce less
//! visually obvious bytes; the all-zeros HLT is preferred for
//! skeleton purposes.
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::IIRModule;
//! use iir_to_ge225::{validate_for_ge225, lower_iir_to_ge225, IIRGe225Config};
//!
//! let module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![],
//!     entry_point: None,
//!     language: "demo".into(),
//!     exports: vec![],
//!     imports: vec![],
//! };
//!
//! assert!(validate_for_ge225(&module).is_empty());
//!
//! let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
//!     .expect("lowering should succeed");
//! // HLT = all-zeros 20-bit word, packed as 3 bytes.
//! assert_eq!(bytes, vec![0x00, 0x00, 0x00]);
//! ```

use interpreter_ir::IIRModule;
use std::fmt;

// ===========================================================================
// GE-225 opcode constants
// ===========================================================================

/// Canonical "halt" sentinel for the GE-225 — three bytes:
/// `[0x00, 0x00, 0x00]` (= the all-zeros 20-bit HLT word, packed
/// big-endian with the top 4 bits of byte 0 zero).
///
/// The GE-225 reference manual documents the all-zeros 20-bit word
/// as the `HLT` instruction.  When emitted at the start of program
/// memory, this halts the machine deterministically — every GE-225
/// simulator and the historical silicon recognises this pattern as
/// "stop".
///
/// ## Word packing
///
/// GE-225 instructions are 20 bits wide.  We pack each word into 3
/// bytes, big-endian, with the top 4 bits of byte 0 always zero
/// (since 20 bits < 24 bits in 3 bytes):
///
/// ```text
/// byte 0: 0000 BBBB   (top 4 bits zero + bits 19..16 of word)
/// byte 1: BBBB BBBB   (bits 15..8 of word)
/// byte 2: BBBB BBBB   (bits 7..0 of word)
/// ```
///
/// For HLT (word = 0x00000): every bit is zero, so all 3 bytes are
/// `0x00`.  Subsequent slices that emit non-zero words use the
/// same big-endian packing.
///
/// ## Why HLT over branch-to-self?
///
/// | Candidate | Pros | Cons |
/// |-----------|------|------|
/// | All-zeros HLT (this) | Documented in the GE-225 reference manual; bytes are unambiguous | None for skeleton purposes |
/// | `BR $.` (branch to self) | Pure single-word idiom | Encoded as a non-zero opcode; doesn't visually distinguish "halt" from arbitrary branch |
/// | Unimplemented opcode | Forces a trap | GE-225 reaction to unused opcodes wasn't formally specified — implementation-defined |
///
/// The all-zeros HLT is the canonical and historically attested
/// choice.
pub const HALT_WORD: [u8; 3] = [0x00, 0x00, 0x00];

// ===========================================================================
// IIRGe225Config
// ===========================================================================

/// Configuration for the IIR → GE-225 lowering pass.
///
/// Currently only the module name is configurable, reserved for
/// future symbol-table / memory-image emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRGe225Config {
    /// Module name — reserved for future symbol-table / `.bin`
    /// header use.
    pub module_name: String,
}

impl IIRGe225Config {
    /// Build a config with a custom module name.
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }
}

impl Default for IIRGe225Config {
    fn default() -> Self {
        Self {
            module_name: "iir_module".into(),
        }
    }
}

// ===========================================================================
// IIRGe225Error
// ===========================================================================

/// Errors that can occur during IIR → GE-225 lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRGe225Error {
    /// The module failed pre-flight validation.
    ValidationFailed(Vec<String>),
    /// An IIR opcode not yet supported by this backend.  v0.1.0
    /// doesn't lower any instructions, so a non-empty function body
    /// would surface this in a future version.
    UnsupportedOp { function: String, op: String },
    /// A type hint that does not map to any GE-225 representation.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape.
    InvalidOperand { function: String, detail: String },
}

impl fmt::Display for IIRGe225Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationFailed(errs) => {
                write!(f, "validation failed:\n  {}", errs.join("\n  "))
            }
            Self::UnsupportedOp { function, op } => {
                write!(f, "unsupported op in function {function:?}: {op}")
            }
            Self::UnsupportedType { function, type_hint } => {
                write!(f, "unsupported type in function {function:?}: {type_hint}")
            }
            Self::InvalidOperand { function, detail } => {
                write!(f, "invalid operand in function {function:?}: {detail}")
            }
        }
    }
}

impl std::error::Error for IIRGe225Error {}

// ===========================================================================
// validate_for_ge225
// ===========================================================================

/// Pre-flight validation for IIR → GE-225 lowering.
///
/// **v0.1.0 stub**: always returns an empty `Vec` — there are no
/// validation rules yet because no instructions are lowered.  Future
/// versions will add rules as opcodes come online (see
/// `MULTILANG-ARCHITECTURE-BACKENDS.md` §A5).
///
/// Mirrors the shape of the other IIR backends' `validate_for_*` so
/// callers can switch backends without changing their pre-flight
/// logic.
pub fn validate_for_ge225(_module: &IIRModule) -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// lower_iir_to_ge225
// ===========================================================================

/// Lower an [`IIRModule`] to a `Vec<u8>` of GE-225 opcode bytes
/// (20-bit words packed 3 bytes each, big-endian).
///
/// **v0.1.0 scope**: emits the canonical 3-byte HLT sentinel
/// `[0x00, 0x00, 0x00]` regardless of the input.  This is the
/// smallest "valid GE-225 program" we can produce — enough to load
/// into a simulator, step once, and confirm the CPU halts.  Real
/// lowering arrives in v0.2.0+ (A5+).
pub fn lower_iir_to_ge225(
    module: &IIRModule,
    _cfg: &IIRGe225Config,
) -> Result<Vec<u8>, IIRGe225Error> {
    let errors = validate_for_ge225(module);
    if !errors.is_empty() {
        return Err(IIRGe225Error::ValidationFailed(errors));
    }
    Ok(HALT_WORD.to_vec())
}
