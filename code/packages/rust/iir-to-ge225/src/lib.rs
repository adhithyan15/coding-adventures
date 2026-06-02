//! # iir-to-ge225 — IIR → GE-225 machine code backend (v0.2.0, A5+).
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
//! MULTILANG-ARCHITECTURE-BACKENDS.md §A5.
//!
//! ## Scope of v0.2.0 (A5+ — first real lowering)
//!
//! | IIR op | GE-225 lowering |
//! |--------|-----------------|
//! | `const dest, Int(n)` (16-bit signed) | `LDA n` (20-bit word, opcode 0x1, 16-bit imm) |
//! | `const dest, Bool(b)` | `LDA 0` or `LDA 1` |
//! | `ret <var>` | `HLT` (20-bit zero word) — real RET needs A5++ |
//! | `ret_void` | `HLT` |
//!
//! ### Accumulator-only first slice
//!
//! Every `const` loads into the accumulator.  The GE-225's
//! arithmetic unit is accumulator-anchored, so this is the natural
//! starting point.  Multi-register allocation arrives in A5++
//! alongside arithmetic; for now, the most recent `const` overwrites
//! whatever ACC held before.  `ret <var>` requires `var` to be the
//! current ACC owner — otherwise the value isn't reachable and we
//! return `UndefinedVariable`.
//!
//! ### Why `ret` → HLT for now?
//!
//! A real return on the GE-225 would unwind via the SBR (Save
//! Branch Register) discipline that JSR (Jump SubRoutine) sets up.
//! Without proper call/return support (which lands in A5++), we
//! cannot synthesise a meaningful `BR <saved>` — there's no saved
//! address.  Emitting `HLT` (the all-zeros word) gives every
//! GE-225 simulator a clean, deterministic stopping point in the
//! meantime.
//!
//! ### Empty-module contract
//!
//! Preserves v0.1.0's behaviour for the canonical "fn main() {}"
//! minimal case: any module with no functions emits the bare
//! `HALT_WORD` so the simulator halts deterministically.  Once at
//! least one function is lowered, the halt sentinel terminates the
//! last function's instruction stream.
//!
//! ## Word packing (recap from v0.1.0)
//!
//! Each 20-bit GE-225 word → 3 bytes (24 bits), big-endian, with
//! the top 4 bits of byte 0 always zero (since 20 bits < 24 bits):
//!
//! ```text
//! byte 0: 0000 OOOO   (top 4 bits zero + 4-bit opcode nibble)
//! byte 1: IIII IIII   (high 8 bits of the 16-bit immediate / address)
//! byte 2: IIII IIII   (low  8 bits of the 16-bit immediate / address)
//! ```
//!
//! v0.2.0 uses two opcode nibbles:
//!
//! * `0x0` — `HLT` (all-zeros word).
//! * `0x1` — `LDA n` (load accumulator with 16-bit signed immediate).
//!
//! Future slices add `0x2` (`ADD`), `0x3` (`SUB`), `0x4` (`BR`), etc.
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
//! use iir_to_ge225::{validate_for_ge225, lower_iir_to_ge225, IIRGe225Config};
//!
//! // const v=5; ret v
//! let f = IIRFunction::new("five", vec![], "i16", vec![
//!     IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "i16"),
//!     IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i16"),
//! ]);
//! let module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![f],
//!     entry_point: Some("five".into()),
//!     language: "demo".into(),
//!     exports: vec![],
//!     imports: vec![],
//! };
//!
//! assert!(validate_for_ge225(&module).is_empty());
//!
//! let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
//!     .expect("lowering should succeed");
//! // LDA 5 = [0x01, 0x00, 0x05]  ;  HLT = [0x00, 0x00, 0x00]
//! assert_eq!(bytes, vec![0x01, 0x00, 0x05, 0x00, 0x00, 0x00]);
//! ```

use interpreter_ir::{IIRModule, Operand};
use std::collections::HashMap;
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
/// `0x00`.
pub const HALT_WORD: [u8; 3] = [0x00, 0x00, 0x00];

/// GE-225 `LDA` opcode nibble — `0x1`.  Load accumulator with a
/// 16-bit signed immediate.
///
/// The opcode occupies the low 4 bits of byte 0 (= bits 19..16 of
/// the 20-bit word).  The 16-bit immediate fills bytes 1 + 2,
/// big-endian.
///
/// Example: `LDA 5`:
/// ```text
/// byte 0: 0000 0001 = 0x01   (top 4 bits zero + LDA opcode nibble)
/// byte 1: 0000 0000 = 0x00   (high byte of 16-bit imm = 0)
/// byte 2: 0000 0101 = 0x05   (low  byte of 16-bit imm = 5)
/// ```
///
/// The GE-225's accumulator is 20 bits wide on real silicon, but
/// our `LDA n` immediate is restricted to 16 bits ([-32768, 32767]
/// signed, or [0, 65535] unsigned, both interpreted via two's
/// complement in the low 16 bits of the word).  Wider values must
/// be built up via `LDA hi; ROTL 16; ADD lo` patterns in A5++.
pub const LDA_OPCODE_NIBBLE: u8 = 0x1;

