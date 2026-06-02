//! # iir-to-intel4004 — IIR → Intel 4004 machine code backend (v0.1.0 skeleton).
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `Vec<u8>` of encoded
//! 8-bit Intel 4004 opcodes, suitable to drop into any 4004
//! simulator or to write out as a flat `.bin` for burning to a
//! 1702/2708 EPROM and plugging into a 4004 dev board.
//!
//! ## Why an Intel 4004 backend?
//!
//! The Intel 4004 (1971) was the **world's first commercial
//! microprocessor**.  Tiny ISA, 4-bit data, 12-bit ROM addresses,
//! single 4-bit accumulator, 16 4-bit registers organised as 8
//! register pairs, tiny ROM (4 KiB max) and RAM (640 bits max).
//!
//! In this codebase the 4004 is primarily a **Brainfuck fit** —
//! BF's minimal needs (single tape pointer, ±1 increment ops,
//! conditional jump on zero) actually do map cleanly to a 4004's
//! accumulator-and-loop programming model.
//!
//! Adding the 4004 gives us:
//!
//! 1. **Historical fidelity.**  The 4004 is where the entire
//!    modern-microprocessor lineage began.
//! 2. **A fourth architecture backend** alongside RV32I (A1),
//!    Intel 8008 (A2), and ARMv7 (A3).  The most constrained
//!    target in the lane by a wide margin.
//! 3. **Stress-tests the IIR's neutrality.**  If a 4-bit-data,
//!    4 KiB-ROM target can swallow the IIR's 64-bit `Operand::Int`
//!    shape (via truncation + range-rejection), every other
//!    backend can too.
//!
//! ## Scope of v0.1.0 (A4)
//!
//! This release is a **skeleton**: any IIR module lowers to a
//! single `JUN 0x000` (jump-unconditional to address 0) infinite-
//! loop halt sentinel, encoded as the two bytes `0x40 0x00`.  No
//! instruction selection yet; that arrives in A4+ and beyond.
//!
//! ## Why `Vec<u8>` output, not textual asm?
//!
//! - **Round-trips with the in-tree intel-4004-assembler and any
//!   4004 simulator** — both consume raw byte streams.
//! - **Deterministic test surface** — 4004 mnemonics have multiple
//!   historical spellings (Intel MCS-4 manual vs modern reverse-
//!   engineered docs).  Bytes are unambiguous.
//! - **Trivial output size** — 4004 instructions are 1 or 2
//!   bytes; emitting bytes directly skips a textual-assembly
//!   round-trip.
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::IIRModule;
//! use iir_to_intel4004::{validate_for_intel4004, lower_iir_to_intel4004, IIRIntel4004Config};
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
//! assert!(validate_for_intel4004(&module).is_empty());
//!
//! let bytes = lower_iir_to_intel4004(&module, &IIRIntel4004Config::default())
//!     .expect("lowering should succeed");
//! // JUN 0x000 = 0x40 0x00 = the canonical 4004-ROM halt idiom.
//! assert_eq!(bytes, vec![0x40, 0x00]);
//! ```

use interpreter_ir::IIRModule;
use std::fmt;

// ===========================================================================
// Intel 4004 opcode constants
// ===========================================================================
//
// The 4004 has no formal HLT.  The canonical "halt" idiom is `JUN
// 0x000` — an unconditional jump back to ROM address 0, which
// (when itself at address 0) loops forever, simulating halt.

