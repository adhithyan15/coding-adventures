//! # `intel8051-backend` — Intel 8051 (MCS-51) backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into Intel 8051 machine code via
//! [`intel8051_encoder`].  Fourth lane of the 9-architecture
//! expansion following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//! Mirror of [`intel8008_backend`] / [`arm1_backend`] in shape and
//! intent — the Intel 8008 in particular is the closer relative here,
//! since both architectures route every scalar value through one
//! implicit "the" register (8008's `A`, 8051's accumulator `A`).
//!
//! ## Scope (v0.1.0 — minimal viable)
//!
//! Covers exactly the trivial-ROM case (`const_*` immediate +
//! `ret_*`) needed by the canonical Twig `42` program:
//!
//! | CIR op | Lowering |
//! |--------|----------|
//! | `const_*` (8-bit unsigned imm) | `MOV A, #imm` |
//! | `ret_*`, `ret_void` | HALT sentinel (`0xA5`) |
//! | Anything else | returns `None` |
//!
//! There is no real register allocator: a trivial "last const var"
//! scheme (identical to `arm1-backend`'s and `intel8008-backend`'s)
//! tracks which single variable the most recent `const_*` wrote into
//! the accumulator, and `ret_*` only succeeds if it returns exactly
//! that variable.  Full op coverage (add/sub/cmp/branches/calls) is
//! intentionally **not** ported here; future increments to this crate
//! can add them, including a real allocator over the 8051's other
//! working registers (`R0`-`R7`) and direct/indirect RAM addressing.
//!
//! Per the migration spec, this is acceptable: the architectural
//! correctness win (IIR → CIR via `Backend` trait) is delivered as
//! soon as the AOT path is wired, regardless of op-set parity.
//!
//! ## Why does `ret_*` lower to the HALT sentinel, not a real HALT instruction?
//!
//! **There is no real HALT instruction on the 8051.** Unlike the
//! Intel 8080/8008 (which have a genuine `HLT` opcode) or a modern ISA
//! with an OS to return control to, an 8051 program never "exits" — a
//! real, running 8051 program that's done working spins forever
//! (`SJMP $`, jump-to-self) or waits for the next interrupt, because
//! the chip has nothing to hand control back to. This is the
//! historically-idiomatic 8051 convention for "the program is done",
//! and it was seriously considered for this backend: detect a fixed
//! `SJMP $` self-loop pattern (`[0x80, 0xFE]`) as the pseudo-halt a
//! simulator recognises, the same way a real 8051 in-circuit debugger
//! would notice the PC has stopped advancing.
//!
//! It was **not** used, for a concrete, non-aesthetic reason: this
//! architecture already has a tested, shipped, documented HALT
//! convention — opcode `0xA5` (reserved/undefined in every MCS-51
//! opcode map), defined in the **existing Python behavioral reference**
//! this Rust simulator was ported from
//! (`intel8051_simulator.state.HALT_OPCODE`, spec 07p,
//! `code/specs/07p-intel-8051-simulator.md`'s "HALT convention"
//! section) and ported unchanged into
//! `intel8051_simulator::opcodes::HALT_OPCODE`. Inventing a *second*,
//! different halt convention (self-jump detection) for the same
//! architecture in the same codebase would fracture parity between
//! the Python and Rust simulators for no benefit — both now agree
//! byte-for-byte on what "the program is done" means, and a consumer
//! that already knows to look for `0xA5` (a debugger, a disassembler,
//! a test harness) keeps working across both language ports.
//!
//! Practically, the sentinel is also strictly simpler for an
//! *emit-only, minimal-viable* backend to detect: `halted()` becomes a
//! single opcode-equality check in `execute::execute` (see
//! `intel8051-simulator`), rather than requiring the simulator to
//! pattern-match "is the *next* fetch going to re-execute the same
//! `SJMP` at the same address" — real but avoidable complexity for a
//! target whose only current job is materialising one constant and
//! stopping. Self-jump detection remains available as a documented
//! fallback for a future increment that wants a more silicon-faithful
//! halt (e.g. if this backend grows real subroutine calls and needs
//! `ret_*` to mean "return to caller" rather than "the whole program is
//! done").
//!
//! ## Why is `Backend::run` not implemented?
//!
//! Emit-only target per the migration spec. Bytes go to
//! `intel8051-simulator`.

use intel8051_encoder::{encode_halt, encode_mov_a_imm, IMM8_MAX};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::fmt;
use vm_core::value::Value;

#[derive(Debug, Default, Clone, Copy)]
pub struct Intel8051Backend;

impl Intel8051Backend {
    pub fn new() -> Self {
        Intel8051Backend
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
            Self::UnsupportedOp(op) => write!(f, "intel8051-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "intel8051-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "intel8051-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "intel8051-backend: const {n} exceeds 8-bit MOV A,#imm range [0, 255]"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Compile a single function's CIR into Intel 8051 bytes.
pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_single_function(cir)
}

fn compile_single_function(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    if cir.is_empty() {
        // Empty CIR -> just the HALT sentinel so the program stops
        // immediately.
        return Ok(vec![encode_halt()]);
    }

    let mut bytes = Vec::new();
    // v0.1.0 uses a trivial single-accumulator allocator: the most
    // recent `const_*` puts its value into A, and `ret_*` returns A.
    // Programs that need more than one live value fall through to
    // `UnsupportedOp` (returned as `None` from the Backend trait) --
    // identical scheme to arm1-backend/intel8008-backend.
    let mut last_const_var: Option<String> = None;
    // Tracks whether a REAL halt instruction was emitted -- NOT
    // whether the trailing byte happens to equal the sentinel value.
    // A trailing-byte comparison is unsound here: `const_* 165` emits
    // `MOV A, #0xA5` (`[0x74, 0xA5]`), whose last byte is
    // numerically identical to the HALT sentinel despite not being
    // one, which would fool a byte-value check into skipping the
    // real terminator.
    let mut terminated = false;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            bytes.push(encode_halt());
            terminated = true;
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
            bytes.push(encode_halt());
            terminated = true;
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let imm8 = encode_immediate_8(instr.srcs.first())?;
            bytes.extend_from_slice(&encode_mov_a_imm(imm8));
            last_const_var = Some(dest.to_string());
            terminated = false;
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    // Defensive -- if no real terminator was emitted, append the HALT
    // sentinel so the program stops instead of running off the end
    // (which would read zeroed code memory as instructions -- 0x00
    // happens to be NOP on the 8051, so an un-terminated program
    // would spin through NOPs rather than crash, but that's still not
    // "the program is done").
    if !terminated {
        bytes.push(encode_halt());
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

/// `MOV A, #imm`'s operand is a plain unsigned 8-bit byte -- accept
/// the range `[0, 255]`.  Negative constants are out of scope for the
/// minimal-viable backend (the 8051 has no signed-immediate MOV form;
/// a real assembler would two's-complement-encode a negative literal,
/// which is future-increment work).
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
    if (0..=IMM8_MAX as i64).contains(&n) {
        Ok(n as u8)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

impl Backend for Intel8051Backend {
    fn name(&self) -> &str {
        "intel8051"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!(
            "intel8051 backend is emit-only; load bytes into intel8051-simulator to execute"
        );
    }
}
