//! # `mos6502-backend` — MOS 6502 backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into MOS 6502 machine code via
//! [`mos6502_encoder`].  Output is `Vec<u8>` — the 6502 is a
//! byte-oriented ISA with no word endianness, so unlike
//! `arm1-backend`/`mips-r2000-backend` there is no byte-order flattening
//! step; the encoder's `Vec<u8>` bytes are already the wire format.
//!
//! Mirror of [`mips_r2000_backend`] / [`arm1_backend`] / [`armv7_backend`]
//! / [`intel8008_backend`] in shape.  Fifth lane of the 9-architecture
//! expansion following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## Scope (v0.1.0 — minimal viable)
//!
//! Minimal viable backend — covers the trivial-ROM case (`const_*`
//! immediate + `ret_*`) needed by the `lang-aot` MOS 6502 e2e smoke test:
//!
//! | CIR op | Lowering |
//! |--------|----------|
//! | `const_*` (8-bit literal, `[0, 255]`) | `LDA #imm` |
//! | `ret_*`, `ret_void` | `BRK` (the pre-existing HALT convention — see below) |
//! | Anything else | returns `None` |
//!
//! There is no real register allocator: a trivial "last const var"
//! scheme tracks which single variable the most recent `const_*` wrote
//! (into the accumulator `A`), and `ret_*` only succeeds if it returns
//! exactly that variable — the same scheme `mips-r2000-backend`/
//! `arm1-backend`/`armv7-backend`/`intel8008-backend` use.  Full op
//! coverage (arithmetic, branches, calls — all of which
//! `mos6502-simulator` already implements in full) is intentionally
//! **not** wired into this backend yet; future increments can extend
//! `compile_to_bytes` to emit them.
//!
//! Per the migration spec, this is acceptable: the architectural
//! correctness win (IIR → CIR via `Backend` trait) is delivered as soon
//! as the AOT path is wired, regardless of op-set parity.
//!
//! ## Why does `ret_*` lower to `BRK`, not a pseudo-halt or KIL/JAM?
//!
//! Unlike ARM1 (1985, no real halt instruction — `arm1-backend` had to
//! invent a pseudo-halt via `SWI #0x123456`), the MOS 6502 already has a
//! genuine, single-byte instruction the *existing, in-tree Python
//! simulator* treats as HALT: `BRK` (opcode `0x00`).  This is not a new
//! convention invented for this lane — `code/packages/python/
//! mos6502-simulator/src/mos6502_simulator/simulator.py`'s module
//! docstring states it outright:
//!
//! > *"Halt condition: BRK (opcode 0x00) is treated as HALT — the
//! > simulator stops and sets `halted=True` in the state. This matches
//! > the convention used throughout the simulator stack (HLT for 8080,
//! > TRAP for IBM 704, etc.)."*
//!
//! `mos6502-simulator` (this crate's transitive dependency via
//! `mos6502-encoder`) ports that exact convention — see
//! `mos6502_simulator::opcodes::BRK_OPCODE` and its module docs.
//! `mos6502-backend` mirrors it rather than reaching for either
//! alternative the historical-arch migration considered: an
//! illegal/undocumented opcode that locks the CPU (`KIL`/`JAM`, e.g.
//! `0x02`), or a self-targeting `JMP $addr` spin loop (the convention a
//! different halt-less lane in this 9-architecture expansion uses).
//! Real `BRK` is technically a software-interrupt instruction on genuine
//! 6502 silicon, not a true HALT — but since `BRK` already carries this
//! meaning throughout this repo's simulator stack, following it here is
//! the correct choice: the compiled bytes behave identically whether
//! loaded into `mos6502-simulator` today or, if a future increment wants
//! it, any other in-tree 6502 program that already expects `BRK` to mean
//! "stop".
//!
//! ## Why is `Backend::run` not implemented?
//!
//! Emit-only target per the migration spec.  Bytes go to
//! `mos6502-simulator`.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use mos6502_encoder::{encode_brk, encode_lda_imm, HALT_BYTE};
use std::fmt;
use vm_core::value::Value;

#[derive(Debug, Default, Clone, Copy)]
pub struct Mos6502Backend;

impl Mos6502Backend {
    pub fn new() -> Self {
        Mos6502Backend
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
            Self::UnsupportedOp(op) => write!(f, "mos6502-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "mos6502-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "mos6502-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "mos6502-backend: const {n} exceeds the 8-bit LDA-immediate range \
                 [0, 255]; the MOS 6502 accumulator is 8 bits wide, so wider or \
                 negative CIR constants have no direct `LDA #imm` lowering"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Compile a single function's CIR into MOS 6502 bytes.
pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_to_bytes(cir)
}

fn compile_to_bytes(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    if cir.is_empty() {
        // Empty CIR -> just BRK so the program halts immediately.
        return Ok(encode_brk());
    }

    let mut bytes = Vec::new();
    // v0.1.0 uses a trivial single-register allocator: the most recent
    // `const_*` puts its value into the accumulator, and `ret_*` returns
    // the accumulator.  Programs that need more than one live var fall
    // through to `UnsupportedOp`.
    let mut last_const_var: Option<String> = None;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            bytes.extend_from_slice(&encode_brk());
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let src_name = parse_var_src(instr, 0, op)?;
            // We only support the case where src is the most recent
            // const'd var (i.e. it's already in the accumulator).
            // Multi-var requires a real register allocator.
            if last_const_var.as_deref() != Some(src_name.as_str()) {
                return Err(BackendError::UnsupportedOp(format!(
                    "ret of {src_name:?} which is not the current accumulator var; \
                     multi-register allocation lands in a future increment"
                )));
            }
            bytes.extend_from_slice(&encode_brk());
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let imm = encode_immediate_8(instr.srcs.first())?;
            // const_* always targets the accumulator in this minimal backend.
            bytes.extend_from_slice(&encode_lda_imm(imm));
            last_const_var = Some(dest.to_string());
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    // Defensive -- if no terminator was emitted, append BRK so the
    // program halts instead of running off the end (which would read
    // zeroed memory as instructions -- opcode 0x00 happens to decode as
    // BRK too, but relying on that would be an accident, not a design).
    if bytes.last() != Some(&HALT_BYTE) {
        bytes.extend_from_slice(&encode_brk());
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

/// The MOS 6502's `LDA #imm` carries a plain, unsigned 8-bit immediate —
/// no rotated/shifted encoding trick like ARM1's barrel shifter.  Accept
/// the full unsigned byte range `[0, 255]`.
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

impl Backend for Mos6502Backend {
    fn name(&self) -> &str {
        "mos6502"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_to_bytes(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!("mos6502 backend is emit-only; load bytes into mos6502-simulator to execute");
    }
}