/// Canonical "halt" sentinel for the Intel 4004 — two bytes:
/// `0x40 0x00` (= `JUN 0x000`, jump-unconditional to ROM address 0).
///
/// Bit layout:
///
/// ```text
/// byte 1: 0100 0000 = 0x40   (JUN opcode + high nibble of 12-bit addr = 0)
/// byte 2: 0000 0000 = 0x00   (low byte of address = 0)
/// ```
///
/// JUN's encoding is `0100 aaaa aaaaaaaa` — the high 4 bits of
/// byte 1 are the JUN opcode (`0100`), the low 4 bits hold the
/// high 4 bits of the 12-bit address, and byte 2 holds the low 8
/// bits.
///
/// When emitted at ROM address 0 (where lowering starts), this
/// instruction's address is 0 and its target is also 0, so the CPU
/// infinitely re-executes it.  Every 4004 simulator and any
/// oscilloscope hooked to the program counter recognises this
/// pattern as "the chip is stuck at address 0".
///
/// ## Why JUN-self over NOP-cycle or unimplemented-opcode?
///
/// | Candidate | Pros | Cons |
/// |-----------|------|------|
/// | `JUN 0x000` (this) | Self-documenting; portable across all 4004 implementations | None for skeleton purposes |
/// | `NOP NOP NOP ...` (0x00 cycle) | Even simpler bytes | Doesn't halt — keeps running into whatever follows |
/// | Unimplemented opcode | Forces a trap | 4004 silicon executes most "unused" bit patterns as NOPs; not portable |
///
/// `JUN 0x000` wins on portability + clarity.
pub const HALT_LOOP: [u8; 2] = [0x40, 0x00];

// ===========================================================================
// IIRIntel4004Config
// ===========================================================================

/// Configuration for the IIR → Intel 4004 lowering pass.
///
/// Currently only the module name is configurable, reserved for
/// future symbol-table / ROM-image emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRIntel4004Config {
    /// Module name — reserved for future symbol-table / `.bin`
    /// header use.
    pub module_name: String,
}

impl IIRIntel4004Config {
    /// Build a config with a custom module name.
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }
}

impl Default for IIRIntel4004Config {
    fn default() -> Self {
        Self {
            module_name: "iir_module".into(),
        }
    }
}

// ===========================================================================
// IIRIntel4004Error
// ===========================================================================

/// Errors that can occur during IIR → Intel 4004 lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRIntel4004Error {
    /// The module failed pre-flight validation.
    ValidationFailed(Vec<String>),
    /// An IIR opcode not yet supported by this backend.  v0.1.0
    /// doesn't lower any instructions, so a non-empty function
    /// body would surface this in a future version.
    UnsupportedOp { function: String, op: String },
    /// A type hint that does not map to any Intel 4004
    /// representation.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape.
    InvalidOperand { function: String, detail: String },
}

impl fmt::Display for IIRIntel4004Error {
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

impl std::error::Error for IIRIntel4004Error {}

// ===========================================================================
// validate_for_intel4004
// ===========================================================================

/// Pre-flight validation for IIR → Intel 4004 lowering.
///
/// **v0.1.0 stub**: always returns an empty `Vec` — there are no
/// validation rules yet because no instructions are lowered.
/// Future versions will add rules as opcodes come online (see
/// `MULTILANG-ARCHITECTURE-BACKENDS.md` §A4).
///
/// Mirrors the shape of the other IIR backends'
/// `validate_for_{wasm,jvm,clr,beam,llvm,riscv,intel8008,armv7}`
/// so callers can switch backends without changing their pre-
/// flight logic.
pub fn validate_for_intel4004(_module: &IIRModule) -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// lower_iir_to_intel4004
// ===========================================================================

/// Lower an [`IIRModule`] to a `Vec<u8>` of Intel 4004 opcode bytes.
///
/// **v0.1.0 scope**: emits the canonical 2-byte halt sentinel `JUN
/// 0x000` (`0x40 0x00`) regardless of the input.  This is the
/// smallest "valid Intel 4004 program" we can produce — enough to
/// load into a simulator, step once, and confirm the CPU is stuck
/// at address 0.  Real lowering arrives in v0.2.0+ (A4+).
pub fn lower_iir_to_intel4004(
    module: &IIRModule,
    _cfg: &IIRIntel4004Config,
) -> Result<Vec<u8>, IIRIntel4004Error> {
    let errors = validate_for_intel4004(module);
    if !errors.is_empty() {
        return Err(IIRIntel4004Error::ValidationFailed(errors));
    }
    Ok(HALT_LOOP.to_vec())
}
