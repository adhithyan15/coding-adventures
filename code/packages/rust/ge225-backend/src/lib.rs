//! # `ge225-backend` — GE-225 backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into 20-bit GE-225 machine code via
//! [`ge225_encoder`].  Plugs into both `aot-core` (AOT byte
//! emission) and `jit-core` (via the shared
//! [`jit_core::backend::Backend`] trait) — same shape as
//! `aarch64-backend` / `x86_64-backend`.
//!
//! ## Scope
//!
//! | Family | CIR mnemonics | Lowering shape |
//! |--------|---------------|----------------|
//! | Constants | `const_i8` … `const_i64`, `const_u8` … `const_u64`, `const_bool` | `(STA r_evict)?` + `LDA n` |
//! | Move | `mov_i8` … `mov_i64`, `mov_u8` … `mov_u64`, `mov_bool` | `(STA r_evict_src)?` + `LD r_src` + `STA r_dest` |
//! | Add | `add_i8` … `add_i64`, `add_u8` … `add_u64` | `(evict)?` + `LD r_lhs` + `ADD r_rhs` |
//! | Sub | `sub_i8` … `sub_i64`, `sub_u8` … `sub_u64` | `(evict)?` + `LD r_lhs` + `SUB r_rhs` |
//! | Neg | `neg_i8` … `neg_i64` | `(evict)?` + `LDA 0` + `SUB r_src` |
//! | Cmp | `cmp_lt_*`, `cmp_eq_*`, `cmp_ne_*`, `cmp_le_*`, `cmp_gt_*`, `cmp_ge_*` (signed and unsigned) | SUB-then-test boolean materialisation |
//! | Control flow | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` | `BR` / `BNZ` / `BZ` with per-function backpatching |
//! | Returns | `ret_*`, `ret_void` | `(LD r_var)?` + `HLT` |
//! | Calls | `call`, `call_builtin` | `call_builtin` is a no-op; `call` returns `None` from `compile` until Phase 3 adds module-level relocation support |
//! | Float, send, properties, globals, type_assert | **NOT YET** | returns `None` |
//!
//! Anything outside this list returns `None` from `compile`, which
//! both `aot-core` and `jit-core` treat as a compile failure for
//! that function (graceful fallback).
//!
//! ## Why CIR (not IIR)?
//!
//! The previous historical-arch crate (`iir-to-ge225`) consumed
//! IIR directly — dynamically typed, requiring the backend to do
//! its own type inference.  This crate consumes the **monomorphised
//! CIR** that `aot_core::infer::infer_types` +
//! `aot_core::specialise::aot_specialise` produce.  Every op is
//! type-suffixed (`add_i64`, `cmp_lt_u32`), so the backend just
//! pattern-matches the prefix.
//!
//! For GE-225 specifically: the silicon has one 20-bit accumulator
//! width.  Every `const_*` variant lowers to the same `LDA n` (with
//! a 16-bit-immediate range check); every `add_*` to the same
//! `ADD r`; etc.  CIR's type suffix is informational only on this
//! arch.
//!
//! ## Why is `Backend::run` not implemented?
//!
//! Per the migration spec
//! ([`HISTORICAL-ARCH-BACKEND-MIGRATION.md`]), the historical-arch
//! backends are **emit-only**.  No in-process simulator exists in
//! this crate; downstream the bytes are loaded into
//! `ge225-simulator` or a custom decoder for execution.
//! `Backend::run` panics with a clear message — the trait is
//! satisfied so the backend can plug into the `jit-core` registry,
//! but no caller should reach `run`.
//!
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`]: ../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md
//!
//! ## Status — v0.1.0 (Phase 2 of the migration)
//!
//! - Implements `Backend` trait.  `compile_function` is the real
//!   entry point; bare `compile` delegates to it with an empty
//!   `FunctionContext`.
//! - Single-function emit only — cross-function `call` returns
//!   `None`.  Phase 3 will add module-level orchestration via
//!   `aot_core::link` + relocations.
//! - Byte sequences are identical to `iir-to-ge225` v0.9.0 for
//!   equivalent CIR inputs: the trivial-case ROMs (6-byte
//!   const+ret, 21-byte add, 33-byte cmp_lt, 15-byte neg) all
//!   round-trip via the same exact bytes.

use ge225_encoder::{
    encode_add, encode_bmi, encode_bnz, encode_br, encode_bz, encode_ld, encode_lda, encode_sta,
    encode_sub, HALT_WORD,
};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::collections::HashMap;
use std::fmt;
use vm_core::value::Value;

// ===========================================================================
// Public types
// ===========================================================================

/// The GE-225 backend handle.  Zero-sized — all state lives per
/// compile invocation.
///
/// Construct with [`Ge225Backend::new()`] (or `Default`).  Register
/// with `jit-core` via `Box<dyn Backend>` if you want JIT
/// integration (but see the module docs — JIT execution will panic
/// because the backend is emit-only).
#[derive(Debug, Default, Clone, Copy)]
pub struct Ge225Backend;

impl Ge225Backend {
    /// Construct a new backend handle.
    pub fn new() -> Self {
        Ge225Backend
    }
}

/// Per-instruction lowering errors.  Returned by [`compile`] for
/// diagnostic use.  The `Backend` trait coerces these to `None`
/// for the JIT / AOT graceful-failure paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    /// CIR op not yet handled by this backend.
    UnsupportedOp(String),
    /// An operand has an unexpected shape.
    InvalidOperand(String),
    /// A variable was referenced before being defined by a prior
    /// CIR instruction's `dest`.
    UndefinedVariable(String),
    /// `const_*` immediate didn't fit in the GE-225's 16-bit
    /// immediate field (range `[-32768, 65535]`).
    ImmediateOutOfRange(i64),
    /// More than the 17-slot pool (ACC + r0..r15) could hold.
    OutOfRegisters(String),
    /// A `jmp` referenced a label not defined in this function.
    UndefinedLabel(String),
    /// A branch target's byte offset exceeded the 16-bit address
    /// field (max 65535).
    BranchTargetOutOfRange(usize),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOp(op) => write!(f, "ge225-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "ge225-backend: invalid operand: {d}"),
            Self::UndefinedVariable(name) => {
                write!(f, "ge225-backend: undefined variable {name:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "ge225-backend: const {n} exceeds the GE-225 16-bit immediate range [-32768, 65535]"
            ),
            Self::OutOfRegisters(name) => write!(
                f,
                "ge225-backend: out of registers (ACC + r0..r15 = 17 slots) while binding {name:?}"
            ),
            Self::UndefinedLabel(label) => {
                write!(f, "ge225-backend: undefined label {label:?}")
            }
            Self::BranchTargetOutOfRange(off) => write!(
                f,
                "ge225-backend: branch target byte offset {off} exceeds the 16-bit address field"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

// ===========================================================================
// Public compile() — single-function emit, no cross-function refs
// ===========================================================================

/// Compile a single function's CIR into GE-225 bytes.
///
/// This is the canonical single-function entry point — the same
/// shape as `aarch64_backend::compile`.  Errors carry a structured
/// `BackendError`; the `Backend` trait's `compile_function` coerces
/// them to `None` for graceful fallback.
///
/// **v0.1.0 scope**: cross-function `call` returns
/// `Err(UnsupportedOp("call"))`.  Phase 3 will add module-level
/// relocation support so `call` can resolve to a callee entry
/// address via `aot_core::link`.
///
/// The `_ctx` parameter is currently unused but kept in the
/// signature to match `aarch64_backend::compile`.  Future versions
/// will use it for entry-vs-non-entry function detection (the
/// `ret_*`-emits-`HLT` vs `RTS` discriminator), at which point the
/// caller will need to populate `FunctionContext::name` correctly.
pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_single_function(cir)
}

// ===========================================================================
// Internals — single-function compile
// ===========================================================================

/// Sentinel `env` value meaning "this var currently lives in ACC".
const ACC_MARKER: u8 = 16;

/// Number of GP registers — re-exported from `ge225-encoder` for
/// the eviction-budget check.
const GP_REGISTER_COUNT: usize = ge225_encoder::GP_REGISTER_COUNT;

fn compile_single_function(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    // Empty CIR → just emit a HLT so the output is still a valid
    // halting program.  Mirrors iir-to-ge225's empty-module path.
    if cir.is_empty() {
        return Ok(HALT_WORD.to_vec());
    }

    let mut bytes = Vec::new();
    let mut env: HashMap<String, u8> = HashMap::new();
    let mut next_reg: usize = 0;
    let mut acc_owner: Option<String> = None;
    let mut labels: HashMap<String, usize> = HashMap::new();
    let mut pending_branches: Vec<(usize, String)> = Vec::new();

    for instr in cir {
        let op = instr.op.as_str();

        // ── label "<name>": record current byte offset ─────────────
        if op == "label" {
            let name = parse_var_src(instr, 0, "label")?;
            labels.insert(name, bytes.len());
            continue;
        }

        // ── jmp "<target>": BR + 16-bit address (backpatched) ──────
        if op == "jmp" {
            let target = parse_var_src(instr, 0, "jmp")?;
            bytes.extend_from_slice(&encode_br(0));
            // encode_br emitted 3 bytes; the high byte is at bytes.len()-2,
            // low byte at bytes.len()-1 — but encode_br doesn't expose
            // intermediate slots, so we re-slice manually.
            let slot = bytes.len() - 2;
            pending_branches.push((slot, target));
            continue;
        }

        // ── jmp_if_true / jmp_if_false cond, "<target>" ────────────
        if op == "jmp_if_true" || op == "jmp_if_false" {
            let cond_name = parse_var_src(instr, 0, op)?;
            let target = parse_var_src(instr, 1, op)?;
            stage_var_into_acc(&cond_name, &env, &mut acc_owner, &mut bytes)?;
            let encode_fn = if op == "jmp_if_true" {
                encode_bnz
            } else {
                encode_bz
            };
            bytes.extend_from_slice(&encode_fn(0));
            let slot = bytes.len() - 2;
            pending_branches.push((slot, target));
            continue;
        }

        // ── call_builtin (dest =)? builtin, args... ────────────────
        //
        // No-op lowering (GE-225 has no I/O opcode on this skeleton).
        // If dest is bound, evict current ACC owner and emit
        // `LDA 0` as a deterministic placeholder return value.
        if op == "call_builtin" {
            // First src must be Var(builtin_name).
            if !matches!(instr.srcs.first(), Some(CIROperand::Var(_))) {
                return Err(BackendError::InvalidOperand(
                    "call_builtin requires srcs[0] = Var(builtin_name)".into(),
                ));
            }
            // Validate remaining Var args are bound.
            for arg in instr.srcs.iter().skip(1) {
                if let CIROperand::Var(name) = arg {
                    if !env.contains_key(name) {
                        return Err(BackendError::UndefinedVariable(name.clone()));
                    }
                }
            }
            if let Some(dest) = instr.dest.as_deref() {
                evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
                bytes.extend_from_slice(&encode_lda(0));
                env.insert(dest.to_string(), ACC_MARKER);
                acc_owner = Some(dest.to_string());
            }
            continue;
        }

        // ── call dest = fn_name, args... ───────────────────────────
        //
        // Phase 2 stub: single-function emit cannot resolve a
        // cross-function callee address.  Phase 3 will add
        // relocation support.
        if op == "call" {
            return Err(BackendError::UnsupportedOp(
                "call (cross-function — needs Phase 3 relocation support)".into(),
            ));
        }

        // ── ret_void: HLT ──────────────────────────────────────────
        if op == "ret_void" {
            bytes.extend_from_slice(&HALT_WORD);
            continue;
        }

        // ── ret_<ty>: stage var into ACC, then HLT ─────────────────
        if op.strip_prefix("ret_").is_some() {
            let src_name = parse_var_src(instr, 0, op)?;
            let loc = *env
                .get(&src_name)
                .ok_or_else(|| BackendError::UndefinedVariable(src_name.clone()))?;
            if loc != ACC_MARKER {
                bytes.extend_from_slice(&encode_ld(loc));
            }
            bytes.extend_from_slice(&HALT_WORD);
            continue;
        }

        // ── const_<ty> dest, lit ───────────────────────────────────
        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let imm16 = encode_immediate_16(instr.srcs.first())?;
            evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            bytes.extend_from_slice(&encode_lda(imm16));
            env.insert(dest.to_string(), ACC_MARKER);
            acc_owner = Some(dest.to_string());
            continue;
        }

        // ── mov_<ty> dest, src ─────────────────────────────────────
        if op.strip_prefix("mov_").is_some() {
            let dest = require_dest(instr, op)?;
            let src_name = parse_var_src(instr, 0, op)?;
            if !env.contains_key(&src_name) {
                return Err(BackendError::UndefinedVariable(src_name.clone()));
            }
            if matches!(env.get(&src_name), Some(&ACC_MARKER)) {
                evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            }
            let src_reg = env[&src_name];
            bytes.extend_from_slice(&encode_ld(src_reg));
            let r_dest = alloc_register(&mut next_reg, dest)?;
            bytes.extend_from_slice(&encode_sta(r_dest));
            env.insert(dest.to_string(), r_dest);
            acc_owner = None;
            continue;
        }

        // ── add_<ty> / sub_<ty> ────────────────────────────────────
        if op.strip_prefix("add_").is_some() || op.strip_prefix("sub_").is_some() {
            let dest = require_dest(instr, op)?;
            let (lhs, rhs) = parse_binop_srcs(instr, op)?;
            check_bound(&env, &lhs)?;
            check_bound(&env, &rhs)?;
            // Same 3-step eviction prep as iir-to-ge225 v0.4.0.
            if matches!(env.get(&lhs), Some(&ACC_MARKER)) {
                evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            }
            if matches!(env.get(&rhs), Some(&ACC_MARKER)) {
                evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            }
            evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            let r_lhs = env[&lhs];
            let r_rhs = env[&rhs];
            bytes.extend_from_slice(&encode_ld(r_lhs));
            if op.starts_with("add_") {
                bytes.extend_from_slice(&encode_add(r_rhs));
            } else {
                bytes.extend_from_slice(&encode_sub(r_rhs));
            }
            env.insert(dest.to_string(), ACC_MARKER);
            acc_owner = Some(dest.to_string());
            continue;
        }

        // ── neg_<ty> dest, src ─────────────────────────────────────
        if op.strip_prefix("neg_").is_some() {
            let dest = require_dest(instr, op)?;
            let src = parse_var_src(instr, 0, op)?;
            check_bound(&env, &src)?;
            if matches!(env.get(&src), Some(&ACC_MARKER)) {
                evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            }
            evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            let r_src = env[&src];
            bytes.extend_from_slice(&encode_lda(0));
            bytes.extend_from_slice(&encode_sub(r_src));
            env.insert(dest.to_string(), ACC_MARKER);
            acc_owner = Some(dest.to_string());
            continue;
        }

        // ── cmp_{lt,gt,eq,ne,le,ge}_<ty> ───────────────────────────
        //
        // CIR cmp ops are type-suffixed (cmp_lt_i64).  We strip the
        // type suffix and dispatch on the predicate.
        if let Some(rest) = op.strip_prefix("cmp_") {
            // The predicate is the first underscore-separated piece.
            let predicate = rest.split('_').next().unwrap_or(rest);
            let (test_opcodes, swap_operands): (&[u8], bool) = match predicate {
                "lt" => (&[ge225_encoder::BMI_OPCODE_NIBBLE], false),
                "gt" => (&[ge225_encoder::BMI_OPCODE_NIBBLE], true),
                "eq" => (&[ge225_encoder::BZ_OPCODE_NIBBLE], false),
                "ne" => (&[ge225_encoder::BNZ_OPCODE_NIBBLE], false),
                "le" => (
                    &[
                        ge225_encoder::BMI_OPCODE_NIBBLE,
                        ge225_encoder::BZ_OPCODE_NIBBLE,
                    ],
                    false,
                ),
                "ge" => (
                    &[
                        ge225_encoder::BMI_OPCODE_NIBBLE,
                        ge225_encoder::BZ_OPCODE_NIBBLE,
                    ],
                    true,
                ),
                _ => return Err(BackendError::UnsupportedOp(op.to_string())),
            };
            let dest = require_dest(instr, op)?;
            let (lhs_orig, rhs_orig) = parse_binop_srcs(instr, op)?;
            let (lhs, rhs) = if swap_operands {
                (rhs_orig, lhs_orig)
            } else {
                (lhs_orig, rhs_orig)
            };
            check_bound(&env, &lhs)?;
            check_bound(&env, &rhs)?;
            if matches!(env.get(&lhs), Some(&ACC_MARKER)) {
                evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            }
            if matches!(env.get(&rhs), Some(&ACC_MARKER)) {
                evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            }
            evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            let r_lhs = env[&lhs];
            let r_rhs = env[&rhs];
            bytes.extend_from_slice(&encode_ld(r_lhs));
            bytes.extend_from_slice(&encode_sub(r_rhs));

            // Materialise the 0/1 boolean.  Layout after this point
            // (relative to `anchor = bytes.len()`):
            //   anchor + 3*n_tests           — test_1, ..., test_n branches to true_target
            //   anchor + 3*n_tests + 3       — LDA 0   (false branch)
            //   anchor + 3*n_tests + 6       — BR end_target
            //   anchor + 3*n_tests + 9       — LDA 1   (true branch — true_target)
            //   anchor + 3*n_tests + 12      — end_target (no bytes; just an address)
            let anchor = bytes.len();
            let n_tests = test_opcodes.len();
            let true_target = anchor + 3 * n_tests + 6;
            let end_target = anchor + 3 * n_tests + 9;
            if end_target > u16::MAX as usize {
                return Err(BackendError::BranchTargetOutOfRange(end_target));
            }
            let true_target_u16 = true_target as u16;
            let end_target_u16 = end_target as u16;
            for &opcode in test_opcodes {
                // Reuse the encoders so the byte sequences match
                // iir-to-ge225 v0.7.0 exactly.
                let bytes_arr: [u8; 3] = match opcode {
                    o if o == ge225_encoder::BMI_OPCODE_NIBBLE => encode_bmi(true_target_u16),
                    o if o == ge225_encoder::BZ_OPCODE_NIBBLE => encode_bz(true_target_u16),
                    o if o == ge225_encoder::BNZ_OPCODE_NIBBLE => encode_bnz(true_target_u16),
                    _ => unreachable!(),
                };
                bytes.extend_from_slice(&bytes_arr);
            }
            bytes.extend_from_slice(&encode_lda(0));
            bytes.extend_from_slice(&encode_br(end_target_u16));
            bytes.extend_from_slice(&encode_lda(1));
            env.insert(dest.to_string(), ACC_MARKER);
            acc_owner = Some(dest.to_string());
            continue;
        }

        // ── Unhandled op ───────────────────────────────────────────
        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    // ── Per-function backpatching pass ────────────────────────────
    for (slot, target) in pending_branches {
        let offset = *labels
            .get(&target)
            .ok_or_else(|| BackendError::UndefinedLabel(target.clone()))?;
        if offset > u16::MAX as usize {
            return Err(BackendError::BranchTargetOutOfRange(offset));
        }
        let offset_u16 = offset as u16;
        bytes[slot] = ((offset_u16 >> 8) & 0xFF) as u8;
        bytes[slot + 1] = (offset_u16 & 0xFF) as u8;
    }

    if bytes.is_empty() {
        bytes.extend_from_slice(&HALT_WORD);
    }
    Ok(bytes)
}

// ===========================================================================
// Per-instruction helpers
// ===========================================================================

fn require_dest<'a>(instr: &'a CIRInstr, op: &str) -> Result<&'a str, BackendError> {
    instr
        .dest
        .as_deref()
        .ok_or_else(|| BackendError::InvalidOperand(format!("{op} requires a dest")))
}

fn parse_var_src(instr: &CIRInstr, idx: usize, op: &str) -> Result<String, BackendError> {
    match instr.srcs.get(idx) {
        Some(CIROperand::Var(s)) => Ok(s.clone()),
        _ => Err(BackendError::InvalidOperand(format!(
            "{op} srcs[{idx}] must be Var"
        ))),
    }
}

fn parse_binop_srcs(instr: &CIRInstr, op: &str) -> Result<(String, String), BackendError> {
    Ok((parse_var_src(instr, 0, op)?, parse_var_src(instr, 1, op)?))
}

fn check_bound(env: &HashMap<String, u8>, name: &str) -> Result<(), BackendError> {
    if env.contains_key(name) {
        Ok(())
    } else {
        Err(BackendError::UndefinedVariable(name.to_string()))
    }
}

fn evict_acc(
    bytes: &mut Vec<u8>,
    env: &mut HashMap<String, u8>,
    acc_owner: &mut Option<String>,
    next_reg: &mut usize,
) -> Result<(), BackendError> {
    if let Some(name) = acc_owner.take() {
        if *next_reg >= GP_REGISTER_COUNT {
            return Err(BackendError::OutOfRegisters(name));
        }
        let r = *next_reg as u8;
        *next_reg += 1;
        bytes.extend_from_slice(&encode_sta(r));
        env.insert(name, r);
    }
    Ok(())
}

fn alloc_register(next_reg: &mut usize, dest: &str) -> Result<u8, BackendError> {
    if *next_reg >= GP_REGISTER_COUNT {
        return Err(BackendError::OutOfRegisters(dest.to_string()));
    }
    let r = *next_reg as u8;
    *next_reg += 1;
    Ok(r)
}

/// Stage a variable's value into ACC.  Used by `jmp_if_*` to read
/// the condition.
///
/// If the var is the current ACC owner, no `LD` is needed.
/// Otherwise emits `LD r_var` and sets `acc_owner = None` (since
/// the var's canonical home is still its register; ACC just has a
/// copy now).
fn stage_var_into_acc(
    name: &str,
    env: &HashMap<String, u8>,
    acc_owner: &mut Option<String>,
    bytes: &mut Vec<u8>,
) -> Result<(), BackendError> {
    let loc = *env
        .get(name)
        .ok_or_else(|| BackendError::UndefinedVariable(name.to_string()))?;
    if loc != ACC_MARKER {
        bytes.extend_from_slice(&encode_ld(loc));
        *acc_owner = None;
    }
    Ok(())
}

/// Decode and range-check a `const_*` immediate operand into a
/// 16-bit value.  CIR's `CIROperand::Int(i64)` covers all integer
/// widths via cast; we just check the value fits the GE-225's
/// 16-bit immediate field.
fn encode_immediate_16(op: Option<&CIROperand>) -> Result<u16, BackendError> {
    let n: i64 = match op {
        Some(CIROperand::Int(n)) => *n,
        Some(CIROperand::Bool(b)) => {
            if *b {
                1
            } else {
                0
            }
        }
        _ => {
            return Err(BackendError::InvalidOperand(
                "const_* srcs[0] must be Int or Bool".into(),
            ));
        }
    };
    if (-32_768..=32_767).contains(&n) {
        Ok((n as i16) as u16)
    } else if (32_768..=65_535).contains(&n) {
        Ok(n as u16)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

// ===========================================================================
// Backend trait impl
// ===========================================================================

impl Backend for Ge225Backend {
    fn name(&self) -> &str {
        "ge225"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        // Per the migration spec: ge225-backend is emit-only.  No
        // in-process simulator is wired here.  A future increment
        // could forward to `ge225-simulator` if/when an executor
        // contract is defined; until then, calling `run` is a bug
        // (jit-core should never reach it because `compile` returns
        // bytes for emit-only inspection, not execution).
        panic!(
            "ge225 backend is emit-only; load bytes into a ge225 simulator to execute.  \
             See code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md."
        );
    }
}
