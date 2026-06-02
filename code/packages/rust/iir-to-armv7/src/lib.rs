//! # iir-to-armv7 — IIR → ARMv7 (A32) machine code backend (v0.1.0 skeleton).
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `Vec<u32>` of encoded
//! 32-bit ARMv7-A (A32) instructions, suitable to drop into the
//! in-tree `arm-simulator` or to write out as a flat `.bin` for
//! `qemu-arm` / `objcopy`.
//!
//! ## Why an ARMv7 backend?
//!
//! ARMv7 (32-bit ARM, A32 encoding) is the **phone-class target** of
//! the LANG VM architecture-backend lane.  It covers Cortex-A7/A8/A9-
//! era SoCs and many embedded boards (early Raspberry Pi, BeagleBone,
//! Olimex A20-OLinuXino) — vastly more deployed silicon than any
//! single 8008 chip ever shipped, but architecturally a clean
//! fixed-width 32-bit RISC like RV32I.
//!
//! Adding ARMv7 as a backend gives us:
//!
//! 1. **A third architecture backend** alongside RV32I (A1) and
//!    Intel 8008 (A2).  The three sit at meaningfully different
//!    points in the design space:
//!    - RV32I: clean 32-bit RISC, load-store, no condition codes.
//!    - Intel 8008: irregular 8-bit accumulator CISC, 14-bit address
//!      bus — historical fidelity for Oct.
//!    - **ARMv7 (A32)**: 32-bit RISC with a `cond` field on EVERY
//!      instruction plus a barrel shifter on the second operand.
//!      Same word width as RV32I, fundamentally different ISA.
//! 2. **Foundation for native phone-OS targets.**  Once the AOT
//!    wiring lands (A3+++), the same LANG VM source program can
//!    cross-compile to ARMv7 Linux executables.
//! 3. **Round-trip with the in-tree `arm-simulator`.**  The
//!    `Vec<u32>` output drops directly into the simulator for
//!    in-process tests.
//!
//! ## Scope of v0.1.0 (A3)
//!
//! This release is a **skeleton**: any IIR module lowers to a single
//! `BKPT #0xFFFF` instruction (encoding `0xE12FFF7F`).  No instruction
//! selection yet; that arrives in A3+ (`const` + `bx lr`) and beyond.
//!
//! ## Why `Vec<u32>` output, not textual asm?
//!
//! - **Round-trips with `arm-simulator`** — its decoder consumes raw
//!   little-endian 32-bit words.
//! - **Deterministic test surface** — `assert_eq!(words[0], 0xE12FFF7F)`
//!   is unambiguous; ARM assembler syntax has GNU `as`, LLVM `clang`,
//!   and ARMASM divergence we don't want to entangle with.
//! - **Trivial encoding shape** — every A32 instruction is exactly 4
//!   bytes (in stark contrast to the 8008's 1/2/3 byte variability).
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::IIRModule;
//! use iir_to_armv7::{validate_for_armv7, lower_iir_to_armv7, IIRArmv7Config};
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
//! assert!(validate_for_armv7(&module).is_empty());
//!
//! let words = lower_iir_to_armv7(&module, &IIRArmv7Config::default())
//!     .expect("lowering should succeed");
//! // 0xE12FFF7F == ARMv7-A BKPT #0xFFFF.
//! assert_eq!(words, vec![0xE12F_FF7F]);
//! ```
//!
//! ## Pipeline position
//!
//! ```text
//! IIRModule
//!   → validate_for_armv7()      pre-flight, returns Vec<String>
//!   → lower_iir_to_armv7()      returns Vec<u32> of A32 words
//!   → (optional)
//!       • arm-simulator: in-process testing
//!       • write to .bin + qemu-arm
//!       • objcopy + linker for an ELF on a phone-class Linux board
//! ```

use interpreter_ir::IIRModule;
use std::fmt;

// ===========================================================================
// ARMv7 (A32) opcode constants
// ===========================================================================
//
// Every A32 instruction is exactly 4 bytes, with a fixed encoding
// template `cond IIII OOOO ... ` where `cond` is the conditional-
// execution prefix (the 4-bit field bits 31..28 every A32 instruction
// carries) and `IIII OOOO` selects the instruction family.  Unlike
// RV32I and the 8008, ARMv7 has no "unconditional" sub-encoding for
// most ops — `cond = 0b1110 = 0xE` is the "always-execute" value used
// everywhere a conditional prefix isn't actively wanted.

