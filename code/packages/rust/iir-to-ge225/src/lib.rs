//! # iir-to-ge225 — IIR → GE-225 machine code backend (v0.3.0, A5++).
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
//! ## Scope of v0.3.0 (A5++ — ACC-first allocator + `mov`)
//!
//! | IIR op | GE-225 lowering |
//! |--------|-----------------|
//! | `const dest, Int(n)` (16-bit signed/unsigned) | `(STA r_evict)?` + `LDA n` |
//! | `const dest, Bool(b)` | `(STA r_evict)?` + `LDA 0 \| 1` |
//! | `mov dest, src` | `(STA r_evict_src)?` + `LD r_src` + `STA r_dest` |
//! | `ret <var>` | `(LD r_var)?` + `HLT` |
//! | `ret_void` | `HLT` |
//!
//! ### The ACC + r0..r15 register pool — 17 slots total
//!
//! v0.3.0 introduces a GE-225 GP register file: 16 four-bit-indexed
//! registers `r0..r15` plus the 20-bit accumulator.  That's the same
//! 17-slot capacity as the iir-to-intel4004 v0.3.0 pool — chosen for
//! symmetry across the architecture-backend lane.
//!
//! ### Allocator strategy — ACC-first linear
//!
//! 1. The first `const` of a function lands in the accumulator.
//! 2. Each subsequent `const` evicts the current ACC owner to the
//!    next free GP register via `STA r` (which on this skeleton's
//!    GE-225 is exchange-with-ACC — mirroring the 4004's `XCH`).
//! 3. `mov dest, src` first evicts ACC if `src` is the current ACC
//!    owner (so `src` has a stable register home), then loads
//!    `src` into ACC via `LD r_src` and stores it into a fresh
//!    register for `dest` via `STA r_dest`.
//! 4. `ret <var>` loads `<var>` into ACC if it's not already there
//!    (via `LD r_var`), then emits `HLT`.
//!
//! The ACC-first model preserves v0.2.0's 6-byte trivial-case ROM
//! for `const v; ret v` — when there's only one `const`, no
//! eviction happens and the output stays `LDA + HLT = 6 bytes`.
//!
//! ### A note on `STA` semantics on this skeleton's GE-225
//!
//! Real GE-225 silicon's `STA` was a pure store (ACC → memory,
//! ACC retained).  Our skeleton models `STA r` as exchange-with-ACC
//! (`r ↔ ACC`) to mirror the iir-to-intel4004's `XCH` idiom — that
//! lets the eviction pattern be **one instruction** instead of two
//! (`STA r` + `LDA 0` to clear ACC).  Documented here as a
//! deliberate educational simplification; a future v0.4.0+ may
//! split this back into a pure `STA` + restore-via-`LD` pair if
//! historical fidelity becomes a goal.
//!
//! ### Why `ret` → HLT still?
//!
//! Same reason as v0.2.0: a real return needs the SBR (Save Branch
//! Register) discipline that `JSR` (Jump Subroutine) sets up.
//! Without proper call/return support (which lands in A5+++), we
//! emit `HLT` as a clean, deterministic stopping point.
//!
//! ## Word format
//!
//! Each 20-bit GE-225 word → 3 bytes (24 bits), big-endian, with
//! the top 4 bits of byte 0 always zero:
//!
//! ```text
//! byte 0: 0000 OOOO   (top 4 bits zero + 4-bit opcode nibble)
//! byte 1: AAAA AAAA   (high 8 bits of 16-bit immediate / addr field)
//! byte 2: AAAA AAAA   (low  8 bits — for STA/LD, low 4 bits hold reg index)
//! ```
//!
//! Opcodes assigned by v0.3.0:
//!
//! | Nibble | Mnemonic | Effect | Word bytes |
//! |--------|----------|--------|------------|
//! | `0x0` | `HLT`   | halt the machine                       | `[0x00, 0x00, 0x00]` |
//! | `0x1` | `LDA n` | load ACC with 16-bit signed immediate  | `[0x01, hi, lo]` |
//! | `0x2` | `STA r` | exchange ACC with `r` (XCH semantics)  | `[0x02, 0x00, r]` |
//! | `0x3` | `LD r`  | load ACC with the value of `r` (copy)  | `[0x03, 0x00, r]` |
//!
//! Future slices take `0x4..0xF` for `ADD`/`SUB`/`BR`/`JSR`/etc.
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
//! use iir_to_ge225::{validate_for_ge225, lower_iir_to_ge225, IIRGe225Config};
//!
//! // const v=5; ret v  — single const, no eviction; LDA + HLT = 6 bytes.
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
//! let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
//!     .expect("lowering should succeed");
//! assert_eq!(bytes, vec![0x01, 0x00, 0x05, 0x00, 0x00, 0x00]);
//! ```

