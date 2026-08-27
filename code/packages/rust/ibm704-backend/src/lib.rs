//! # `ibm704-backend` — IBM 704 backend for jit-core / aot-core.
//!
//! L4 of the McCarthy Lisp implementation — closes the round-trip
//! from McCarthy Lisp source back to the silicon Lisp was born on
//! (IBM 704, MIT, 1959).  See
//! [`MCCARTHY-LISP-PLAN.md`](../../../specs/MCCARTHY-LISP-PLAN.md).
//!
//! Mirror of `ge225-backend` / `intel4004-backend` /
//! `armv7-backend` / `intel8008-backend` / `riscv-backend`
//! in shape, just for the 36-bit IBM 704.
//!
//! ## v0.2.0 scope — executable minimal output
//!
//! This backend handles only:
//!
//! | CIR family | Status |
//! |------------|--------|
//! | `const_*` (0–32767, single-var case) | ✓ → `CLA literal_address` |
//! | `ret_*` (value matches the last `const_*` dest) | ✓ → `HTR 0` |
//! | `ret_void` | ✓ → `HTR 0` |
//! | Anything else | returns `None` from `Backend::compile` |
//!
//! The accumulator-tracking pattern matches `intel8008-backend`'s
//! v0.1.0 implementation: a single-tracked "current accumulator
//! var" gets loaded from a sign-magnitude literal pool and read back by `HTR 0` on exit.
//! Multi-var allocation, branches, calls, and CONS are intentionally
//! deferred to future increments.
//!
//! ## Wire format
//!
//! Each instruction or literal is one 36-bit IBM 704 word, packed as five
//! big-endian bytes (the first byte's high nibble is zero).
//! Per-function byte streams concatenate directly — `lang-aot`
//! writes them straight to disk.

use ibm704_encoder::{encode_cla, pack_word, ADDR_MASK, HTR_HALT_BYTES};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::fmt;
use vm_core::value::Value;

/// The IBM 704 backend.  Stateless — every call to `compile` /
/// `compile_function` constructs a fresh per-function lowering.
#[derive(Debug, Default, Clone, Copy)]
pub struct Ibm704Backend;

impl Ibm704Backend {
    pub fn new() -> Self {
        Ibm704Backend
    }
}

/// Errors `ibm704-backend` reports.
///
/// `Backend::compile` collapses these to `None`; the inherent
/// `compile` function returns them as-is so `lang-aot` can include
/// the message in its `Ibm704BackendError` variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    /// CIR op the v0.1.0 backend doesn't yet handle.
    UnsupportedOp(String),
    /// `ret` of a value that isn't the current single-tracked var.
    InvalidOperand(String),
    /// `const_*` with an immediate outside the 15-bit `CLA Y`
    /// address-field range `[0, 32767]`.
    ImmediateOutOfRange(i64),
    /// Instructions plus literal-pool words exceed 32K-word memory.
    ProgramTooLarge(usize),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOp(op) => write!(f, "ibm704-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "ibm704-backend: invalid operand: {d}"),
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "ibm704-backend: const {n} exceeds supported sign-magnitude literal range [0, 32767]"
            ),
            Self::ProgramTooLarge(words) => write!(
                f,
                "ibm704-backend: program requires {words} words, exceeding 32768-word memory"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Lower a single function's CIR to IBM 704 bytes.  Top-level API
/// `lang-aot` calls.
pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_single_function(cir)
}

fn compile_single_function(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    // Empty body → bare HTR 0.  Same fallback shape every minimal-
    // viable historical-arch backend uses.
    if cir.is_empty() {
        return Ok(HTR_HALT_BYTES.to_vec());
    }

    #[derive(Clone, Copy)]
    enum Operation {
        LoadLiteral(usize),
        Halt,
    }

    let mut operations = Vec::with_capacity(cir.len());
    let mut literals = Vec::new();
    let mut last_const_var: Option<String> = None;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            operations.push(Operation::Halt);
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
            operations.push(Operation::Halt);
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?.to_string();
            let imm15 = encode_immediate_15(instr.srcs.first())?;
            let literal_index = literals.len();
            literals.push(imm15);
            operations.push(Operation::LoadLiteral(literal_index));
            last_const_var = Some(dest);
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    if operations.is_empty() {
        operations.push(Operation::Halt);
    }

    let total_words = operations
        .len()
        .checked_add(literals.len())
        .ok_or(BackendError::ProgramTooLarge(usize::MAX))?;
    if total_words > ADDR_MASK as usize + 1 {
        return Err(BackendError::ProgramTooLarge(total_words));
    }

    let literal_base = operations.len();
    let mut bytes = Vec::with_capacity(total_words * HTR_HALT_BYTES.len());
    for operation in operations {
        match operation {
            Operation::LoadLiteral(index) => {
                let address = literal_base + index;
                bytes.extend_from_slice(&pack_word(encode_cla(address as u16)));
            }
            Operation::Halt => bytes.extend_from_slice(&HTR_HALT_BYTES),
        }
    }
    for literal in literals {
        bytes.extend_from_slice(&pack_word(literal as u64));
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

/// Supported non-negative sign-magnitude literal range: `[0, 32767]`.
/// `compile_single_function` places the returned value in the literal pool and
/// addresses that word with `CLA`; the value never occupies CLA's address bits.
fn encode_immediate_15(op: Option<&CIROperand>) -> Result<u16, BackendError> {
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
    let max = ADDR_MASK as i64;
    if (0..=max).contains(&n) {
        Ok(n as u16)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

// ===========================================================================
// Backend trait impl
// ===========================================================================

impl Backend for Ibm704Backend {
    fn name(&self) -> &str {
        "ibm704"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!(
            "ibm704 backend is emit-only; load 5-byte-per-word output into an \
             IBM 704 simulator (or wire `Backend::run` to a future \
             ibm704-simulator).  See code/specs/MCCARTHY-LISP-PLAN.md."
        );
    }
}
