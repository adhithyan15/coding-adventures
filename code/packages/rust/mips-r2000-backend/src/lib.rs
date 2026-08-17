//! # `mips-r2000-backend` — MIPS R2000 backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into MIPS R2000 machine code via
//! [`mips_r2000_encoder`].  Output is `Vec<u8>` (big-endian-encoded R2000
//! words — MIPS R2000's default byte order) so callers can write it
//! straight to a `.bin` file.
//!
//! Mirror of [`armv7_backend`] / [`intel8008_backend`] in shape.  First
//! lane of the 9-architecture expansion following the pattern documented
//! in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## Scope (v0.1.0 — minimal viable)
//!
//! Minimal viable backend — covers the trivial-ROM case (`const_*`
//! immediate + `ret_*`) needed by the `lang-aot` MIPS R2000 e2e smoke
//! test:
//!
//! | CIR op | Lowering |
//! |--------|----------|
//! | `const_*` (16-bit signed imm) | `ADDIU $v0, $zero, imm` |
//! | `ret_*`, `ret_void` | `JR $ra` |
//! | Anything else | returns `None` |
//!
//! There is no real register allocator: a trivial "last const var" scheme
//! tracks which single variable the most recent `const_*` wrote, and
//! `ret_*` only succeeds if it returns exactly that variable — the same
//! scheme `armv7-backend`/`intel8008-backend` use.  Full op coverage
//! (add/sub/cmp/branches/calls) is intentionally **not** ported here;
//! future increments to this crate can add them.
//!
//! Per the migration spec, this is acceptable: the architectural
//! correctness win (IIR → CIR via `Backend` trait) is delivered as soon
//! as the AOT path is wired, regardless of op-set parity.
//!
//! ## Why is `Backend::run` not implemented?
//!
//! Emit-only target per the migration spec.  Bytes go to
//! `mips-r2000-simulator`.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use mips_r2000_encoder::{encode_addiu, RET_WORD, V0, ZERO};
use std::fmt;
use vm_core::value::Value;

#[derive(Debug, Default, Clone, Copy)]
pub struct MipsR2000Backend;

impl MipsR2000Backend {
    pub fn new() -> Self {
        MipsR2000Backend
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
            Self::UnsupportedOp(op) => write!(f, "mips-r2000-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "mips-r2000-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "mips-r2000-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "mips-r2000-backend: const {n} exceeds 16-bit signed ADDIU immediate range \
                 [-32768, 32767]; wider values require lui+ori pairs"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Compile a single function's CIR into MIPS R2000 bytes (big-endian
/// R2000 words flattened to `u8`).
pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_single_function(cir)
}

fn compile_single_function(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    if cir.is_empty() {
        // Empty CIR → just JR $ra so the program returns immediately.
        return Ok(RET_WORD.to_be_bytes().to_vec());
    }

    let mut bytes = Vec::new();
    // v0.1.0 uses a trivial single-register allocator: the most recent
    // `const_*` puts its value into $v0, and `ret_*` returns $v0.
    // Programs that need more than one live var fall through to
    // `UnsupportedOp` (returned as `None` from the Backend trait).
    let mut last_const_var: Option<String> = None;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            bytes.extend_from_slice(&RET_WORD.to_be_bytes());
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let src_name = parse_var_src(instr, 0, op)?;
            // We only support the case where src is the most recent
            // const'd var (i.e. it's already in $v0).  Multi-var
            // requires a real register allocator.
            if last_const_var.as_deref() != Some(src_name.as_str()) {
                return Err(BackendError::UnsupportedOp(format!(
                    "ret of {src_name:?} which is not the current $v0 var; \
                     multi-register allocation lands in a future increment"
                )));
            }
            bytes.extend_from_slice(&RET_WORD.to_be_bytes());
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let imm16 = encode_immediate_16(instr.srcs.first())?;
            // const_* always targets $v0 in this minimal backend.
            bytes.extend_from_slice(&encode_addiu(V0, ZERO, imm16 as i32).to_be_bytes());
            last_const_var = Some(dest.to_string());
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    // Defensive — if no terminator was emitted, append JR $ra so the
    // program returns instead of running off the end.
    if bytes.len() < 4 || bytes[bytes.len() - 4..] != RET_WORD.to_be_bytes() {
        bytes.extend_from_slice(&RET_WORD.to_be_bytes());
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

/// `ADDIU`'s immediate field is 16 bits, sign-extended.  Accept the
/// signed range `[-32768, 32767]`; wider constants need a `lui`+`ori`
/// pair, which is out of scope for the minimal-viable backend.
fn encode_immediate_16(op: Option<&CIROperand>) -> Result<i16, BackendError> {
    let n: i64 = match op {
        Some(CIROperand::Int(n)) => *n,
        Some(CIROperand::Bool(b)) => i64::from(*b),
        _ => {
            return Err(BackendError::InvalidOperand(
                "const_* srcs[0] must be Int or Bool".into(),
            ));
        }
    };
    if (-32768..=32767).contains(&n) {
        Ok(n as i16)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

impl Backend for MipsR2000Backend {
    fn name(&self) -> &str {
        "mips-r2000"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!(
            "mips-r2000 backend is emit-only; load bytes into mips-r2000-simulator to execute"
        );
    }
}