use interpreter_ir::{IIRModule, Operand};
use std::collections::HashMap;
use std::fmt;

// ===========================================================================
// GE-225 opcode constants
// ===========================================================================

/// Canonical "halt" sentinel for the GE-225 — three bytes:
/// `[0x00, 0x00, 0x00]` (= the all-zeros 20-bit `HLT` word, packed
/// big-endian with the top 4 bits of byte 0 zero).
pub const HALT_WORD: [u8; 3] = [0x00, 0x00, 0x00];

/// GE-225 `LDA` opcode nibble — `0x1`.  Load accumulator with a
/// 16-bit immediate.  Word layout: `[0x01, hi, lo]`.
pub const LDA_OPCODE_NIBBLE: u8 = 0x1;

/// GE-225 `STA` opcode nibble — `0x2`.  In this skeleton, `STA r`
/// exchanges ACC with register `r` (XCH semantics — see module
/// docs for the historical caveat).  Word layout: `[0x02, 0x00, r]`
/// where `r` occupies the low 4 bits of byte 2.
pub const STA_OPCODE_NIBBLE: u8 = 0x2;

/// GE-225 `LD` opcode nibble — `0x3`.  Load ACC with the contents
/// of register `r` (copy — `r` unchanged).  Word layout:
/// `[0x03, 0x00, r]`.
pub const LD_OPCODE_NIBBLE: u8 = 0x3;

/// Sentinel `env` value meaning "this var currently lives in the
/// accumulator (ACC)", distinct from real register indices `0..=15`.
const ACC_MARKER: u8 = 16;

/// Number of GP registers.  Combined with ACC this gives a 17-slot
/// pool — identical to the iir-to-intel4004 v0.3.0 capacity.
const GP_REGISTER_COUNT: usize = 16;

/// Supported instruction opcodes in v0.3.0 (A5++).
const SUPPORTED_OPS: &[&str] = &["const", "mov", "ret", "ret_void"];

// ===========================================================================
// IIRGe225Config
// ===========================================================================

/// Configuration for the IIR → GE-225 lowering pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRGe225Config {
    /// Module name — reserved for future symbol-table / `.bin`
    /// header use.
    pub module_name: String,
}

