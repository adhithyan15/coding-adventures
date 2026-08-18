//! # `sparc-v8-backend` — SPARC V8 backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into SPARC V8 machine code via
//! [`sparc_v8_encoder`].  Output is `Vec<u8>` (big-endian-encoded
//! SPARC V8 words — SPARC's default byte order, same as MIPS R2000)
//! so callers can write it straight to a `.bin` file.
//!
//! Mirror of [`mips_r2000_backend`] / [`arm1_backend`] in shape.
//! Sixth lane of the 9-architecture expansion following the pattern
//! documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## Scope (v0.1.0 — minimal viable)
//!
//! Minimal viable backend — covers the trivial-ROM case (`const_*`
//! immediate + `ret_*`) needed by the `lang-aot` SPARC V8 e2e smoke
//! test:
//!
//! | CIR op | Lowering |
//! |--------|----------|
//! | `const_*` (13-bit signed imm) | `ADD %g0, imm, %o0` |
//! | `ret_*`, `ret_void` | `ta 0` (HALT) |
//! | Anything else | returns `None` |
//!
//! There is no real register allocator: a trivial "last const var"
//! scheme tracks which single variable the most recent `const_*` wrote,
//! and `ret_*` only succeeds if it returns exactly that variable — the
//! same scheme `mips-r2000-backend`/`arm1-backend` use.  Full op
//! coverage (add/sub/cmp/branches/calls) is intentionally **not**
//! ported here; future increments to this crate can add them.
//!
//! Per the migration spec, this is acceptable: the architectural
//! correctness win (IIR → CIR via `Backend` trait) is delivered as
//! soon as the AOT path is wired, regardless of op-set parity.
//!
//! ## Why `%o0`, not a `%g` register, for the return value?
//!
//! `%o0` is the real SPARC ABI's integer return-value register (see
//! `sparc_v8_encoder`'s crate-level doc comment for the full
//! derivation and the SPARC V8 manual citation).  `%o0` is a windowed
//! register, but this backend's CIR lowering never emits `SAVE`/
//! `RESTORE`, so the Current Window Pointer never moves for the
//! lifetime of a compiled program — `%o0` therefore always resolves
//! to the same fixed physical register, with none of the
//! window-rotation complexity real SPARC calling-convention code has
//! to reason about across call boundaries.
//!
//! ## Register-window scoping — deferred, not stubbed-broken
//!
//! `sparc-v8-simulator` (one layer down) implements `SAVE`/`RESTORE`
//! and the full windowed-register-file machinery completely and
//! correctly — see that crate's docs.  This backend simply never emits
//! them: v0.1.0's CIR lowering only ever touches `%g0` and `%o0`,
//! which resolve identically regardless of CWP as long as no `SAVE`/
//! `RESTORE` executes.  A future increment adding real function calls
//! would need a register allocator that emits `SAVE` at function entry
//! and `RESTORE` at `ret_*` — the simulator underneath is already
//! ready for that; only this backend's CIR-to-word lowering needs to
//! grow.
//!
//! ## Why `ret_*` lowers to `ta 0`, not `RESTORE` + `JMPL`
//!
//! A real SPARC subroutine returns via `RESTORE %g0, %g0, %g0` (undo
//! the register window) followed by `JMPL %i7+8, %g0` (return to the
//! caller, skipping the two-instruction CALL-annotation slot).  Both
//! require a live caller context (a `%i7` set by a preceding `CALL`)
//! that the minimal-viable `const_*`/`ret_*` scope never establishes —
//! there is no caller for a trivial ROM.  `sparc-v8-simulator` already
//! defines exactly the right primitive for "the program is done":
//! `ta 0` (trap always, software trap #0), which its executor
//! intercepts to set `halted() == true` and stop the
//! fetch-decode-execute loop, leaving the computed value in `%o0` for
//! the caller to read.  This mirrors `mips-r2000-backend`'s `JR $ra`-
//! as-`ret` choice is NOT taken here for the same reason `arm1-backend`
//! didn't use `MOVS PC, R14`: no caller context exists.  `ta 0` plays
//! the same role `arm1-backend`'s pseudo-halt `SWI #0x123456` plays for
//! ARM1 — a simulator-level halt convention documented in the existing
//! Python `sparc-v8-simulator` reference (`state.py`'s `HALT_WORD`),
//! not invented for this backend.
//!
//! ## Why is `Backend::run` not implemented?
//!
//! Emit-only target per the migration spec.  Bytes go to
//! `sparc-v8-simulator`.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use sparc_v8_encoder::{encode_add_imm, G0, HALT_WORD, O0};
use std::fmt;
use vm_core::value::Value;

