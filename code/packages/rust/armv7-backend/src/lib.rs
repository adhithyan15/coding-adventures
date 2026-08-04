//! # `armv7-backend` — ARMv7-A (A32) backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into ARMv7 machine code via
//! [`armv7_encoder`].  Output is `Vec<u8>` (little-endian-encoded
//! A32 words) so callers can write it straight to a `.bin` file.
//!
//! Mirror of [`ge225-backend`] / [`intel4004-backend`] in shape.
//!
//! ## Scope (v0.1.0 — Phase 5 of the historical-arch migration)
//!
//! Minimal viable migration — covers the AAPCS32-conforming
//! trivial-ROM case (`const_*` immediate + `ret_*`) needed by
//! the lang-aot ARMv7 e2e smoke test:
//!
//! | CIR op | Lowering |
//! |--------|----------|
//! | `const_*` (8-bit imm) | `MOV r0, #imm` |
//! | `ret_*`, `ret_void` | `BX LR` |
//! | Anything else | returns `None` |
//!
//! The full op coverage that the deprecated `iir-to-armv7` v0.4.6
//! had (add/sub/and/or/xor/adc/sbb/cmp/branches/calls) is **not**
//! ported here — those landed in `iir-to-armv7` over many
//! increments and would balloon this PR.  Future increments to
//! this crate can add them; until then, larger CIR programs
//! fall through to `None` and the AOT pipeline reports a graceful
//! compile failure.
//!
//! Per the migration spec, this is acceptable: the architectural
//! correctness win (IIR → CIR via Backend trait) is delivered as
//! soon as the AOT path is wired, regardless of op-set parity.
//!
//! ## Why is `Backend::run` not implemented?
//!
//! Emit-only target per the migration spec.  Bytes go to
//! `arm-simulator`, `qemu-arm`, or a real Cortex-A class SoC.

use armv7_encoder::{encode_mov_imm, BX_LR, MOV_IMM_MAX};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::collections::HashMap;
use std::fmt;
use vm_core::value::Value;

#[derive(Debug, Default, Clone, Copy)]
pub struct Armv7Backend;

impl Armv7Backend {
    pub fn new() -> Self {
        Armv7Backend
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
            Self::UnsupportedOp(op) => write!(f, "armv7-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "armv7-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "armv7-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "armv7-backend: const {n} exceeds 8-bit MOV-immediate range [0, 255]; \
                 wider values require movw/movt or rotated immediates"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Compile a single function's CIR into ARMv7 bytes (little-endian
/// A32 words flattened to `u8`).
pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    let words = compile_to_words(cir)?;
    let mut bytes = Vec::with_capacity(words.len() * 4);
    for w in &words {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    Ok(bytes)
}

fn compile_to_words(cir: &[CIRInstr]) -> Result<Vec<u32>, BackendError> {
    if cir.is_empty() {
        // Empty CIR → just BX LR so the program returns immediately.
        return Ok(vec![BX_LR]);
    }

    let mut words = Vec::new();
    // For v0.1.0 we use a trivial single-register allocator: the
    // most recent `const_*` puts its value into r0, and `ret_*`
    // returns r0.  Programs that need more than one live var fall
    // through to `UnsupportedOp` (returned as `None` from the
    // Backend trait).
    let mut last_const_var: Option<String> = None;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            words.push(BX_LR);
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let src_name = parse_var_src(instr, 0, op)?;
            // We only support the case where src is the most
            // recent const'd var (i.e. it's already in r0).
            // Multi-var requires a real register allocator.
            if last_const_var.as_deref() != Some(src_name.as_str()) {
                return Err(BackendError::UnsupportedOp(format!(
                    "ret of {src_name:?} which is not the current r0 var; \
                     multi-register allocation lands in a future increment"
                )));
            }
            words.push(BX_LR);
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let imm8 = encode_immediate_8(instr.srcs.first())?;
            // const_* always targets r0 in this minimal backend.
            words.push(encode_mov_imm(0, imm8));
            last_const_var = Some(dest.to_string());
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    // Defensive — if no terminator was emitted, append BX LR so
    // the program returns instead of running off the end.
    if words.last().is_none_or(|&w| w != BX_LR) {
        words.push(BX_LR);
    }
    Ok(words)
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
    if (0..=MOV_IMM_MAX as i64).contains(&n) {
        Ok(n as u8)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

// Reserve `HashMap` import so the module pre-imports it for the
// allocator state we'll add in future increments.  Suppress the
// dead-code warning for now.
#[allow(dead_code)]
fn _force_hashmap_import() -> HashMap<String, u8> {
    HashMap::new()
}

impl Backend for Armv7Backend {
    fn name(&self) -> &str {
        "armv7"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_to_words(ir)
            .ok()
            .map(|words| words.iter().flat_map(|w| w.to_le_bytes()).collect())
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!(
            "armv7 backend is emit-only; load bytes into an ARMv7 simulator, qemu-arm, or \
             objcopy + a phone-class Linux linker to execute.  \
             See code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md."
        );
    }
}