/// Sentinel `env` value meaning "this var currently lives in the
/// accumulator (ACC)".  Distinct from any register index that
/// future slices will use.
const ACC_MARKER: u8 = 16;

/// Supported instruction opcodes in v0.2.0 (A5+).
///
/// * `const dest, Int(n)` / `const dest, Bool(b)` lowers to a
///   single `LDA n` word with `dest` taking over the accumulator.
/// * `ret <var>` requires `var` to be the current ACC owner; emits
///   `HLT` (real RET via SBR discipline lands in A5++).
/// * `ret_void` just emits `HLT`.
const SUPPORTED_OPS: &[&str] = &["const", "ret", "ret_void"];

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
    /// An IIR opcode not yet supported by this backend.
    UnsupportedOp { function: String, op: String },
    /// A type hint that does not map to any GE-225 representation.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape, or an `Int` immediate
    /// falls outside the 16-bit signed range `[-32768, 32767]`.
    InvalidOperand { function: String, detail: String },
    /// A variable was returned (via `ret`) without ever being bound
    /// by a `const`, or it's no longer the current accumulator
    /// owner.  v0.2.0 only tracks ACC; multi-register liveness
    /// arrives in A5++.
    UndefinedVariable { function: String, name: String },
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
            Self::UndefinedVariable { function, name } => {
                write!(
                    f,
                    "variable {name:?} is not in the GE-225 accumulator in \
                     function {function:?} (v0.2.0 tracks only ACC; \
                     multi-register liveness arrives in A5++)"
                )
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
/// **v0.2.0 stub**: always returns an empty `Vec` — per-instruction
/// validation happens during `lower_iir_to_ge225` itself.  Future
/// versions may move structural checks here.
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
/// **v0.2.0 scope (A5+)**: see the module-level docs for the per-op
/// lowering table.
pub fn lower_iir_to_ge225(
    module: &IIRModule,
    _cfg: &IIRGe225Config,
) -> Result<Vec<u8>, IIRGe225Error> {
    let errors = validate_for_ge225(module);
    if !errors.is_empty() {
        return Err(IIRGe225Error::ValidationFailed(errors));
    }

    // Trivial empty-module contract — preserves v0.1.0 callable
    // behaviour for the canonical "fn main() {}" minimal case.
    if module.functions.is_empty() {
        return Ok(HALT_WORD.to_vec());
    }

    let mut bytes = Vec::new();
    for f in &module.functions {
        // ── Per-function accumulator state ────────────────────────────
        //
        // The GE-225's ALU is accumulator-anchored: arithmetic and
        // immediate loads all flow through the 20-bit ACC.  We
        // maintain two pieces of state:
        //
        //   env: HashMap<String, u8>
        //     var name → location.  In v0.2.0 every entry is
        //     `ACC_MARKER` (= 16) since we don't yet allocate GP
        //     registers.  Future slices will use 0..15 for real
        //     registers, mirroring the iir-to-intel4004 v0.3.0
        //     ACC-first allocator.
        //
        //   acc_owner: Option<String>
        //     Which var (if any) currently owns ACC.  Each new
        //     `const` clobbers ACC, becoming the new owner.  `ret`
        //     requires its src to be the current owner — otherwise
        //     the value is unreachable in v0.2.0 (it's been
        //     overwritten by a later const).
        let mut env: HashMap<String, u8> = HashMap::new();
        let mut acc_owner: Option<String> = None;

        for instr in &f.instructions {
            if !SUPPORTED_OPS.contains(&instr.op.as_str()) {
                return Err(IIRGe225Error::UnsupportedOp {
                    function: f.name.clone(),
                    op: instr.op.clone(),
                });
            }
            match instr.op.as_str() {
                // ── const dest, Int(n) → LDA n ────────────────────────
                //
                // 20-bit word layout: bits 19..16 = LDA opcode (0x1),
                // bits 15..0 = the 16-bit two's-complement immediate.
                // Packed big-endian into 3 bytes per v0.1.0's
                // convention.
                "const" => {
                    let dest = require_dest(instr, "const", &f.name)?;
                    let imm16 = encode_immediate_16(instr.srcs.first(), &f.name)?;
                    bytes.extend_from_slice(&encode_lda(imm16));
                    env.insert(dest.to_string(), ACC_MARKER);
                    acc_owner = Some(dest.to_string());
                }

                // ── ret <var>: require var == acc_owner; emit HLT ─────
                //
                // v0.2.0 has no LD register-source instruction yet;
                // every value lives in ACC and gets overwritten by
                // the next `const`.  So `ret v` requires v to still
                // be the current ACC owner.  This is restrictive but
                // correct — the alternative (silently producing wrong
                // code) is worse.  A5++ adds register allocation and
                // lifts this restriction.
                "ret" => {
                    let src_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => {
                            return Err(IIRGe225Error::InvalidOperand {
                                function: f.name.clone(),
                                detail: "ret srcs[0] must be Var".into(),
                            })
                        }
                    };
                    let _ = env.get(&src_name).ok_or_else(|| {
                        IIRGe225Error::UndefinedVariable {
                            function: f.name.clone(),
                            name: src_name.clone(),
                        }
                    })?;
                    if acc_owner.as_deref() != Some(src_name.as_str()) {
                        return Err(IIRGe225Error::UndefinedVariable {
                            function: f.name.clone(),
                            name: src_name,
                        });
                    }
                    bytes.extend_from_slice(&HALT_WORD);
                }

                // ── ret_void: just HLT ────────────────────────────────
                "ret_void" => {
                    bytes.extend_from_slice(&HALT_WORD);
                }

                _ => unreachable!("SUPPORTED_OPS guard above prevents this"),
            }
        }
    }

    // Defensive — if every function was empty, fall back to
    // HALT_WORD so the output remains a valid halting program.
    if bytes.is_empty() {
        bytes.extend_from_slice(&HALT_WORD);
    }

    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Per-instruction helpers
// ---------------------------------------------------------------------------

fn require_dest<'a>(
    instr: &'a interpreter_ir::IIRInstr,
    op: &str,
    fn_name: &str,
) -> Result<&'a str, IIRGe225Error> {
    instr
        .dest
        .as_deref()
        .ok_or_else(|| IIRGe225Error::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!("{op} requires a dest"),
        })
}