impl IIRGe225Config {
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
    /// falls outside the 16-bit range.
    InvalidOperand { function: String, detail: String },
    /// A variable was referenced (`mov` / `ret`) without ever being
    /// bound.
    UndefinedVariable { function: String, name: String },
    /// The function tried to bind more locals than the 17-slot
    /// register pool (ACC + r0..r15) can hold.  Memory spilling
    /// lands in a future increment.
    OutOfRegisters { function: String, name: String },
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
                    "undefined variable {name:?} in function {function:?}"
                )
            }
            Self::OutOfRegisters { function, name } => {
                write!(
                    f,
                    "out of GE-225 registers (ACC + r0..r15 = 17 slots) \
                     while binding {name:?} in function {function:?}; \
                     memory spilling not yet supported"
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
/// **v0.3.0 stub**: still returns an empty `Vec` — per-instruction
/// validation happens during `lower_iir_to_ge225` itself.
pub fn validate_for_ge225(_module: &IIRModule) -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// lower_iir_to_ge225
// ===========================================================================

/// Lower an [`IIRModule`] to a `Vec<u8>` of GE-225 opcode bytes
/// (20-bit words packed 3 bytes each, big-endian).
///
/// See the module-level docs for the v0.3.0 per-op lowering table.
pub fn lower_iir_to_ge225(
    module: &IIRModule,
    _cfg: &IIRGe225Config,
) -> Result<Vec<u8>, IIRGe225Error> {
    let errors = validate_for_ge225(module);
    if !errors.is_empty() {
        return Err(IIRGe225Error::ValidationFailed(errors));
    }

    // Trivial empty-module contract — preserves v0.1.0's behaviour
    // for the canonical "fn main() {}" minimal case.
    if module.functions.is_empty() {
        return Ok(HALT_WORD.to_vec());
    }

    let mut bytes = Vec::new();
    for f in &module.functions {
        // ── Per-function ACC-first allocator state ───────────────────
        //
        // env: HashMap<String, u8>
        //   var name → physical location.  Values in `0..=15` are
        //   GP register indices for r0..r15; `ACC_MARKER` (= 16)
        //   means "currently lives in ACC".
        //
        // next_reg: usize
        //   Next free GP register index.  Bumps from 0 upward as
        //   we spill names from ACC into r0, r1, ...  Hits the
        //   `GP_REGISTER_COUNT` ceiling at 16.
        //
        // acc_owner: Option<String>
        //   Which var (if any) currently owns ACC.  When a new
        //   const / LD-clobbering op arrives, we first evict the
        //   current owner via STA r (= XCH r on this skeleton).
        let mut env: HashMap<String, u8> = HashMap::new();
        let mut next_reg: usize = 0;
        let mut acc_owner: Option<String> = None;

        for instr in &f.instructions {
            if !SUPPORTED_OPS.contains(&instr.op.as_str()) {
                return Err(IIRGe225Error::UnsupportedOp {
                    function: f.name.clone(),
                    op: instr.op.clone(),
                });
            }
            match instr.op.as_str() {
                // ── const dest, Int(n) → (STA r_evict)? + LDA n ───────
                //
                // If ACC is owned by a different var, evict it to its
                // next-free real register via STA r BEFORE the LDA
                // (which would otherwise clobber ACC).  Then emit LDA n
                // and dest becomes the new ACC owner.
                "const" => {
                    let dest = require_dest(instr, "const", &f.name)?;
                    let imm16 = encode_immediate_16(instr.srcs.first(), &f.name)?;
                    evict_acc(
                        &mut bytes,
                        &mut env,
                        &mut acc_owner,
                        &mut next_reg,
                        &f.name,
                    )?;
                    bytes.extend_from_slice(&encode_lda(imm16));
                    env.insert(dest.to_string(), ACC_MARKER);
                    acc_owner = Some(dest.to_string());
                }

                // ── mov dest, src → (STA r_evict)? + LD r_src + STA r_dest
                //
                // Lowering shape:
                //   - If src currently lives in ACC, evict src to a
                //     fresh GP register so it has a stable home.
                //   - LD r_src               ; ACC ← src's value
                //   - alloc r_dest
                //   - STA r_dest             ; r_dest ↔ ACC  (r_dest gets
                //                            ; src's value; ACC gets junk)
                //   - env[dest] = r_dest; acc_owner = None.
                "mov" => {
                    let dest = require_dest(instr, "mov", &f.name)?;
                    let src_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => {
                            return Err(IIRGe225Error::InvalidOperand {
                                function: f.name.clone(),
                                detail: "mov srcs[0] must be Var".into(),
                            })
                        }
                    };
                    // Confirm src exists in env (so we can give a
                    // crisp UndefinedVariable rather than a
                    // misleading OutOfRegisters later).
                    if !env.contains_key(&src_name) {
                        return Err(IIRGe225Error::UndefinedVariable {
                            function: f.name.clone(),
                            name: src_name,
                        });
                    }
                    // If src lives in ACC, evict so LD below has a
                    // stable register source.  evict_acc() also
                    // updates env[src] from ACC_MARKER to the real
                    // register index.
                    if matches!(env.get(&src_name), Some(&ACC_MARKER)) {
                        evict_acc(
                            &mut bytes,
                            &mut env,
                            &mut acc_owner,
                            &mut next_reg,
                            &f.name,
                        )?;
                    }
                    let src_reg = lookup_register(&env, &src_name, &f.name)?;
                    debug_assert!(
                        src_reg <= 15,
                        "src_reg should be a real GP register after eviction"
                    );
                    bytes.extend_from_slice(&encode_ld(src_reg));
                    let r_dest = alloc_register(&mut next_reg, dest, &f.name)?;
                    bytes.extend_from_slice(&encode_sta(r_dest));
                    env.insert(dest.to_string(), r_dest);
                    // After STA (XCH), ACC holds the (junk) old r_dest.
                    acc_owner = None;
                }

                // ── ret <var>: (LD r_var)? + HLT ──────────────────────
                //
                // If var is already the ACC owner, no LD is needed —
                // ACC already holds its value.  Otherwise emit
                // `LD r_var` to stage it into ACC, then HLT.
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
                    let src_reg = lookup_register(&env, &src_name, &f.name)?;
                    if src_reg != ACC_MARKER {
                        bytes.extend_from_slice(&encode_ld(src_reg));
                    }
                    bytes.extend_from_slice(&HALT_WORD);
                }

                // ── ret_void: HLT ─────────────────────────────────────
                "ret_void" => {
                    bytes.extend_from_slice(&HALT_WORD);
                }

                _ => unreachable!("SUPPORTED_OPS guard above prevents this"),
            }
        }
    }

    // Defensive — if every function was empty, fall back to
    // HALT_WORD so the output is still a valid halting program.
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

/// Evict the current ACC owner (if any) to its next-free GP register
/// via `STA r` (= XCH r on this skeleton).  Updates the `env`
/// mapping and clears `acc_owner`.  No-op when ACC is unowned.
///
/// Returns `OutOfRegisters` if all 16 GP registers are already
/// allocated.
fn evict_acc(
    bytes: &mut Vec<u8>,
    env: &mut HashMap<String, u8>,
    acc_owner: &mut Option<String>,
    next_reg: &mut usize,
    fn_name: &str,
) -> Result<(), IIRGe225Error> {
    if let Some(name) = acc_owner.take() {
        if *next_reg >= GP_REGISTER_COUNT {
            return Err(IIRGe225Error::OutOfRegisters {
                function: fn_name.to_string(),
                name,
            });
        }
        let r = *next_reg as u8;
        *next_reg += 1;
        bytes.extend_from_slice(&encode_sta(r));
        env.insert(name, r);
    }
    Ok(())
}

/// Allocate a fresh GP register for `dest`.  Returns the 4-bit
/// register index, or `OutOfRegisters` if all 16 are taken.
fn alloc_register(
    next_reg: &mut usize,
    dest: &str,
    fn_name: &str,
) -> Result<u8, IIRGe225Error> {
    if *next_reg >= GP_REGISTER_COUNT {
        return Err(IIRGe225Error::OutOfRegisters {
            function: fn_name.to_string(),
            name: dest.to_string(),
        });
    }
    let r = *next_reg as u8;
    *next_reg += 1;
    Ok(r)
}

fn lookup_register(
    env: &HashMap<String, u8>,
    name: &str,
    fn_name: &str,
) -> Result<u8, IIRGe225Error> {
    env.get(name)
        .copied()
        .ok_or_else(|| IIRGe225Error::UndefinedVariable {
            function: fn_name.to_string(),
            name: name.to_string(),
        })
}

// ---------------------------------------------------------------------------
// Word-encoding helpers
// ---------------------------------------------------------------------------

/// Encode an `LDA n` 20-bit word as 3 bytes, big-endian.
fn encode_lda(imm16: u16) -> [u8; 3] {
    [
        LDA_OPCODE_NIBBLE,
        ((imm16 >> 8) & 0xFF) as u8,
        (imm16 & 0xFF) as u8,
    ]
}

/// Encode an `STA r` 20-bit word as 3 bytes.  The register index `r`
/// is masked to 4 bits and placed in the low 4 bits of byte 2.
fn encode_sta(r: u8) -> [u8; 3] {
    [STA_OPCODE_NIBBLE, 0x00, r & 0x0F]
}

/// Encode an `LD r` 20-bit word as 3 bytes.
fn encode_ld(r: u8) -> [u8; 3] {
    [LD_OPCODE_NIBBLE, 0x00, r & 0x0F]
}

/// Decode and range-check a `const` immediate operand into a 16-bit
/// value (two's-complement reinterpretation for negatives).
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
    if (-32768..=32767).contains(&n) {
        Ok((n as i16) as u16)
    } else if (32768..=65535).contains(&n) {
        Ok(n as u16)
    } else {
        Err(IIRGe225Error::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!(
                "const {n} exceeds 16-bit immediate range ([-32768, 65535]); \
                 the GE-225 v0.3.0 LDA immediate is 16 bits wide — wider \
                 values must be built up via LDA-shift-ADD chains in A5+++"
            ),
        })
    }
}
