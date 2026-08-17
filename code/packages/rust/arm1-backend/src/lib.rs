//! # `arm1-backend` — ARM1 (ARMv1) backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into ARM1 machine code via
//! [`arm1_encoder`].  Output is `Vec<u8>` (little-endian-encoded
//! ARM1 words — ARM1's byte order, matching
//! `arm1_simulator::ARM1::read_word`/`write_word`) so callers can
//! write it straight to a `.bin` file.
//!
//! Mirror of [`mips_r2000_backend`] / [`armv7_backend`] /
//! [`intel8008_backend`] in shape.  Second lane of the
//! 9-architecture expansion following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## Scope (v0.1.0 — minimal viable)
//!
//! Minimal viable backend — covers the trivial-ROM case (`const_*`
//! immediate + `ret_*`) needed by the `lang-aot` ARM1 e2e smoke
//! test:
//!
//! | CIR op | Lowering |
//! |--------|----------|
//! | `const_*` (8-bit unrotated imm) | `MOV R0, #imm` |
//! | `ret_*`, `ret_void` | pseudo-halt `SWI #0x123456` (see below) |
//! | Anything else | returns `None` |
//!
//! There is no real register allocator: a trivial "last const var"
//! scheme tracks which single variable the most recent `const_*`
//! wrote, and `ret_*` only succeeds if it returns exactly that
//! variable — the same scheme `mips-r2000-backend`/
//! `armv7-backend`/`intel8008-backend` use.  Full op coverage
//! (add/sub/cmp/branches/calls) is intentionally **not** ported
//! here; future increments to this crate can add them, including a
//! real allocator over ARM1's other 14 general-purpose registers.
//!
//! Per the migration spec, this is acceptable: the architectural
//! correctness win (IIR → CIR via `Backend` trait) is delivered as
//! soon as the AOT path is wired, regardless of op-set parity.
//!
//! ## Why does `ret_*` lower to a pseudo-halt, not `BX LR`?
//!
//! ARMv7's `BX LR` return-from-function convention doesn't exist in
//! ARMv1 (1985) — there is no `BX` instruction, and the era's
//! subroutine-return idiom, `MOVS PC, R14`, requires a live `R14`
//! that only a preceding `BL` sets up (i.e. it needs a *caller*).
//! The minimal-viable `const_*`/`ret_*` scope compiles a whole
//! program's worth of CIR with no caller in the picture — the
//! trivial ROM just needs to compute a value and stop.
//!
//! `arm1-simulator` already defines exactly this: a pseudo-halt
//! instruction, `SWI #0x123456` (`arm1_simulator::HALT_SWI`), that
//! its `execute_swi` intercepts specially — when the SWI's 24-bit
//! comment field equals `HALT_SWI`, the simulator sets its internal
//! `halted` flag (observable via `ARM1::halted()`) instead of
//! entering Supervisor mode like a genuine SWI would.  This is a
//! simulator-level halt convention (parallel to the Intel 8008
//! backend's `HLT` byte or the GE-225 backend's `HLT` word), not
//! real ARM1 silicon behaviour.  Lowering `ret_*`/`ret_void` to this
//! pseudo-halt is the semantically correct choice here: it is the
//! only instruction that actually stops the fetch-decode-execute
//! loop, leaving the computed value in `R0` for the caller to read
//! via `read_register(0)` — see `arm1_encoder`'s crate-level doc
//! comment for the full derivation.
//!
//! ## Why is `Backend::run` not implemented?
//!
//! Emit-only target per the migration spec.  Bytes go to
//! `arm1-simulator`.

use arm1_encoder::{encode_halt, encode_mov_imm, COND_AL, HALT_WORD, R0};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::fmt;
use vm_core::value::Value;

#[derive(Debug, Default, Clone, Copy)]
pub struct Arm1Backend;

impl Arm1Backend {
    pub fn new() -> Self {
        Arm1Backend
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
            Self::UnsupportedOp(op) => write!(f, "arm1-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "arm1-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "arm1-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "arm1-backend: const {n} exceeds 8-bit MOV-immediate range [0, 255]; \
                 wider values require the barrel shifter's rotated-immediate form, \
                 which is out of scope for the minimal-viable backend"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Compile a single function's CIR into ARM1 bytes (little-endian
/// ARM1 words flattened to `u8`).
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
        // Empty CIR → just the pseudo-halt so the program stops
        // immediately.
        return Ok(vec![encode_halt()]);
    }

    let mut words = Vec::new();
    // v0.1.0 uses a trivial single-register allocator: the most
    // recent `const_*` puts its value into R0, and `ret_*` returns
    // R0.  Programs that need more than one live var fall through
    // to `UnsupportedOp` (returned as `None` from the Backend
    // trait).
    let mut last_const_var: Option<String> = None;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            words.push(encode_halt());
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let src_name = parse_var_src(instr, 0, op)?;
            // We only support the case where src is the most recent
            // const'd var (i.e. it's already in R0).  Multi-var
            // requires a real register allocator.
            if last_const_var.as_deref() != Some(src_name.as_str()) {
                return Err(BackendError::UnsupportedOp(format!(
                    "ret of {src_name:?} which is not the current R0 var; \
                     multi-register allocation lands in a future increment"
                )));
            }
            words.push(encode_halt());
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let imm8 = encode_immediate_8(instr.srcs.first())?;
            // const_* always targets R0 in this minimal backend.
            words.push(encode_mov_imm(COND_AL, R0, imm8 as u32));
            last_const_var = Some(dest.to_string());
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    // Defensive — if no terminator was emitted, append the
    // pseudo-halt so the program stops instead of running off the
    // end (which would read zeroed memory as instructions).
    if words.last() != Some(&HALT_WORD) {
        words.push(encode_halt());
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

/// ARM1's `MOV Rd, #imm8` (as `arm1_encoder::encode_mov_imm` wraps
/// it) carries an unrotated 8-bit immediate — no barrel-shifter
/// rotation is applied in this minimal-viable backend.  Accept the
/// unsigned range `[0, 255]`; wider or negative constants need the
/// rotated-immediate form (or `MVN`), which is out of scope here.
fn encode_immediate_8(op: Option<&CIROperand>) -> Result<u8, BackendError> {
    let n: i64 = match op {
        Some(CIROperand::Int(n)) => *n,
        Some(CIROperand::Bool(b)) => i64::from(*b),
        _ => {
            return Err(BackendError::InvalidOperand(
                "const_* srcs[0] must be Int or Bool".into(),
            ));
        }
    };
    if (0..=255).contains(&n) {
        Ok(n as u8)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

impl Backend for Arm1Backend {
    fn name(&self) -> &str {
        "arm1"
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
        panic!("arm1 backend is emit-only; load bytes into arm1-simulator to execute");
    }
}
