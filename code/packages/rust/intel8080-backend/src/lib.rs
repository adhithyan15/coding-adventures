//! # `intel8080-backend` — Intel 8080 backend for jit-core / aot-core.
//!
//! Third lane of the 9-architecture expansion. Mirror of
//! `intel8008-backend` in shape — the 8080 is the 8008's direct
//! architectural successor: still an 8-bit accumulator machine with a
//! real `HLT` opcode, so the "const → accumulator, ret → HLT" backend
//! shape maps almost directly (unlike MIPS/ARM1, which needed different
//! return-mechanism handling).
//!
//! ## v0.1.0 scope — minimal viable
//!
//! Same scope as `intel8008-backend` v0.1.0: just enough to compile the
//! trivial IIR program `const 42; ret` to real Intel 8080 machine code
//! bytes, byte-for-byte (`MVI A, 42; HLT` = `[0x3E, 0x2A, 0x76]`).
//!
//! | CIR family | Status |
//! |------------|--------|
//! | `const_*` (8-bit immediate, single-var case) | ✓ → `MVI A, n` |
//! | `ret_*`, `ret_void` | ✓ → `HLT` (entry-function exit) |
//! | Anything else | returns `None` |
//!
//! Per the GUIDING CONSTRAINT (see
//! `code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`), the architectural
//! correctness win (IIR → CIR via the `Backend` trait) is delivered
//! regardless of op-set parity. Future increments to `intel8080-backend`
//! can port richer op coverage (mov/add/sub/cmp/branches/calls) using the
//! full ISA `intel8080-simulator` already implements.

use intel8080_encoder::{encode_mvi_a, HLT, MVI_MAX};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::fmt;
use vm_core::value::Value;

#[derive(Debug, Default, Clone, Copy)]
pub struct Intel8080Backend;

impl Intel8080Backend {
    pub fn new() -> Self {
        Intel8080Backend
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
            Self::UnsupportedOp(op) => write!(f, "intel8080-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "intel8080-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "intel8080-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "intel8080-backend: const {n} exceeds 8-bit MVI immediate range [0, 255]"
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

impl Backend for Intel8080Backend {
    fn name(&self) -> &str {
        "intel8080"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!(
            "intel8080 backend is emit-only; load bytes into intel8080-simulator to execute"
        );
    }
}
