//! # iir-to-intel8008 — IIR → Intel 8008 machine code backend (v0.1.0 skeleton).
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `Vec<u8>` of encoded
//! 8-bit Intel 8008 opcodes, suitable to drop into the in-tree
//! [`intel8008-simulator`] or to write out as a flat `.bin` for an
//! external emulator.
//!
//! ## Why an Intel 8008 backend?
//!
//! The Intel 8008 (1972) is the first commercial 8-bit microprocessor.
//! In this codebase it's **Oct's native target** — the Oct front-end
//! produces IIR specifically intended to round-trip through an 8008.
//!
//! Adding the 8008 as a backend gives us:
//!
//! 1. **Historical fidelity.**  Oct programs can run on the actual ISA
//!    they were designed for.
//! 2. **A second architecture backend** alongside RISC-V (A1).  The two
//!    architectures sit at opposite ends of the design space: RV32I is
//!    a clean modern load-store RISC; the 8008 is an irregular
//!    accumulator-based CISC with 8-bit registers and 14-bit
//!    addressing.  Each exposes a different set of constraints in the
//!    backend interface.
//! 3. **A foundation for A4 (Intel 4004)** — the 4004 is even more
//!    constrained, but shares much of the historical-microprocessor
//!    backend shape that A2 establishes.
//!
//! ## Scope of v0.1.0 (A2)
//!
//! This release is a **skeleton**: any IIR module lowers to a single
//! `HLT` instruction (opcode `0x76`).  No instruction selection yet;
//! that arrives in A2+ (basic arithmetic / MOV / register allocation)
//! and beyond.
//!
//! ## Why `Vec<u8>` output, not textual asm?
//!
//! - **Round-trips with `intel8008-simulator`** — its
//!   [`Simulator::run`] method consumes raw `&[u8]` instruction streams.
//! - **Deterministic test surface** — `assert_eq!(bytes, vec![0x76])`
//!   is unambiguous; assembler syntax for the 8008 has Intel-mnemonics
//!   vs MCS-8 historical-mnemonics divergence.
//! - **Trivial output size** — Intel 8008 instructions are 1, 2, or 3
//!   bytes; emitting bytes directly skips a textual-assembly round-trip
//!   that contributes nothing.
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::IIRModule;
//! use iir_to_intel8008::{validate_for_intel8008, lower_iir_to_intel8008, IIRIntel8008Config};
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
//! assert!(validate_for_intel8008(&module).is_empty());
//!
//! let bytes = lower_iir_to_intel8008(&module, &IIRIntel8008Config::default())
//!     .expect("lowering should succeed");
//! // 0x76 == Intel 8008 HLT.
//! assert_eq!(bytes, vec![0x76]);
//! ```
//!
//! ## Pipeline position
//!
//! ```text
//! IIRModule
//!   → validate_for_intel8008()    pre-flight, returns Vec<String>
//!   → lower_iir_to_intel8008()    returns Vec<u8> of 8008 opcodes
//!   → (optional)
//!       • intel8008_simulator::Simulator::run for in-process testing
//!       • write to .bin + external emulator
//!       • burn to a 1702 EPROM (Oct's intended deployment path)
//! ```

use interpreter_ir::IIRModule;
use std::fmt;

// ===========================================================================
// Intel 8008 opcode constants used by v0.1.0
// ===========================================================================
//
// The 8008's instruction encoding is irregular by modern standards: the
// top 2 bits group instructions into four families (immediate-byte ops,
// register-register MOV, ALU on register, and condition/jump/call), and
// the lower 6 bits select within the family.  HLT lives in the
// register-register MOV family but encodes "MOV M,M" — semantically a
// self-move on the memory-pointer pseudo-register — which the silicon
// implements as a halt.
//
// Bit pattern of HLT: `01 110 110` = `0x76`.  The simulator's
// `Simulator::halted()` accessor flips true after this byte is
// executed.

/// Intel 8008 `HLT` opcode — `0x76`.  Halts the CPU.
pub const HLT: u8 = 0x76;

// ===========================================================================
// IIRIntel8008Config
// ===========================================================================

/// Configuration for the IIR → Intel 8008 lowering pass.
///
/// Currently only the module name is configurable, reserved for future
/// symbol-table / ROM-image emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRIntel8008Config {
    /// Module name — reserved for future symbol-table / `.bin` header use.
    pub module_name: String,
}

impl IIRIntel8008Config {
    /// Build a config with a custom module name.
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }
}

impl Default for IIRIntel8008Config {
    fn default() -> Self {
        Self {
            module_name: "iir_module".into(),
        }
    }
}

// ===========================================================================
// IIRIntel8008Error
// ===========================================================================

/// Errors that can occur during IIR → Intel 8008 lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRIntel8008Error {
    /// The module failed pre-flight validation.
    ValidationFailed(Vec<String>),
    /// An IIR opcode not yet supported by this backend.  v0.1.0 doesn't
    /// lower any instructions, so a non-empty function body would
    /// surface this in a future version.
    UnsupportedOp { function: String, op: String },
    /// A type hint that does not map to any Intel 8008 representation.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape.
    InvalidOperand { function: String, detail: String },
}

impl fmt::Display for IIRIntel8008Error {
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

impl std::error::Error for IIRIntel8008Error {}

// ===========================================================================
// validate_for_intel8008
// ===========================================================================

/// Pre-flight validation for IIR → Intel 8008 lowering.
///
/// **v0.1.0 stub**: always returns an empty `Vec` — there are no
/// validation rules yet because no instructions are lowered.  Future
/// versions will add rules as opcodes come online (see
/// `MULTILANG-ARCHITECTURE-BACKENDS.md` §A2).
///
/// Mirrors the shape of the other IIR backends'
/// `validate_for_{wasm,jvm,clr,beam,llvm,riscv}` so callers can switch
/// backends without changing their pre-flight logic.
pub fn validate_for_intel8008(_module: &IIRModule) -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// lower_iir_to_intel8008
// ===========================================================================

/// Lower an [`IIRModule`] to a `Vec<u8>` of Intel 8008 opcode bytes.
///
/// **v0.1.0 scope**: emits a single `HLT` regardless of the input.
/// This is the smallest "valid Intel 8008 program" we can produce —
/// enough to load into the simulator, step once, and confirm
/// [`Simulator::halted`] returns `true`.  Real lowering arrives in
/// v0.2.0+ (A2+).
pub fn lower_iir_to_intel8008(
    module: &IIRModule,
    _cfg: &IIRIntel8008Config,
) -> Result<Vec<u8>, IIRIntel8008Error> {
    let errors = validate_for_intel8008(module);
    if !errors.is_empty() {
        return Err(IIRIntel8008Error::ValidationFailed(errors));
    }
    // Single emitted byte: HLT.  Wrapped through the named constant so a
    // future revision of the simulator's opcode table that changes the
    // halt encoding flows through automatically — we'd update `HLT` once
    // and every backend re-emits the right byte.
    Ok(vec![HLT])
}