/// ARMv7-A `BKPT #0xFFFF` opcode — `0xE12FFF7F`.  Triggers a
/// breakpoint exception; semantically "stop execution".
///
/// Bit layout (cond=AL):
///
/// ```text
/// 31..28  cond    = 0xE = 1110            (always — unconditional)
/// 27..20          = 0001 0010 = 0x12      (BKPT opcode family)
/// 19.. 8  imm12   = 0xFFF                 (top 12 bits of imm16)
///  7.. 4          = 0111 = 0x7            (BKPT opcode family)
///  3.. 0  imm4    = 0xF                   (bottom 4 bits of imm16)
/// ```
///
/// Concatenated: `1110 0001 0010 1111_1111_1111 0111 1111` =
/// `0xE12FFF7F`.
///
/// ## Why BKPT and not WFI or `b .`?
///
/// | Candidate | Pros | Cons |
/// |-----------|------|------|
/// | `BKPT #imm16` | Semantically "stop"; every ARM debugger / emulator recognises it | None for skeleton purposes |
/// | `WFI`         | True halt | Requires kernel/hypervisor privilege; illegal in userspace |
/// | `B .`         | Pure userspace, no traps | Burns CPU; harder to detect without a host timeout |
///
/// BKPT wins on simplicity + emulator round-trip.  The
/// `arm-simulator`'s decoder flags it as `bkpt` and stops single-
/// stepping.
pub const BKPT: u32 = 0xE12F_FF7F;

// ===========================================================================
// IIRArmv7Config
// ===========================================================================

/// Configuration for the IIR → ARMv7 lowering pass.
///
/// Currently only the module name is configurable, reserved for future
/// symbol-table / ELF-section emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRArmv7Config {
    /// Module name — reserved for future symbol-table / `.bin` header use.
    pub module_name: String,
}

impl IIRArmv7Config {
    /// Build a config with a custom module name.
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }
}

impl Default for IIRArmv7Config {
    fn default() -> Self {
        Self {
            module_name: "iir_module".into(),
        }
    }
}

// ===========================================================================
// IIRArmv7Error
// ===========================================================================

/// Errors that can occur during IIR → ARMv7 lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRArmv7Error {
    /// The module failed pre-flight validation.
    ValidationFailed(Vec<String>),
    /// An IIR opcode not yet supported by this backend.  v0.1.0
    /// doesn't lower any instructions, so a non-empty function body
    /// would surface this in a future version.
    UnsupportedOp { function: String, op: String },
    /// A type hint that does not map to any ARMv7 representation.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape.
    InvalidOperand { function: String, detail: String },
}

impl fmt::Display for IIRArmv7Error {
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

impl std::error::Error for IIRArmv7Error {}

// ===========================================================================
// validate_for_armv7
// ===========================================================================

/// Pre-flight validation for IIR → ARMv7 lowering.
///
/// **v0.1.0 stub**: always returns an empty `Vec` — there are no
/// validation rules yet because no instructions are lowered.  Future
/// versions will add rules as opcodes come online (see
/// `MULTILANG-ARCHITECTURE-BACKENDS.md` §A3).
///
/// Mirrors the shape of the other IIR backends'
/// `validate_for_{wasm,jvm,clr,beam,llvm,riscv,intel8008}` so callers
/// can switch backends without changing their pre-flight logic.
pub fn validate_for_armv7(_module: &IIRModule) -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// lower_iir_to_armv7
// ===========================================================================

/// Lower an [`IIRModule`] to a `Vec<u32>` of ARMv7 (A32) opcode words.
///
/// **v0.1.0 scope**: emits a single `BKPT #0xFFFF` regardless of the
/// input.  This is the smallest "valid A32 program" we can produce —
/// enough to load into the simulator, step once, and confirm the
/// breakpoint exception fires.  Real lowering arrives in v0.2.0+ (A3+).
pub fn lower_iir_to_armv7(
    module: &IIRModule,
    _cfg: &IIRArmv7Config,
) -> Result<Vec<u32>, IIRArmv7Error> {
    let errors = validate_for_armv7(module);
    if !errors.is_empty() {
        return Err(IIRArmv7Error::ValidationFailed(errors));
    }
    Ok(vec![BKPT])
}
