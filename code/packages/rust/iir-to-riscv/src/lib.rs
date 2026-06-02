//! # iir-to-riscv — IIR → RV32I machine code backend (v0.1.0 skeleton).
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `Vec<u32>` of encoded
//! 32-bit RISC-V instructions, suitable to drop into the in-tree
//! [`riscv-simulator`] or to write out as a flat `.bin` for `qemu-riscv32`.
//!
//! ## Why an architecture backend?
//!
//! The wasm / JVM / CLR / BEAM / LLVM backends all target *software*
//! runtimes that own register allocation and instruction selection.
//! RISC-V is the first **architecture** backend — output is real hardware
//! ISA, decoded directly by the bundled `riscv-simulator` (RV32I + M-mode
//! traps) or by QEMU / a physical SiFive / Espressif RISC-V chip.
//!
//! Strategic priority among architecture backends: RISC-V is the most
//! open of the candidates (royalty-free spec, broad simulator
//! availability, growing hardware footprint).  Once it works, A2-A5
//! (Intel 8008, ARMv7, Intel 4004, GE-225) follow the same shape.
//!
//! ## Why a `Vec<u32>` output (not textual asm)?
//!
//! - **Round-trips with the simulator.**  `riscv-simulator` decodes raw
//!   32-bit words; emitting them directly skips the assembler step.
//! - **Deterministic test surface.**  `assert!(words[0] == 0x00008067)`
//!   for `ret` reads cleanly in test failures.
//! - **No textual-format coupling.**  GNU / LLVM assembly syntax diverge
//!   on edge cases; raw words avoid both.
//!
//! A textual `.s` emitter could be added as a sibling later without
//! breaking callers.
//!
//! ## Scope of v0.1.0 (A1)
//!
//! This release is a **skeleton**: any IIR module lowers to a single
//! `ret` (encoded as `jalr x0, x1, 0`, which is the standard RV32I
//! return-from-function pseudo-instruction).  No instruction selection
//! yet; that arrives in A1+ (arith / cmp / control flow), A1++ (calls +
//! locals + ecall print), and A1+++ (lang-aot `--target=riscv32`
//! wiring).
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::IIRModule;
//! use iir_to_riscv::{validate_for_riscv, lower_iir_to_riscv, IIRRiscvConfig};
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
//! assert!(validate_for_riscv(&module).is_empty());
//!
//! let words = lower_iir_to_riscv(&module, &IIRRiscvConfig::default())
//!     .expect("lowering should succeed");
//! // 0x00008067 == `jalr x0, x1, 0` == RV32I `ret`.
//! assert_eq!(words, vec![0x0000_8067]);
//! ```
//!
//! ## Pipeline position
//!
//! ```text
//! IIRModule
//!   → validate_for_riscv()       pre-flight, returns Vec<String>
//!   → lower_iir_to_riscv()       returns Vec<u32> of RV32I words
//!   → (optional) riscv-simulator::execute() OR write to .bin OR link with `ld`
//! ```

use interpreter_ir::IIRModule;
use std::fmt;

use riscv_simulator::encoding::encode_jalr;

// ===========================================================================
// Register names — symbolic constants for x0, x1 to keep the call sites
// readable without depending on a wider register-file abstraction.
// ===========================================================================
//
// RV32I has 32 integer registers x0..x31.  The ABI assigns roles:
//   x0  — hardwired zero
//   x1  — return address (`ra`)
//   x2  — stack pointer (`sp`)
//   x10 — first argument / first return value (`a0`)
//   ...
//
// v0.1.0 only needs `x0` (rd of the trap-less `jalr`) and `x1` (rs1
// holding the return address).  Future versions will expand this.
const X0_ZERO: u32 = 0;
const X1_RA:   u32 = 1;

// ===========================================================================
// IIRRiscvConfig
// ===========================================================================

/// Configuration for the IIR → RV32I lowering pass.
///
/// Currently only the assembly module name is configurable — it's stored
/// for future symbol-table emission but has no effect on v0.1.0's
/// minimal output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRRiscvConfig {
    /// Module name — reserved for future ELF / linker artefact emission.
    pub module_name: String,
}

impl IIRRiscvConfig {
    /// Build a config with a custom module name.
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }
}

impl Default for IIRRiscvConfig {
    fn default() -> Self {
        Self {
            module_name: "iir_module".into(),
        }
    }
}

// ===========================================================================
// IIRRiscvError
// ===========================================================================

/// Errors that can occur during IIR → RV32I lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRRiscvError {
    /// The module failed pre-flight validation.
    ValidationFailed(Vec<String>),
    /// An IIR opcode not yet supported by this backend.  v0.1.0 doesn't
    /// lower any instructions, so a non-empty function body returns this.
    UnsupportedOp { function: String, op: String },
    /// A type hint that does not map to any RV32I representation.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape.
    InvalidOperand { function: String, detail: String },
}

impl fmt::Display for IIRRiscvError {
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

impl std::error::Error for IIRRiscvError {}

// ===========================================================================
// validate_for_riscv
// ===========================================================================

/// Pre-flight validation for IIR → RV32I lowering.
///
/// **v0.1.0 stub**: always returns an empty `Vec` — there are no
/// validation rules yet because no instructions are lowered.  Future
/// versions will add rules as opcodes come online (see
/// `MULTILANG-ARCHITECTURE-BACKENDS.md` §A1).
///
/// Mirrors the shape of the other IIR backends'
/// `validate_for_{wasm,jvm,clr,beam,llvm}` so callers can switch
/// backends without changing their pre-flight logic.
pub fn validate_for_riscv(_module: &IIRModule) -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// lower_iir_to_riscv
// ===========================================================================

/// Lower an [`IIRModule`] to a `Vec<u32>` of encoded RV32I instructions.
///
/// **v0.1.0 scope**: emits a single `ret` regardless of the input.
/// This is the smallest "valid RISC-V" output we can produce — enough
/// to load into [`riscv_simulator`], single-step, and confirm the
/// encoding round-trips.  Real lowering arrives in v0.2.0+ (A1+).
///
/// # The encoded word
///
/// `ret` is the standard RV32I assembly pseudo-instruction for
/// "return from function".  It encodes as `jalr x0, x1, 0`:
///
/// - `rd  = x0` — discard the next PC (we don't care where ret was called
///   from for future call chains; the simulator already tracks call
///   depth).
/// - `rs1 = x1 (ra)` — jump to the return address held in the standard
///   ABI register `ra`.
/// - `imm = 0` — no offset.
///
/// The bit pattern is `0x0000_8067`.  Verify in Volume I §2.5 of the
/// RISC-V User-Level ISA spec (page 30 in the 2019-12-13 ratified
/// edition).  We assert this in tests rather than trust the encoding
/// blindly.
pub fn lower_iir_to_riscv(
    module: &IIRModule,
    _cfg: &IIRRiscvConfig,
) -> Result<Vec<u32>, IIRRiscvError> {
    // Even though the v0.1.0 validator is a stub, we run it
    // unconditionally so the contract is established: callers can rely
    // on `lower_iir_to_riscv` returning `ValidationFailed` for any rule
    // the validator catches.  Wiring it now means later versions can
    // add rules without changing the API.
    let errors = validate_for_riscv(module);
    if !errors.is_empty() {
        return Err(IIRRiscvError::ValidationFailed(errors));
    }

    // The single emitted instruction.  Computed via the encoder rather
    // than hardcoded as a literal so any future revision of
    // `encode_jalr` (e.g. typo fix) flows through automatically.
    let ret = encode_jalr(X0_ZERO, X1_RA, 0);
    Ok(vec![ret])
}
