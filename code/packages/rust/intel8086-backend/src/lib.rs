//! # `intel8086-backend` — Intel 8086 backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into Intel 8086 machine code via
//! [`intel8086_encoder`]. Output is `Vec<u8>` — the 8086 is byte-oriented
//! at the encoding level (multi-byte immediates are little-endian, but
//! there's no fixed instruction-word width to flatten, unlike
//! `arm1-backend`/`mips-r2000-backend`), so the encoder's `Vec<u8>` bytes
//! are already the wire format.
//!
//! Mirror of [`mos6502_backend`]/[`arm1_backend`]/[`armv7_backend`] in
//! shape. Ninth and **final** lane of the 9-architecture expansion
//! following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## Scope (v0.1.0 — minimal viable)
//!
//! Minimal viable backend — covers the trivial-ROM case (`const_*`
//! immediate + `ret_*`) needed by the `lang-aot` Intel 8086 e2e smoke
//! test:
//!
//! | CIR op | Lowering |
//! |--------|----------|
//! | `const_*` (16-bit unsigned literal, `[0, 65535]`) | `MOV AX, #imm16` |
//! | `ret_*`, `ret_void` | `HLT` (a genuine hardware halt — see below) |
//! | Anything else | returns `None` |
//!
//! There is no real register allocator: a trivial "last const var"
//! scheme tracks which single variable the most recent `const_*` wrote
//! (into the accumulator `AX` — the 8086's primary 16-bit accumulator/
//! return-value register), and `ret_*` only succeeds if it returns
//! exactly that variable — the same scheme `mips-r2000-backend`/
//! `arm1-backend`/`armv7-backend`/`mos6502-backend` use. Full op
//! coverage (arithmetic, register-to-register moves, control flow — all
//! of which `intel8086-simulator` implements for its curated subset) is
//! intentionally **not** wired into this backend yet; future increments
//! can extend `compile_to_bytes` to emit them.
//!
//! Per the migration spec, this is acceptable: the architectural
//! correctness win (IIR → CIR via `Backend` trait) is delivered as soon
//! as the AOT path is wired, regardless of op-set parity.
//!
//! ## Why does `ret_*` lower to real `HLT`, not a pseudo-halt?
//!
//! Unlike ARM1 (1985, no real halt instruction — `arm1-backend` had to
//! invent a pseudo-halt via `SWI #0x123456`) or MOS 6502 (whose `BRK` is
//! technically a software-interrupt opcode that this repo's simulator
//! stack *treats* as HALT by convention), the Intel 8086 has a genuine,
//! single-byte, no-operand hardware instruction whose sole purpose is to
//! stop the fetch-decode-execute loop: `HLT` (opcode `0xF4`). This isn't
//! a simulator-level convention this lane invented or inherited — it's
//! real silicon behaviour, faithfully ported from `code/packages/python/
//! intel-8086-simulator`'s `simulator.py` (`if op == 0xF4: self._halted =
//! True; return "HLT"`). `ret_*`/`ret_void` lowering to `HLT` is
//! therefore the most direct, least-invented choice of any halt-related
//! decision in this entire 9-architecture campaign.
//!
//! ## The `terminated: bool` pattern — and the bug class it avoids
//!
//! A real bug was found and fixed in **four** prior lanes of this
//! campaign (Intel 8051, Intel 8080, MOS 6502, Zilog Z80): the backend's
//! defensive "is the program already terminated?" check was written as a
//! **trailing-byte-value comparison** (`bytes.last() == Some(&HALT_BYTE)`)
//! or, worse, an `is_empty()` check. Both are unsound for this exact
//! reason: `MOV AX, imm16` encodes as `[0xB8, imm_lo, imm_hi]`, and
//! **the immediate's own bytes can numerically collide with the halt
//! opcode**. `HALT_BYTE` is `0xF4`; an immediate like `0xF400` encodes as
//! `[0xB8, 0x00, 0xF4]` — trailing byte `0xF4`, identical to `HLT`,
//! despite this program never having executed a real halt instruction at
//! all. A trailing-byte check would wrongly conclude "already
//! terminated" and skip appending the real `HLT`, silently shipping a
//! program with **no genuine halt instruction** — the CPU would fetch
//! whatever garbage byte follows in memory as the next opcode. `is_empty
//! ()` is unsound for a different reason: any `const_*` at all makes
//! `bytes` non-empty long before a real terminator is ever emitted, so
//! it can never correctly detect "no terminator yet" once the loop is
//! underway.
//!
//! This backend avoids the whole bug class by tracking an explicit
//! `terminated: bool` local, not a byte-value proxy:
//!
//! - Starts `false`.
//! - Set to `true` **only** when a real `ret_*`/`ret_void` arm pushes a
//!   genuine `HLT`.
//! - Reset to `false` whenever any further `const_*` (or other
//!   non-terminating instruction) is emitted afterward.
//! - At the end of the loop, if `terminated` is still `false`, a real
//!   `HLT` is appended — regardless of what byte value happens to sit
//!   last in the buffer.
//!
//! See `const_whose_encoded_high_byte_collides_with_halt_opcode_still_gets_real_terminator`
//! in `tests/test_backend.rs` for a regression test that would fail
//! against a naive trailing-byte-comparison implementation.
//!
//! ## Why is `Backend::run` not implemented?
//!
//! Emit-only target per the migration spec. Bytes go to
//! `intel8086-simulator`.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use intel8086_encoder::{encode_hlt, encode_mov_reg_imm16, REG_AX};
use std::fmt;
use vm_core::value::Value;

