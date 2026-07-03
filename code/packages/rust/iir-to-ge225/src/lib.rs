//! # iir-to-ge225 — IIR → GE-225 machine code backend (v0.9.0, A5++++++++++).
//!
//! ## ⚠ DEPRECATED — use `ge225-backend` instead
//!
//! As of Phase 3 of the historical-arch backend migration
//! ([`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md)),
//! this crate is deprecated.  It sits at the wrong architectural
//! layer in the compiler stack: it consumes dynamically-typed IIR
//! directly and bypasses the `jit_core::backend::Backend` trait
//! that `aot-core` and `jit-core` use to plug native-emit backends
//! into both AOT and JIT in one shot.
//!
//! The replacement pair is:
//!
//! - **[`ge225-encoder`]** — the pure encoding tables (opcode
//!   constants, `encode_*` helpers).  No IR knowledge.
//! - **[`ge225-backend`]** — implements
//!   `jit_core::backend::Backend` over **monomorphised CIR**
//!   (`add_i64`, `cmp_lt_u32`, …) and emits the same bytes this
//!   crate did.
//!
//! `lang-aot --emit=ge225` already routes through the new pair as
//! of Phase 3; existing public API (constants and
//! `lower_iir_to_ge225`) of this crate continues to work for
//! backward compatibility but emits deprecation warnings.
//!
//! [`ge225-encoder`]: https://docs.rs/ge225-encoder
//! [`ge225-backend`]: https://docs.rs/ge225-backend
//!
//! ## Original module docs (still applicable to the lowering algorithm)
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
//! | `0x4` | `ADD r` | `ACC ← ACC + r` (r unchanged)          | `[0x04, 0x00, r]` |
//! | `0x5` | `SUB r` | `ACC ← ACC - r` (r unchanged)          | `[0x05, 0x00, r]` |
//! | `0x6` | `BR a`  | unconditional branch to byte addr `a`  | `[0x06, hi, lo]` |
//! | `0x7` | `BNZ a` | branch if ACC ≠ 0                      | `[0x07, hi, lo]` |
//! | `0x8` | `BZ a`  | branch if ACC = 0                      | `[0x08, hi, lo]` |
//! | `0x9` | `JSR a` | push PC+3, branch to `a`               | `[0x09, hi, lo]` |
//! | `0xA` | `RTS`   | pop, branch to popped address          | `[0x0A, 0x00, 0x00]` |
//! | `0xB` | `BMI a` | branch if ACC sign bit set (negative)  | `[0x0B, hi, lo]` |
//!
//! As of v0.7.0, `BMI` is **active** — `cmp_lt` and `cmp_le` lower
//! through it.  Future slices take `0xC..0xF`.
//!
//! ## Quick start
//!
//! ```
//! #![allow(deprecated)]
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
// GE-225 opcode constants — re-exported from `ge225-encoder`
// ===========================================================================
//
// Phase 1 of the historical-arch backend migration (see
// `code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`) carved these
// out of this crate into `ge225-encoder`, which is now the single
// source of truth.  This crate re-exports them so its public API
// is unchanged — `iir_to_ge225::HALT_WORD` still works for any
// existing caller — but every downstream consumer is encouraged
// to depend on `ge225-encoder` directly.

pub use ge225_encoder::{
    ADD_OPCODE_NIBBLE, BMI_OPCODE_NIBBLE, BNZ_OPCODE_NIBBLE, BR_OPCODE_NIBBLE, BZ_OPCODE_NIBBLE,
    HALT_WORD, JSR_OPCODE_NIBBLE, LDA_OPCODE_NIBBLE, LD_OPCODE_NIBBLE, RTS_OPCODE_NIBBLE, RTS_WORD,
    STA_OPCODE_NIBBLE, SUB_OPCODE_NIBBLE,
};
// Bring the `encode_*` helpers into scope under their bare names
// (we still use them throughout the lowering pass below).
use ge225_encoder::{encode_add, encode_ld, encode_lda, encode_sta, encode_sub};

/// Sentinel `env` value meaning "this var currently lives in the
/// accumulator (ACC)", distinct from real register indices `0..=15`.
const ACC_MARKER: u8 = 16;

