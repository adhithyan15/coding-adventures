//! # `intel4004-backend` — Intel 4004 backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into Intel 4004 machine code via
//! [`intel4004_encoder`].  Mirror of `ge225-backend` /
//! `aarch64-backend` in shape and intent.
//!
//! ## Scope (v0.1.0 — Phase 4 of the migration)
//!
//! Same op set the deprecated `iir-to-intel4004` v0.3.0 covered,
//! but consuming **monomorphised CIR** (`const_i64`, `ret_i64`,
//! …) instead of dynamically-typed IIR.
//!
//! | Family | CIR mnemonics | Lowering |
//! |--------|---------------|----------|
//! | Constants | `const_i8` … `const_i64`, `const_u8` … `const_u64`, `const_bool` | `(XCH r_evict)?` + `LDM n` |
//! | Move | `mov_*` | `(XCH r_evict_src)?` + `LD r_src` + `XCH r_dest` |
//! | Returns | `ret_*`, `ret_void` | `(LD r_var)?` + `JUN 0x000` (halt loop) |
//! | Anything else | — | returns `None` (graceful AOT/JIT fallback) |
//!
//! ## Why is `Backend::run` not implemented?
//!
//! Per the migration spec, the historical-arch backends are
//! **emit-only**.  Bytes go to a downstream simulator
//! (`intel4004-simulator`), the in-tree `intel-4004-assembler`
//! for round-trip, or an EPROM burner for a 4004 dev board.
//! `Backend::run` panics with a clear message.

use intel4004_encoder::{encode_ld, encode_ldm, encode_xch, HALT_LOOP};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::collections::HashMap;
use std::fmt;
use vm_core::value::Value;

// ===========================================================================
// Public types
// ===========================================================================

#[derive(Debug, Default, Clone, Copy)]
pub struct Intel4004Backend;

impl Intel4004Backend {
    pub fn new() -> Self {
        Intel4004Backend
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    UnsupportedOp(String),
    InvalidOperand(String),
    UndefinedVariable(String),
    ImmediateOutOfRange(i64),
    OutOfRegisters(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOp(op) => write!(f, "intel4004-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "intel4004-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "intel4004-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "intel4004-backend: const {n} exceeds 4-bit nibble range [-8, 15]"
            ),
            Self::OutOfRegisters(n) => write!(
                f,
                "intel4004-backend: out of registers (ACC + r0..r15 = 17 slots) while binding {n:?}"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

// ===========================================================================
// Public compile() — single-function emit
// ===========================================================================

pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_single_function(cir)
}

// ===========================================================================
// Internals
// ===========================================================================

const ACC_MARKER: u8 = 16;
const GP_REGISTER_COUNT: usize = intel4004_encoder::GP_REGISTER_COUNT;

fn compile_single_function(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    if cir.is_empty() {
        return Ok(HALT_LOOP.to_vec());
    }

    let mut bytes = Vec::new();
    let mut env: HashMap<String, u8> = HashMap::new();
    let mut next_reg: usize = 0;
    let mut acc_owner: Option<String> = None;

    for instr in cir {
        let op = instr.op.as_str();

        // ── ret_void: JUN 0x000 ──────────────────────────────────────
        if op == "ret_void" {
            bytes.extend_from_slice(&HALT_LOOP);
            continue;
        }

        // ── ret_<ty>: (LD r_var)? + JUN 0x000 ────────────────────────
        if op.strip_prefix("ret_").is_some() {
            let src_name = parse_var_src(instr, 0, op)?;
            let loc = *env
                .get(&src_name)
                .ok_or_else(|| BackendError::UndefinedVariable(src_name.clone()))?;
            if loc != ACC_MARKER {
                bytes.push(encode_ld(loc));
            }
            bytes.extend_from_slice(&HALT_LOOP);
            continue;
        }

        // ── const_<ty> dest, lit ─────────────────────────────────────
        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let nibble = encode_immediate_nibble(instr.srcs.first())?;
            evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            bytes.push(encode_ldm(nibble));
            env.insert(dest.to_string(), ACC_MARKER);
            acc_owner = Some(dest.to_string());
            continue;
        }

        // ── mov_<ty> dest, src ───────────────────────────────────────
        if op.strip_prefix("mov_").is_some() {
            let dest = require_dest(instr, op)?;
            let src_name = parse_var_src(instr, 0, op)?;
            if !env.contains_key(&src_name) {
                return Err(BackendError::UndefinedVariable(src_name));
            }
            if matches!(env.get(&src_name), Some(&ACC_MARKER)) {
                evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg)?;
            }
            let src_reg = env[&src_name];
            bytes.push(encode_ld(src_reg));
            let r_dest = alloc_register(&mut next_reg, dest)?;
            bytes.push(encode_xch(r_dest));
            env.insert(dest.to_string(), r_dest);
            acc_owner = None;
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    if bytes.is_empty() {
        bytes.extend_from_slice(&HALT_LOOP);
    }
    Ok(bytes)
}

// ===========================================================================
// Helpers
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
        bytes.push(encode_xch(r));
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

/// Decode and range-check a `const_*` immediate into a 4-bit value.
/// Accepts `[0, 15]` directly and `[-8, -1]` via two's-complement
/// reinterpretation (matching the deprecated iir-to-intel4004's
/// behaviour).
fn encode_immediate_nibble(op: Option<&CIROperand>) -> Result<u8, BackendError> {
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
    if (0..=15).contains(&n) {
        Ok(n as u8)
    } else if (-8..0).contains(&n) {
        Ok((n & 0xF) as u8)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

// ===========================================================================
// Backend trait impl
// ===========================================================================

impl Backend for Intel4004Backend {
    fn name(&self) -> &str {
        "intel4004"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!(
            "intel4004 backend is emit-only; load bytes into an Intel 4004 simulator to execute.  \
             See code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md."
        );
    }
}
