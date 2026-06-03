//! # `intel8008-backend` — Intel 8008 backend for jit-core / aot-core.
//!
//! Phase 6 of the historical-arch backend migration.  Mirror of
//! `ge225-backend` / `intel4004-backend` / `armv7-backend` in
//! shape, just for the 8-bit Intel 8008 (1972) — the second-
//! generation Intel microprocessor, Oct's native target.
//!
//! ## v0.1.0 scope — minimal viable
//!
//! Same scope as `armv7-backend` v0.1.0: just enough to keep the
//! existing lang-aot Intel 8008 e2e smoke test passing
//! byte-for-byte (Twig `42` → `MVI A, 42; HLT` =
//! `[0x3E, 0x2A, 0x76]`).
//!
//! | CIR family | Status |
//! |------------|--------|
//! | `const_*` (8-bit immediate, single-var case) | ✓ → `MVI A, n` |
//! | `ret_*`, `ret_void` | ✓ → `HLT` (entry-function exit) |
//! | Anything else | returns `None` |
//!
//! Per the GUIDING CONSTRAINT, the architectural correctness win
//! (IIR → CIR via Backend trait) is delivered regardless of
//! op-set parity.  Future increments to `intel8008-backend` can
//! port the richer op coverage that `iir-to-intel8008` v0.3.9
//! had (mov/add/sub/cmp/branches/calls).

use intel8008_encoder::{encode_mvi_a, HLT, MVI_MAX};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::fmt;
use vm_core::value::Value;

#[derive(Debug, Default, Clone, Copy)]
pub struct Intel8008Backend;

impl Intel8008Backend {
    pub fn new() -> Self {
        Intel8008Backend
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    UnsupportedOp(String),
    InvalidOperand(String),
    UndefinedVariable(String),
    ImmediateOutOfRange(i64),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOp(op) => write!(f, "intel8008-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "intel8008-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "intel8008-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "intel8008-backend: const {n} exceeds 8-bit MVI immediate range [0, 255]"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_single_function(cir)
}

fn compile_single_function(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    if cir.is_empty() {
        return Ok(vec![HLT]);
    }

    let mut bytes = Vec::new();
    let mut last_const_var: Option<String> = None;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            bytes.push(HLT);
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let src_name = parse_var_src(instr, 0, op)?;
            if last_const_var.as_deref() != Some(src_name.as_str()) {
                return Err(BackendError::UnsupportedOp(format!(
                    "ret of {src_name:?} which is not the current accumulator var; \
                     multi-register allocation lands in a future increment"
                )));
            }
            bytes.push(HLT);
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let imm8 = encode_immediate_8(instr.srcs.first())?;
            bytes.extend_from_slice(&encode_mvi_a(imm8));
            last_const_var = Some(dest.to_string());
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    if bytes.is_empty() {
        bytes.push(HLT);
    }
    Ok(bytes)
}

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

fn encode_immediate_8(op: Option<&CIROperand>) -> Result<u8, BackendError> {
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
    if (0..=MVI_MAX as i64).contains(&n) {
        Ok(n as u8)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

impl Backend for Intel8008Backend {
    fn name(&self) -> &str {
        "intel8008"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!(
            "intel8008 backend is emit-only; load bytes into an Intel 8008 simulator \
             or burn to a 1702 EPROM to execute.  \
             See code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md."
        );
    }
}