/// Encode an `LDA n` 20-bit word as 3 bytes, big-endian.
///
/// Layout:
/// ```text
/// byte 0: 0000 0001  (top 4 bits zero + LDA opcode nibble 0x1)
/// byte 1: high 8 bits of the 16-bit immediate
/// byte 2: low  8 bits of the 16-bit immediate
/// ```
fn encode_lda(imm16: u16) -> [u8; 3] {
    [
        LDA_OPCODE_NIBBLE,
        ((imm16 >> 8) & 0xFF) as u8,
        (imm16 & 0xFF) as u8,
    ]
}

/// Decode and range-check a `const` immediate operand into a 16-bit
/// value (two's-complement reinterpretation for negatives).
///
/// * `Int(n)` with `n` in `[-32768, 32767]` → `n as i16 as u16`.
/// * `Bool(true)` → 1, `Bool(false)` → 0.
/// * Out-of-range or non-numeric → `InvalidOperand`.
///
/// The 16-bit ceiling reflects the v0.2.0 instruction format (4-bit
/// opcode + 16-bit immediate fills the 20-bit word).  Future slices
/// can synthesise wider values via `LDA hi; SHL 16; ADD lo` chains.
fn encode_immediate_16(op: Option<&Operand>, fn_name: &str) -> Result<u16, IIRGe225Error> {
    let n: i64 = match op {
        Some(Operand::Int(n)) => *n,
        Some(Operand::Bool(b)) => {
            if *b {
                1
            } else {
                0
            }
        }
        _ => {
            return Err(IIRGe225Error::InvalidOperand {
                function: fn_name.to_string(),
                detail: "const srcs[0] must be Int or Bool".into(),
            })
        }
    };
    // Accept both signed-16 and unsigned-16 ranges; both reinterpret
    // cleanly via two's complement into the same 16 bits.
    if (-32768..=32767).contains(&n) {
        Ok((n as i16) as u16)
    } else if (32768..=65535).contains(&n) {
        Ok(n as u16)
    } else {
        Err(IIRGe225Error::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!(
                "const {n} exceeds 16-bit immediate range ([-32768, 65535]); \
                 the GE-225 v0.2.0 LDA immediate is 16 bits wide — wider \
                 values must be built up via LDA-shift-ADD chains in A5++"
            ),
        })
    }
}