#[derive(Debug, Default, Clone, Copy)]
pub struct Intel8086Backend;

impl Intel8086Backend {
    pub fn new() -> Self {
        Intel8086Backend
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
            Self::UnsupportedOp(op) => write!(f, "intel8086-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "intel8086-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "intel8086-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "intel8086-backend: const {n} exceeds the 16-bit MOV-immediate range \
                 [0, 65535]; AX is 16 bits wide, so wider or negative CIR constants \
                 have no direct `MOV AX,#imm16` lowering"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Compile a single function's CIR into Intel 8086 bytes.
pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_to_bytes(cir)
}

fn compile_to_bytes(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    if cir.is_empty() {
        // Empty CIR -> just HLT so the program halts immediately.
        return Ok(encode_hlt());
    }

    let mut bytes = Vec::new();
    // v0.1.0 uses a trivial single-register allocator: the most recent
    // `const_*` puts its value into AX, and `ret_*` returns AX. Programs
    // that need more than one live var fall through to `UnsupportedOp`.
    let mut last_const_var: Option<String> = None;

    // Tracks "has a genuine halt-convention instruction (HLT) already
    // been pushed?" -- an explicit boolean, NOT a trailing-byte-value
    // comparison. See this module's doc for the exact byte-collision bug
    // class this avoids (fixed in four prior lanes of this campaign:
    // Intel 8051, Intel 8080, MOS 6502, Zilog Z80).
    let mut terminated = false;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            bytes.extend_from_slice(&encode_hlt());
            terminated = true;
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let src_name = parse_var_src(instr, 0, op)?;
            // We only support the case where src is the most recent
            // const'd var (i.e. it's already in AX). Multi-var requires
            // a real register allocator.
            if last_const_var.as_deref() != Some(src_name.as_str()) {
                return Err(BackendError::UnsupportedOp(format!(
                    "ret of {src_name:?} which is not the current AX var; \
                     multi-register allocation lands in a future increment"
                )));
            }
            bytes.extend_from_slice(&encode_hlt());
            terminated = true;
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let imm = encode_immediate_16(instr.srcs.first())?;
            // const_* always targets AX in this minimal backend.
            bytes.extend_from_slice(&encode_mov_reg_imm16(REG_AX, imm));
            last_const_var = Some(dest.to_string());
            // A non-terminating instruction was just emitted -- even if
            // the buffer's trailing byte now happens to numerically
            // equal HALT_BYTE (imm's high byte can be 0xF4), the program
            // has NOT halted. This reset is the crux of the
            // terminated:bool pattern: a byte-value check has no
            // equivalent "reset" step, which is exactly how the bug
            // class this avoids slips in.
            terminated = false;
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    // Defensive -- if no terminator was emitted, append HLT so the
    // program halts instead of running off the end (which would read
    // whatever byte follows in memory as the next opcode). Driven by
    // the `terminated` flag, NOT by inspecting `bytes`' trailing byte --
    // see this module's doc and the regression test in
    // tests/test_backend.rs.
    if !terminated {
        bytes.extend_from_slice(&encode_hlt());
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

/// The Intel 8086's `MOV reg16,#imm16` carries a plain, unsigned 16-bit
/// immediate. Accept the full unsigned range `[0, 65535]`; `AX` is 16
/// bits wide, so wider or negative CIR constants have no direct
/// lowering in this minimal-viable backend.
fn encode_immediate_16(op: Option<&CIROperand>) -> Result<u16, BackendError> {
    let n: i64 = match op {
        Some(CIROperand::Int(n)) => *n,
        Some(CIROperand::Bool(b)) => i64::from(*b),
        _ => {
            return Err(BackendError::InvalidOperand(
                "const_* srcs[0] must be Int or Bool".into(),
            ));
        }
    };
    if (0..=0xFFFF).contains(&n) {
        Ok(n as u16)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

impl Backend for Intel8086Backend {
    fn name(&self) -> &str {
        "intel8086"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_to_bytes(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!("intel8086 backend is emit-only; load bytes into intel8086-simulator to execute");
    }
}
