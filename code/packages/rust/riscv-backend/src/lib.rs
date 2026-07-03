//! # `riscv-backend` — RV32I backend for jit-core / aot-core.
//!
//! Phase 7 (the FINAL lane) of the historical-arch backend
//! migration.  Mirror of [`ge225-backend`] / [`intel4004-backend`] /
//! [`armv7-backend`] / [`intel8008-backend`] in shape — but for
//! the 32-bit RISC-V (RV32I) base ISA.
//!
//! ## Why "FINAL" lane?
//!
//! The RV32I backend was the **original** historical-arch target
//! (the A1+ cascade in May 2026).  It shipped at the wrong layer
//! — `iir-to-riscv` consumed dynamic IIR directly — which kicked
//! off the architectural correctness migration documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`].  Phase 7 closes the
//! loop: RV32I now consumes typed CIR via the [`Backend`] trait,
//! the same way every other arch backend does.
//!
//! ## v0.1.0 scope — minimal viable
//!
//! Same scope as `intel8008-backend` v0.1.0 and `armv7-backend`
//! v0.1.0: just enough to keep the existing lang-aot RV32I e2e
//! smoke tests passing byte-for-byte.
//!
//! | CIR family | Status |
//! |------------|--------|
//! | `const_*` (12-bit signed immediate, single-var case) | ✓ → `addi rd, x0, n` |
//! | `ret_*` (value match the last `const_*` dest) | ✓ → `addi a0, rs, 0` + `jalr x0, x1, 0` |
//! | `ret_void` | ✓ → `jalr x0, x1, 0` |
//! | Anything else | returns `None` from `Backend::compile` |
//!
//! Per the GUIDING CONSTRAINT, minimal viable op coverage is
//! acceptable for the historical arches.  Future increments can
//! port the richer ops `iir-to-riscv` v0.3.3 had (add/sub/cmp/
//! branches/calls/ecall print_i64).
//!
//! ## Wire format
//!
//! Each emitted instruction is a 32-bit RV32I word, flattened to
//! little-endian bytes per the RISC-V spec.  Per-function byte
//! streams can be concatenated directly — `lang-aot` writes them
//! straight to disk as a flat `.bin`.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use riscv_encoder::{encode_addi, A0, RET_WORD, TEMP_REGISTERS, X0_ZERO};
use std::fmt;
use vm_core::value::Value;

/// The RV32I backend.  Stateless — every call to `compile` /
/// `compile_function` constructs a fresh per-function lowering.
#[derive(Debug, Default, Clone, Copy)]
pub struct Riscv32Backend;

impl Riscv32Backend {
    pub fn new() -> Self {
        Riscv32Backend
    }
}

/// Errors `riscv-backend` reports.
///
/// `compile` (the trait method) collapses these to `None`; the
/// inherent `compile` function returns them as-is so `lang-aot`
/// can include the message in `RiscvBackendError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    /// CIR op the v0.1.0 backend doesn't yet handle.
    UnsupportedOp(String),
    /// `ret` of a value that isn't the current single-tracked var.
    InvalidOperand(String),
    /// Reference to a name we haven't seen via `const_*`.
    UndefinedVariable(String),
    /// `const_*` with an immediate outside the 12-bit signed
    /// `addi` range `[-2048, 2047]`.
    ImmediateOutOfRange(i64),
    /// Linear allocator ran out of temporary registers.
    OutOfRegisters,
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOp(op) => write!(f, "riscv-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "riscv-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "riscv-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "riscv-backend: const {n} exceeds 12-bit signed addi immediate range \
                 [-2048, 2047]"
            ),
            Self::OutOfRegisters => write!(
                f,
                "riscv-backend: temp-register pool exhausted (max 7 simultaneous vars); \
                 stack-spilling support lands in a future increment"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Lower a single function's CIR to RV32I bytes.  Top-level API
/// `lang-aot` calls.
pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_single_function(cir)
}

fn compile_single_function(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    // Empty CIR → bare `jalr x0, x1, 0`.  Same fallback shape as
    // intel8008-backend's bare-HLT case.
    if cir.is_empty() {
        return Ok(RET_WORD.to_le_bytes().to_vec());
    }

    let mut words: Vec<u32> = Vec::new();

    // Per-function linear temp allocator (matches iir-to-riscv
    // v0.3.3's pattern: hand out TEMP_REGISTERS[i] for the i'th
    // distinct var).  We track every `const_*` dest so `ret_*`
    // can mv the right temp into `a0`.
    let mut env: Vec<(String, u32)> = Vec::new();

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            words.push(RET_WORD);
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let src_name = parse_var_src(instr, 0, op)?;
            let src_reg = lookup(&env, &src_name)?;
            // mv a0, src  (encoded as `addi a0, src, 0`).
            words.push(encode_addi(A0, src_reg, 0));
            // jalr x0, x1, 0 (canonical return).
            words.push(RET_WORD);
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?.to_string();
            let imm12 = encode_immediate_12(instr.srcs.first())?;
            let rd = allocate(&mut env, dest)?;
            // addi rd, x0, imm12
            words.push(encode_addi(rd, X0_ZERO, imm12));
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    // Empty CIR fell through above; here we may have a body with
    // no terminator (some IIR shapes don't emit explicit ret).
    // Mirror intel8008-backend's "always end with HLT" guard.
    if words.is_empty() {
        words.push(RET_WORD);
    }

    // Flatten to little-endian bytes — the wire format the
    // lang-aot RV32I .bin path expects.
    let mut bytes = Vec::with_capacity(words.len() * 4);
    for w in &words {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    Ok(bytes)
}

// ===========================================================================
// Per-function allocator helpers
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

/// 12-bit signed `addi` immediate range: `[-2048, 2047]`.
fn encode_immediate_12(op: Option<&CIROperand>) -> Result<i32, BackendError> {
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
    if (-2048..=2047).contains(&n) {
        Ok(n as i32)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

/// Allocate the next free temp register for `name`.  Returns the
/// existing assignment if `name` is already in the env (a re-use
/// — same value, same temp).
fn allocate(env: &mut Vec<(String, u32)>, name: String) -> Result<u32, BackendError> {
    if let Some((_, reg)) = env.iter().find(|(n, _)| *n == name) {
        return Ok(*reg);
    }
    if env.len() >= TEMP_REGISTERS.len() {
        return Err(BackendError::OutOfRegisters);
    }
    let reg = TEMP_REGISTERS[env.len()];
    env.push((name, reg));
    Ok(reg)
}

fn lookup(env: &[(String, u32)], name: &str) -> Result<u32, BackendError> {
    env.iter()
        .find_map(|(n, r)| if n == name { Some(*r) } else { None })
        .ok_or_else(|| BackendError::UndefinedVariable(name.to_string()))
}

// ===========================================================================
// Backend trait impl — plugs into jit-core's registry alongside
// aarch64-backend and x86_64-backend.
// ===========================================================================

impl Backend for Riscv32Backend {
    fn name(&self) -> &str {
        "riscv32"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        // Per the GUIDING CONSTRAINT, the historical-arch backends
        // are emit-only.  Real RV32I execution would forward to
        // `riscv-simulator::Simulator::run` here — a fine future
        // increment but not blocking the migration.
        panic!(
            "riscv32 backend is emit-only; load bytes into the in-tree \
             riscv-simulator, qemu-riscv32, or a flash loader to execute.  \
             See code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md."
        );
    }
}