#[derive(Debug, Default, Clone, Copy)]
pub struct SparcV8Backend;

impl SparcV8Backend {
    pub fn new() -> Self {
        SparcV8Backend
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
            Self::UnsupportedOp(op) => write!(f, "sparc-v8-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "sparc-v8-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "sparc-v8-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "sparc-v8-backend: const {n} exceeds 13-bit signed ADD immediate range \
                 [-4096, 4095]; wider values require a SETHI+ADD/OR pair"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Compile a single function's CIR into SPARC V8 bytes (big-endian
/// SPARC words flattened to `u8`).
pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_single_function(cir)
}

fn compile_single_function(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    if cir.is_empty() {
        // Empty CIR → just `ta 0` so the program halts immediately.
        return Ok(HALT_WORD.to_be_bytes().to_vec());
    }

    let mut bytes = Vec::new();
    // v0.1.0 uses a trivial single-register allocator: the most recent
    // `const_*` puts its value into %o0, and `ret_*` returns %o0.
    // Programs that need more than one live var fall through to
    // `UnsupportedOp` (returned as `None` from the Backend trait).
    let mut last_const_var: Option<String> = None;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            bytes.extend_from_slice(&HALT_WORD.to_be_bytes());
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let src_name = parse_var_src(instr, 0, op)?;
            // We only support the case where src is the most recent
            // const'd var (i.e. it's already in %o0).  Multi-var
            // requires a real register allocator.
            if last_const_var.as_deref() != Some(src_name.as_str()) {
                return Err(BackendError::UnsupportedOp(format!(
                    "ret of {src_name:?} which is not the current %o0 var; \
                     multi-register allocation lands in a future increment"
                )));
            }
            bytes.extend_from_slice(&HALT_WORD.to_be_bytes());
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let imm13 = encode_immediate_13(instr.srcs.first())?;
            // const_* always targets %o0 in this minimal backend.
            bytes.extend_from_slice(&encode_add_imm(O0, G0, imm13 as i32).to_be_bytes());
            last_const_var = Some(dest.to_string());
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    // Defensive — if no terminator was emitted, append `ta 0` so the
    // program halts instead of running off the end.
    if bytes.len() < 4 || bytes[bytes.len() - 4..] != HALT_WORD.to_be_bytes() {
        bytes.extend_from_slice(&HALT_WORD.to_be_bytes());
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

/// `ADD`'s immediate field is a sign-extended 13-bit value (SPARC V8
/// Format 3i's `simm13`).  Accept the signed range `[-4096, 4095]`;
/// wider constants need a `SETHI`+`ADD`/`OR` pair, which is out of
/// scope for the minimal-viable backend.
fn encode_immediate_13(op: Option<&CIROperand>) -> Result<i16, BackendError> {
    let n: i64 = match op {
        Some(CIROperand::Int(n)) => *n,
        Some(CIROperand::Bool(b)) => i64::from(*b),
        _ => {
            return Err(BackendError::InvalidOperand(
                "const_* srcs[0] must be Int or Bool".into(),
            ));
        }
    };
    if (-4096..=4095).contains(&n) {
        Ok(n as i16)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

impl Backend for SparcV8Backend {
    fn name(&self) -> &str {
        "sparc-v8"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!(
            "sparc-v8 backend is emit-only; load bytes into sparc-v8-simulator to execute"
        );
    }
}