/// Number of GP registers.  Re-exported from `ge225-encoder` for
/// the lowering pass below; kept as a local `const` (not a `pub use`)
/// so changing this requires touching the encoder.
const GP_REGISTER_COUNT: usize = ge225_encoder::GP_REGISTER_COUNT;

/// Supported instruction opcodes in v0.9.0 (A5++++++++++).
const SUPPORTED_OPS: &[&str] = &[
    "const", "mov", "add", "sub", "neg",
    "cmp_lt", "cmp_eq", "cmp_ne", "cmp_le", "cmp_gt", "cmp_ge",
    "label", "jmp", "jmp_if_true", "jmp_if_false",
    "call", "call_builtin",
    "ret", "ret_void",
];

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
    /// A `jmp` / `jmp_if_true` / `jmp_if_false` referenced a label
    /// name that wasn't defined by a `label` op anywhere in the
    /// same function.  Cross-function jumps aren't supported —
    /// labels are per-function in v0.5.0.
    UndefinedLabel { function: String, label: String },
    /// A branch target's resolved byte offset exceeds the 16-bit
    /// address space the `BR` / `BNZ` / `BZ` instruction word can
    /// encode (65 536 bytes / ~21 845 instruction words).  Programs
    /// that large would need a wider address-field encoding.
    BranchTargetOutOfRange {
        function: String,
        label: String,
        offset: usize,
    },
    /// A `call` referenced a function name that wasn't defined
    /// anywhere in the module.  Reported at module-level backpatch
    /// time so all functions have been seen.
    UndefinedFunction { caller: String, callee: String },
    /// A call target's resolved entry byte offset exceeds the
    /// 16-bit address field of the `JSR` instruction word.
    CallTargetOutOfRange {
        caller: String,
        callee: String,
        offset: usize,
    },
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
            Self::UndefinedLabel { function, label } => {
                write!(
                    f,
                    "undefined label {label:?} referenced by branch in function {function:?}"
                )
            }
            Self::BranchTargetOutOfRange {
                function,
                label,
                offset,
            } => {
                write!(
                    f,
                    "branch target {label:?} at byte offset {offset} in function \
                     {function:?} exceeds the 16-bit address field (max 65535)"
                )
            }
            Self::UndefinedFunction { caller, callee } => {
                write!(
                    f,
                    "undefined function {callee:?} referenced by call in function {caller:?}"
                )
            }
            Self::CallTargetOutOfRange {
                caller,
                callee,
                offset,
            } => {
                write!(
                    f,
                    "call target {callee:?} entry at byte offset {offset} in function \
                     {caller:?} exceeds the 16-bit address field (max 65535)"
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
///
/// # ⚠ Deprecated
///
/// This entry point sits at the wrong layer in the compiler
/// pipeline — see the module-level deprecation banner.  Use
/// [`ge225_backend::compile`] over CIR instead.
#[deprecated(
    since = "0.10.0",
    note = "use `ge225_backend::compile` over CIR — see code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md"
)]
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

    // ── Module-level call-backpatching state (v0.6.0, A5++++++) ──
    //
    // The GE-225 has no PC-relative `JSR` — every call carries an
    // absolute 16-bit byte address of the callee's entry word.  To
    // make forward-references possible, we emit `JSR <hi=0><lo=0>`
    // placeholders into `bytes` and record
    // `(slot_byte_offset, callee_name, caller_name)` in
    // `pending_calls`.  After every function has been emitted (so
    // every callee's entry address is known), we walk
    // `pending_calls` and write the resolved 16-bit address into
    // each slot's two bytes.  Mirrors the iir-to-intel8008 v0.3.9
    // call-backpatching pattern.
    let mut function_addrs: HashMap<String, usize> = HashMap::new();
    let mut pending_calls: Vec<(usize, String, String)> = Vec::new();

    // ── Entry-function discriminator ────────────────────────────
    //
    // `ret` / `ret_void` in the entry function emit `HLT` (so the
    // program halts cleanly when main returns).  Every other
    // function emits `RTS` (return from subroutine).  When
    // `entry_point` is `None` we conservatively make all rets
    // emit `RTS` — the IR author chose to omit an entry, so they
    // own the consequences.
    let entry_name = module.entry_point.as_deref();

    for f in &module.functions {
        // Record this function's entry byte offset for module-level
        // call-backpatching.
        function_addrs.insert(f.name.clone(), bytes.len());
        let is_entry_fn = entry_name == Some(f.name.as_str());
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

        // ── Per-function label-resolution state (v0.5.0) ──────────────
        //
        // The GE-225 has no PC-relative addressing in this skeleton —
        // every branch carries a 16-bit absolute byte address.  Forward
        // branches must be backpatched: when a `jmp X` is emitted before
        // `label X` appears, we record `(slot_high_byte_offset, X)` in
        // `pending_branches`.  After the function body is emitted, we
        // look up each target in `labels` and write the address into the
        // slot's two bytes (big-endian: byte at slot = hi, byte at slot+1
        // = lo).
        //
        // CRITICAL: byte offsets are scoped to the FULL `bytes` Vec, not
        // to the current function — that's because branches encode the
        // absolute byte address, and the function is emitted into the
        // same continuous byte stream.  But labels are per-function:
        // referencing a label in another function is rejected as
        // `UndefinedLabel`.
        //
        // A duplicate `label` definition overwrites the prior position
        // (last-one-wins) — same convention as iir-to-intel8008.
        let mut labels: HashMap<String, usize> = HashMap::new();
        let mut pending_branches: Vec<(usize, String)> = Vec::new();
        let function_start = bytes.len();

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

                // ── add / sub dest, lhs, rhs ──────────────────────────
                //
                // Lowering shape (3-step prep + 2-instruction arith):
                //
                //   1. If lhs lives in ACC, evict it (STA r) so it
                //      has a stable register home.
                //   2. If rhs lives in ACC (post-lhs-eviction, this
                //      requires lhs==rhs in the original IR), evict
                //      too.  After this both lhs and rhs are in real
                //      registers.
                //   3. Evict any remaining ACC owner so ACC is free
                //      for the LD r_lhs below.
                //
                //   LD  r_lhs          ; ACC ← lhs
                //   ADD r_rhs          ; ACC ← lhs + rhs   (or SUB)
                //
                //   env[dest] = ACC_MARKER; acc_owner = Some(dest).
                //
                // This deliberately conservative scheme always emits
                // the LD even when lhs was already the ACC owner —
                // it keeps the lowering shape predictable (always 2
                // words for the arithmetic step) at a small byte cost.
                // A future v0.5.0 may peephole-elide the LD when
                // lhs == acc_owner.
                "add" | "sub" => {
                    let dest = require_dest(instr, instr.op.as_str(), &f.name)?;
                    let (lhs_name, rhs_name) = parse_binop_srcs(instr, &f.name)?;
                    // Bind-check both operands up front for crisp errors.
                    if !env.contains_key(&lhs_name) {
                        return Err(IIRGe225Error::UndefinedVariable {
                            function: f.name.clone(),
                            name: lhs_name,
                        });
                    }
                    if !env.contains_key(&rhs_name) {
                        return Err(IIRGe225Error::UndefinedVariable {
                            function: f.name.clone(),
                            name: rhs_name,
                        });
                    }
                    // Step 1: evict lhs from ACC if present.
                    if matches!(env.get(&lhs_name), Some(&ACC_MARKER)) {
                        evict_acc(
                            &mut bytes,
                            &mut env,
                            &mut acc_owner,
                            &mut next_reg,
                            &f.name,
                        )?;
                    }
                    // Step 2: evict rhs from ACC if present.
                    if matches!(env.get(&rhs_name), Some(&ACC_MARKER)) {
                        evict_acc(
                            &mut bytes,
                            &mut env,
                            &mut acc_owner,
                            &mut next_reg,
                            &f.name,
                        )?;
                    }
                    // Step 3: evict any remaining ACC owner.
                    evict_acc(
                        &mut bytes,
                        &mut env,
                        &mut acc_owner,
                        &mut next_reg,
                        &f.name,
                    )?;
                    // Both operands are now in real GP registers; ACC is free.
                    let r_lhs = lookup_register(&env, &lhs_name, &f.name)?;
                    let r_rhs = lookup_register(&env, &rhs_name, &f.name)?;
                    debug_assert!(r_lhs <= 15 && r_rhs <= 15);
                    bytes.extend_from_slice(&encode_ld(r_lhs));
                    let arith = match instr.op.as_str() {
                        "add" => encode_add(r_rhs),
                        "sub" => encode_sub(r_rhs),
                        _ => unreachable!(),
                    };
                    bytes.extend_from_slice(&arith);
                    env.insert(dest.to_string(), ACC_MARKER);
                    acc_owner = Some(dest.to_string());
                }

                // ── neg dest, src → (evict)? + LDA 0 + SUB r_src ──────
                //
                // Two's-complement negation via subtract-from-zero:
                //
                //   ACC = 0;            (LDA 0)
                //   ACC = ACC - src;    (SUB r_src)
                //   → ACC = -src
                //
                // Steps:
                //   1. If src lives in ACC, evict it (so SUB has a
                //      stable register source).
                //   2. Evict any remaining ACC owner so LDA 0 doesn't
                //      clobber a live value.
                //   3. LD A 0 + SUB r_src.
                //   4. env[dest] = ACC_MARKER; acc_owner = Some(dest).
                //
                // The 16-bit immediate `0` in LDA 0 is just
                // `[0x01, 0x00, 0x00]`.  The trivial-case ROM is
                // `LDA n; STA r0; LDA 0; SUB r0; HLT` = 15 bytes,
                // pinned by tests.
                "neg" => {
                    let dest = require_dest(instr, "neg", &f.name)?;
                    let src_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => {
                            return Err(IIRGe225Error::InvalidOperand {
                                function: f.name.clone(),
                                detail: "neg requires srcs[0] = Operand::Var(src)".into(),
                            })
                        }
                    };
                    if !env.contains_key(&src_name) {
                        return Err(IIRGe225Error::UndefinedVariable {
                            function: f.name.clone(),
                            name: src_name,
                        });
                    }
                    // If src is in ACC, evict to a register first.
                    if matches!(env.get(&src_name), Some(&ACC_MARKER)) {
                        evict_acc(
                            &mut bytes,
                            &mut env,
                            &mut acc_owner,
                            &mut next_reg,
                            &f.name,
                        )?;
                    }
                    // Evict any remaining ACC owner (LDA 0 will clobber it).
                    evict_acc(
                        &mut bytes,
                        &mut env,
                        &mut acc_owner,
                        &mut next_reg,
                        &f.name,
                    )?;
                    let r_src = lookup_register(&env, &src_name, &f.name)?;
                    debug_assert!(r_src <= 15);
                    // LDA 0 then SUB r_src.
                    bytes.extend_from_slice(&encode_lda(0));
                    bytes.extend_from_slice(&encode_sub(r_src));
                    env.insert(dest.to_string(), ACC_MARKER);
                    acc_owner = Some(dest.to_string());
                }

                // ── cmp_{lt,eq,ne,le,gt,ge} dest, a, b ─────────────────
                //
                // ACC-based boolean materialisation pattern:
                //
                //   LD r_lhs                        ; ACC = lhs
                //   SUB r_rhs                       ; ACC = lhs - rhs (sign/zero set)
                //   <test1> <true_target>           ; conditional skip-to-true
                //   <test2> <true_target>           ; (only for le/ge — second branch)
                //   LDA 0                           ; false branch: dest = 0
                //   BR <end_target>
                //   <true_target>: LDA 1            ; true branch: dest = 1
                //   <end_target>:                   ; dest lives in ACC
                //
                // Mapping IIR op → test opcodes:
                //   cmp_lt: BMI            (a - b < 0  ⇔  a < b)
                //   cmp_gt: BMI, swap      (a > b  ⇔  b < a)
                //   cmp_eq: BZ             (a - b == 0)
                //   cmp_ne: BNZ            (a - b ≠ 0)
                //   cmp_le: BMI || BZ      (a ≤ b  ⇔  a < b ∨ a == b)
                //   cmp_ge: BMI || BZ, swap (a ≥ b  ⇔  b ≤ a)
                //
                // Single-test ops (lt/gt/eq/ne) emit 18 bytes after
                // the LD+SUB stage; double-test ops (le/ge) emit
                // 21 bytes.  The trivial-case `cmp_lt c, a, b; ret c`
                // shape is documented in the spec.
                "cmp_lt" | "cmp_eq" | "cmp_ne" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
                    let dest = require_dest(instr, instr.op.as_str(), &f.name)?;
                    let (lhs_orig, rhs_orig) = parse_binop_srcs(instr, &f.name)?;
                    // For cmp_gt and cmp_ge, swap operands so we can
                    // reuse the cmp_lt / cmp_le emit path verbatim.
                    let (lhs_name, rhs_name) = match instr.op.as_str() {
                        "cmp_gt" | "cmp_ge" => (rhs_orig, lhs_orig),
                        _ => (lhs_orig, rhs_orig),
                    };
                    // The post-SUB tests: which conditional branches
                    // jump to the "true" arm.
                    let test_opcodes: &[u8] = match instr.op.as_str() {
                        "cmp_lt" | "cmp_gt" => &[BMI_OPCODE_NIBBLE],
                        "cmp_eq" => &[BZ_OPCODE_NIBBLE],
                        "cmp_ne" => &[BNZ_OPCODE_NIBBLE],
                        "cmp_le" | "cmp_ge" => &[BMI_OPCODE_NIBBLE, BZ_OPCODE_NIBBLE],
                        _ => unreachable!(),
                    };
                    // Bind-check both operands up front for crisp errors.
                    if !env.contains_key(&lhs_name) {
                        return Err(IIRGe225Error::UndefinedVariable {
                            function: f.name.clone(),
                            name: lhs_name,
                        });
                    }
                    if !env.contains_key(&rhs_name) {
                        return Err(IIRGe225Error::UndefinedVariable {
                            function: f.name.clone(),
                            name: rhs_name,
                        });
                    }
                    // Same eviction strategy as add/sub: ensure both
                    // operands are in real GP registers, ACC is free.
                    if matches!(env.get(&lhs_name), Some(&ACC_MARKER)) {
                        evict_acc(
                            &mut bytes,
                            &mut env,
                            &mut acc_owner,
                            &mut next_reg,
                            &f.name,
                        )?;
                    }
                    if matches!(env.get(&rhs_name), Some(&ACC_MARKER)) {
                        evict_acc(
                            &mut bytes,
                            &mut env,
                            &mut acc_owner,
                            &mut next_reg,
                            &f.name,
                        )?;
                    }
                    evict_acc(
                        &mut bytes,
                        &mut env,
                        &mut acc_owner,
                        &mut next_reg,
                        &f.name,
                    )?;
                    let r_lhs = lookup_register(&env, &lhs_name, &f.name)?;
                    let r_rhs = lookup_register(&env, &rhs_name, &f.name)?;
                    debug_assert!(r_lhs <= 15 && r_rhs <= 15);
                    bytes.extend_from_slice(&encode_ld(r_lhs));
                    bytes.extend_from_slice(&encode_sub(r_rhs));

                    // Now ACC holds the signed difference.  Emit the
                    // boolean-materialisation suffix.  Compute the
                    // jump targets from the current byte offset:
                    //
                    //   n_tests = test_opcodes.len()   (1 or 2)
                    //   suffix layout (each instr = 3 bytes):
                    //     +0..3*n:        test_1, ..., test_n  → true_target
                    //     +3*n..3*n+3:    LDA 0
                    //     +3*n+3..3*n+6:  BR end_target
                    //     +3*n+6..3*n+9:  LDA 1  (true_target lands here)
                    //     +3*n+9:         end (no bytes; just an address)
                    let anchor = bytes.len();
                    let n_tests = test_opcodes.len();
                    let true_target = anchor + 3 * n_tests + 6;
                    let end_target = anchor + 3 * n_tests + 9;
                    if end_target > u16::MAX as usize {
                        return Err(IIRGe225Error::BranchTargetOutOfRange {
                            function: f.name.clone(),
                            label: format!("{}-internal-end", instr.op),
                            offset: end_target,
                        });
                    }
                    let true_target_u16 = true_target as u16;
                    let end_target_u16 = end_target as u16;
                    // Emit each conditional test pointing at true_target.
                    for &opcode in test_opcodes {
                        bytes.push(opcode);
                        bytes.push(((true_target_u16 >> 8) & 0xFF) as u8);
                        bytes.push((true_target_u16 & 0xFF) as u8);
                    }
                    // False branch: LDA 0; BR end.
                    bytes.extend_from_slice(&encode_lda(0));
                    bytes.push(BR_OPCODE_NIBBLE);
                    bytes.push(((end_target_u16 >> 8) & 0xFF) as u8);
                    bytes.push((end_target_u16 & 0xFF) as u8);
                    // True branch: LDA 1.
                    bytes.extend_from_slice(&encode_lda(1));
                    // dest takes over ACC.
                    env.insert(dest.to_string(), ACC_MARKER);
                    acc_owner = Some(dest.to_string());
                }

                // ── label "<name>": record current byte offset ─────────
                //
                // Zero bytes emitted.  `label` is purely a marker so
                // subsequent backpatching can resolve a forward branch
                // to a concrete 16-bit address.  A duplicate name
                // overwrites the prior position (last-one-wins).
                "label" => {
                    let name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => {
                            return Err(IIRGe225Error::InvalidOperand {
                                function: f.name.clone(),
                                detail: "label requires srcs[0] = Operand::Var(name)".into(),
                            })
                        }
                    };
                    labels.insert(name, bytes.len());
                }

                // ── jmp "<name>": unconditional 3-byte branch (BR addr)
                //
                // Pass 1: emit `0x06 0x00 0x00` and record (slot, name)
                // in pending_branches where slot is the byte offset of
                // the high-address byte (slot+0 = hi, slot+1 = lo).
                //
                // Branches do not modify ACC — `acc_owner` stays valid
                // across the branch instruction itself (though the
                // dynamic ACC contents at the target may differ).
                "jmp" => {
                    let target = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => {
                            return Err(IIRGe225Error::InvalidOperand {
                                function: f.name.clone(),
                                detail: "jmp requires srcs[0] = Operand::Var(label)".into(),
                            })
                        }
                    };
                    bytes.push(BR_OPCODE_NIBBLE);
                    let slot = bytes.len();
                    bytes.push(0); // high-address placeholder
                    bytes.push(0); // low-address placeholder
                    pending_branches.push((slot, target));
                }

                // ── jmp_if_true cond, "<label>" → (LD r_cond)? + BNZ addr
                // ── jmp_if_false cond, "<label>" → (LD r_cond)? + BZ addr
                //
                // Operand layout: srcs = [Var(cond_var), Var(target_label)].
                //
                // Stage cond into ACC (skip LD when cond is already the
                // ACC owner), then emit the conditional branch with a
                // 16-bit placeholder for backpatching.  Branches don't
                // clobber ACC, so the cond's value is still readable
                // after the branch — but acc_owner-as-cond-name remains
                // valid only if no eviction happened.
                "jmp_if_true" | "jmp_if_false" => {
                    let cond_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => {
                            return Err(IIRGe225Error::InvalidOperand {
                                function: f.name.clone(),
                                detail: format!(
                                    "{} requires srcs[0] = Operand::Var(cond)",
                                    instr.op
                                ),
                            })
                        }
                    };
                    let target = match instr.srcs.get(1) {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => {
                            return Err(IIRGe225Error::InvalidOperand {
                                function: f.name.clone(),
                                detail: format!(
                                    "{} requires srcs[1] = Operand::Var(label)",
                                    instr.op
                                ),
                            })
                        }
                    };
                    // Stage cond into ACC.
                    let cond_loc = *env.get(&cond_name).ok_or_else(|| {
                        IIRGe225Error::UndefinedVariable {
                            function: f.name.clone(),
                            name: cond_name.clone(),
                        }
                    })?;
                    if cond_loc != ACC_MARKER {
                        bytes.extend_from_slice(&encode_ld(cond_loc));
                        // ACC now holds cond's value but cond's
                        // canonical home is still its register; do not
                        // change acc_owner — set it to None so later
                        // const can evict cleanly without trying to
                        // double-evict cond.
                        acc_owner = None;
                    }
                    let opcode_nibble = if instr.op == "jmp_if_true" {
                        BNZ_OPCODE_NIBBLE
                    } else {
                        BZ_OPCODE_NIBBLE
                    };
                    bytes.push(opcode_nibble);
                    let slot = bytes.len();
                    bytes.push(0); // high-address placeholder
                    bytes.push(0); // low-address placeholder
                    pending_branches.push((slot, target));
                }

                // ── call (dest = )? fn_name → (evict ACC)? + JSR <addr>
                //
                // Operand layout: srcs = [Var(fn_name)], optional dest.
                //
                // Pass 1: evict any ACC owner (JSR's callee may clobber
                // ACC).  Emit `JSR 0x0000` placeholder bytes and record
                // (slot, callee, caller) in `pending_calls`.  After
                // every function has been emitted, the module-level
                // backpatching pass writes the callee's entry address
                // into the slot.
                //
                // After JSR returns, ACC holds the callee's return
                // value (by convention).  If the IIR site binds a
                // dest, claim ACC for dest.  Otherwise discard the
                // return value (acc_owner = None).
                //
                // Arguments aren't yet supported — calls in v0.6.0 are
                // zero-arg, single-return-value (via ACC).  Mirrors
                // iir-to-intel8008 v0.3.9's call-staging shape.
                "call" => {
                    let callee = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => {
                            return Err(IIRGe225Error::InvalidOperand {
                                function: f.name.clone(),
                                detail: "call requires srcs[0] = Operand::Var(fn_name)".into(),
                            })
                        }
                    };
                    // Evict any current ACC owner before JSR (the
                    // callee will clobber ACC).
                    evict_acc(
                        &mut bytes,
                        &mut env,
                        &mut acc_owner,
                        &mut next_reg,
                        &f.name,
                    )?;
                    bytes.push(JSR_OPCODE_NIBBLE);
                    let slot = bytes.len();
                    bytes.push(0); // high-address placeholder
                    bytes.push(0); // low-address placeholder
                    pending_calls.push((slot, callee, f.name.clone()));
                    // If the IIR site binds a dest, the callee's
                    // return value (in ACC) becomes dest's home.
                    if let Some(dest) = instr.dest.as_deref() {
                        env.insert(dest.to_string(), ACC_MARKER);
                        acc_owner = Some(dest.to_string());
                    } else {
                        // Discarded return value — leave ACC unowned.
                        acc_owner = None;
                    }
                }

                // ── call_builtin (dest =)? builtin_name, arg1, arg2 ...
                //
                // v0.8.0 lowering: NO-OP.  The GE-225 historically
                // routed I/O through a teletype the modern simulator
                // doesn't model, so there's no real opcode that fits
                // a "print" or "input" builtin.  We still:
                //
                //   * Validate that the first src is a Var (the
                //     builtin name) — catches IR-shape bugs.
                //   * Validate that every Var argument is bound in
                //     `env` — catches use-before-definition.
                //   * If a `dest` is bound, evict the current ACC
                //     owner and emit a deterministic `LDA 0` so dest
                //     has a well-defined value (instead of leaving
                //     env in an inconsistent state).
                //
                // No-dest call_builtin (e.g. `print_i64(x)`) emits
                // **zero bytes** — the entire instruction collapses
                // to a no-op.  A future increment could dispatch on
                // the builtin name and emit a synthesised I/O word
                // (e.g. JSR to a host-stub address).
                "call_builtin" => {
                    // First src: the builtin name (Var).
                    if !matches!(instr.srcs.first(), Some(Operand::Var(_))) {
                        return Err(IIRGe225Error::InvalidOperand {
                            function: f.name.clone(),
                            detail:
                                "call_builtin requires srcs[0] = Operand::Var(builtin_name)"
                                    .into(),
                        });
                    }
                    // Validate remaining Var args are bound.
                    for arg in instr.srcs.iter().skip(1) {
                        if let Operand::Var(name) = arg {
                            if !env.contains_key(name) {
                                return Err(IIRGe225Error::UndefinedVariable {
                                    function: f.name.clone(),
                                    name: name.clone(),
                                });
                            }
                        }
                        // Non-Var args (Int/Bool literals) are
                        // tolerated silently — the IIR shape allows
                        // them and they don't reference env.
                    }
                    // If a dest is bound, give it a deterministic
                    // placeholder value (0) in ACC.
                    if let Some(dest) = instr.dest.as_deref() {
                        evict_acc(
                            &mut bytes,
                            &mut env,
                            &mut acc_owner,
                            &mut next_reg,
                            &f.name,
                        )?;
                        bytes.extend_from_slice(&encode_lda(0));
                        env.insert(dest.to_string(), ACC_MARKER);
                        acc_owner = Some(dest.to_string());
                    }
                    // No-dest case: zero bytes emitted; acc_owner
                    // and env both unchanged.
                }

                // ── ret <var>: (LD r_var)? + HLT-or-RTS ───────────────
                //
                // If var is already the ACC owner, no LD is needed —
                // ACC already holds its value.  Otherwise emit
                // `LD r_var` to stage it into ACC.  Then:
                //
                //   * If this is the module's entry function: emit HLT
                //     (program halts cleanly when main returns).
                //   * Else: emit RTS (return from subroutine — pops the
                //     return address pushed by the corresponding JSR).
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
                    if is_entry_fn {
                        bytes.extend_from_slice(&HALT_WORD);
                    } else {
                        bytes.extend_from_slice(&RTS_WORD);
                    }
                }

                // ── ret_void: HLT (entry) or RTS (else) ───────────────
                "ret_void" => {
                    if is_entry_fn {
                        bytes.extend_from_slice(&HALT_WORD);
                    } else {
                        bytes.extend_from_slice(&RTS_WORD);
                    }
                }

                _ => unreachable!("SUPPORTED_OPS guard above prevents this"),
            }
        }

        // ── Per-function backpatching pass ───────────────────────────
        //
        // Now that every `label` op in this function has been seen
        // and recorded in `labels`, resolve every pending forward /
        // backward branch by writing the target's 16-bit absolute
        // byte offset into its 2-byte slot.  Errors raised here are
        // reported with the function name + label name so the user
        // can locate the offending IR site.
        //
        // The `function_start` value is captured but not currently
        // used for the address calculation — branches encode absolute
        // byte offsets within the entire emitted byte stream, so the
        // label position recorded earlier is already correct.  We
        // keep `function_start` as a hook for a future change that
        // wants per-function relative offsets.
        let _ = function_start;
        for (slot, target) in pending_branches {
            let offset = *labels.get(&target).ok_or_else(|| {
                IIRGe225Error::UndefinedLabel {
                    function: f.name.clone(),
                    label: target.clone(),
                }
            })?;
            if offset > u16::MAX as usize {
                return Err(IIRGe225Error::BranchTargetOutOfRange {
                    function: f.name.clone(),
                    label: target,
                    offset,
                });
            }
            let offset_u16 = offset as u16;
            bytes[slot] = ((offset_u16 >> 8) & 0xFF) as u8;
            bytes[slot + 1] = (offset_u16 & 0xFF) as u8;
        }
    }

    // ── Module-level call backpatching ───────────────────────────
    //
    // Every function has now been emitted into `bytes`, so every
    // callee's entry byte address is recorded in `function_addrs`.
    // Walk the per-caller `pending_calls` queue and write each
    // resolved 16-bit byte address into the JSR slot.
    //
    // Errors flow up with both the caller and callee names so the
    // user can locate the offending IR site without parsing the
    // module twice.
    for (slot, callee, caller) in pending_calls {
        let offset = *function_addrs.get(&callee).ok_or_else(|| {
            IIRGe225Error::UndefinedFunction {
                caller: caller.clone(),
                callee: callee.clone(),
            }
        })?;
        if offset > u16::MAX as usize {
            return Err(IIRGe225Error::CallTargetOutOfRange {
                caller,
                callee,
                offset,
            });
        }
        let offset_u16 = offset as u16;
        bytes[slot] = ((offset_u16 >> 8) & 0xFF) as u8;
        bytes[slot + 1] = (offset_u16 & 0xFF) as u8;
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
//
// As of Phase 1 of the historical-arch backend migration, the
// per-opcode `encode_*` helpers live in `ge225-encoder` and are
// `use`d at the top of this file.  This crate still owns the
// `IR-aware` encoders (e.g. `encode_immediate_16` which range-
// checks an `Operand::Int`) — those continue to live below.

/// Parse `(Var(lhs), Var(rhs))` out of a binary-op `IIRInstr.srcs`.
/// Returns `InvalidOperand` if the shape doesn't match.
fn parse_binop_srcs(
    instr: &interpreter_ir::IIRInstr,
    fn_name: &str,
) -> Result<(String, String), IIRGe225Error> {
    let lhs = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.clone(),
        _ => {
            return Err(IIRGe225Error::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("{} srcs[0] must be Var", instr.op),
            })
        }
    };
    let rhs = match instr.srcs.get(1) {
        Some(Operand::Var(s)) => s.clone(),
        _ => {
            return Err(IIRGe225Error::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("{} srcs[1] must be Var", instr.op),
            })
        }
    };
    Ok((lhs, rhs))
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
