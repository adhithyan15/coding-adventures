//! # `m68k-backend` — Motorola 68000 backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into Motorola 68000 machine code via
//! [`m68k_encoder`].  Output is `Vec<u8>` of big-endian bytes — the
//! 68000's native byte order, so unlike `arm1-backend` (which flattens
//! little-endian ARM1 words) there is no endianness conversion step
//! here; `m68k_encoder`'s bytes are already the wire format, matching
//! `mos6502-backend`'s simplicity (though for a different reason — the
//! 6502 has no word endianness at all, while the 68000 has one and
//! `m68k_encoder` already emits it correctly).
//!
//! Mirror of [`mos6502_backend`] / [`arm1_backend`] / [`armv7_backend`]
//! / [`intel8008_backend`] in shape.  Eighth lane of the 9-architecture
//! expansion following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## Scope (v0.1.0 — minimal viable)
//!
//! Minimal viable backend — covers the trivial-ROM case (`const_*`
//! immediate + `ret_*`) needed by the `lang-aot` M68K e2e smoke test:
//!
//! | CIR op | Lowering |
//! |--------|----------|
//! | `const_*` (32-bit literal, `[i32::MIN, u32::MAX]`) | `MOVE.L #imm, D0` |
//! | `ret_*`, `ret_void` | `TRAP #15` (the pre-existing HALT convention — see below) |
//! | Anything else | returns `None` |
//!
//! There is no real register allocator: a trivial "last const var"
//! scheme tracks which single variable the most recent `const_*` wrote
//! (into data register `D0`), and `ret_*` only succeeds if it returns
//! exactly that variable — the same scheme `mos6502-backend`/
//! `arm1-backend`/`armv7-backend`/`intel8008-backend` use.  Full op
//! coverage (arithmetic, comparisons, branches, calls — all of which
//! `m68k-simulator` already implements for a useful subset of the ISA)
//! is intentionally **not** wired into this backend yet; future
//! increments can extend `compile_to_bytes` to emit them.
//!
//! Per the migration spec, this is acceptable: the architectural
//! correctness win (IIR → CIR via `Backend` trait) is delivered as soon
//! as the AOT path is wired, regardless of op-set parity.
//!
//! ## Why does `ret_*` lower to `TRAP #15`, not `STOP #imm`?
//!
//! See `m68k_simulator`'s crate-level doc ("Halt convention") for the
//! full derivation.  Short version: the pre-existing Python simulator's
//! own `state.py` documents *both* `STOP` and `TRAP #15` as halting
//! conditions ("halted: True after STOP or TRAP #15 executes"), but its
//! own test suite's `_stop()` helper — used 100+ times across
//! `test_instructions.py`/`test_programs.py` — is `TRAP #15`, not
//! `STOP #imm` (which appears exactly once, in a module-level doctest).
//! `TRAP #15` is therefore the dominant, already-established convention
//! this lane mirrors, following this repo's own rule for such ties:
//! reuse what the pre-existing reference already does rather than invent
//! a fresh one (the same rule `mos6502-backend`'s `BRK` and
//! `arm1-backend`'s pseudo-halt `SWI` each followed for their own ISAs).
//!
//! ## Why is `Backend::run` not implemented?
//!
//! Emit-only target per the migration spec.  Bytes go to
//! `m68k-simulator`.
//!
//! ## Security note: the termination check uses a `bool`, not a byte
//! comparison
//!
//! A prior lane (Intel 8051) shipped a defensive "did I already emit a
//! terminator?" check that compared the *trailing emitted byte* against
//! the halt sentinel's byte value — which was unsound, because the
//! sentinel's byte value also happened to be a valid data-immediate
//! byte, so a `const_*` whose immediate happened to end in that byte
//! coincidentally produced a trailing byte identical to the sentinel,
//! fooling the check into skipping the real terminator (fixed in
//! `intel8051-backend` commit `19e360d`).  `TRAP #15`'s low byte is
//! `0x4F`, which is equally reachable as the low byte of a
//! `MOVE.L #imm, D0` immediate (e.g. `const_i64 79` — `0x4F` — with no
//! following `ret`), so the same trap applies here.  `compile_to_bytes`
//! therefore tracks an explicit `terminated: bool`, set `true` only when
//! a real `ret_*`/`ret_void` arm pushes `encode_trap15()`, and reset to
//! `false` whenever a further `const_*` is emitted — never a
//! byte/word-value comparison against the halt encoding.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use m68k_encoder::{assemble, encode_move_l_imm_to_dn, encode_trap15, D0};
use std::fmt;
use vm_core::value::Value;

#[derive(Debug, Default, Clone, Copy)]
pub struct M68kBackend;

impl M68kBackend {
    pub fn new() -> Self {
        M68kBackend
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
            Self::UnsupportedOp(op) => write!(f, "m68k-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "m68k-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "m68k-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "m68k-backend: const {n} exceeds the 32-bit MOVE.L-immediate range \
                 [{}, {}]; the 68000's data registers are 32 bits wide, so wider \
                 CIR constants have no direct `MOVE.L #imm, D0` lowering",
                i64::from(i32::MIN),
                i64::from(u32::MAX)
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Compile a single function's CIR into M68K bytes (big-endian — the
/// 68000's native byte order, and `m68k_encoder`'s wire format already).
pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_to_bytes(cir)
}

fn compile_to_bytes(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    if cir.is_empty() {
        // Empty CIR -> just TRAP #15 so the program halts immediately.
        return Ok(assemble(&[encode_trap15()]));
    }

    let mut bytes = Vec::new();
    // v0.1.0 uses a trivial single-register allocator: the most recent
    // `const_*` puts its value into D0, and `ret_*` returns D0.
    // Programs that need more than one live var fall through to
    // `UnsupportedOp`.
    let mut last_const_var: Option<String> = None;
    // Tracks whether a REAL halt instruction was emitted -- NOT whether
    // the trailing byte(s) happen to equal the sentinel's encoding.  See
    // this module's "Security note" doc section: `TRAP #15`'s low byte
    // (`0x4F`) is also reachable as the low byte of a `MOVE.L #imm, D0`
    // immediate, so a byte-value comparison would be unsound here the
    // same way it was in `intel8051-backend` before that lane's fix.
    let mut terminated = false;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            bytes.extend_from_slice(&encode_trap15());
            terminated = true;
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let src_name = parse_var_src(instr, 0, op)?;
            // We only support the case where src is the most recent
            // const'd var (i.e. it's already in D0).  Multi-var
            // requires a real register allocator.
            if last_const_var.as_deref() != Some(src_name.as_str()) {
                return Err(BackendError::UnsupportedOp(format!(
                    "ret of {src_name:?} which is not the current D0 var; \
                     multi-register allocation lands in a future increment"
                )));
            }
            bytes.extend_from_slice(&encode_trap15());
            terminated = true;
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let imm = encode_immediate_32(instr.srcs.first())?;
            // const_* always targets D0 in this minimal backend.
            bytes.extend_from_slice(&encode_move_l_imm_to_dn(D0, imm));
            last_const_var = Some(dest.to_string());
            terminated = false;
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    // Defensive -- if no REAL terminator was emitted, append TRAP #15 so
    // the program halts instead of running off the end (which would
    // read zeroed memory as instructions -- opword 0x0000 decodes as
    // the deferred line-0 immediate group in `m68k-simulator`, which
    // fails closed by halting anyway, but relying on that would be an
    // accident, not a design -- see this module's "Security note").
    if !terminated {
        bytes.extend_from_slice(&encode_trap15());
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

/// The 68000's `MOVE.L #imm, D0` carries a full 32-bit immediate — wider
/// than ARM1's unrotated 8-bit `MOV`-immediate or the 6502's 8-bit
/// `LDA`-immediate.  Accept any `i64` whose value fits in the union of
/// the signed 32-bit range and the unsigned 32-bit range (`n as u32`
/// then reproduces the correct 32-bit two's-complement bit pattern for
/// every value in that union, matching how a real assembler would
/// encode either `MOVE.L #-1, D0` or `MOVE.L #0xFFFFFFFF, D0` as the
/// identical opcode bytes).
fn encode_immediate_32(op: Option<&CIROperand>) -> Result<u32, BackendError> {
    let n: i64 = match op {
        Some(CIROperand::Int(n)) => *n,
        Some(CIROperand::Bool(b)) => i64::from(*b),
        _ => {
            return Err(BackendError::InvalidOperand(
                "const_* srcs[0] must be Int or Bool".into(),
            ));
        }
    };
    if (i64::from(i32::MIN)..=i64::from(u32::MAX)).contains(&n) {
        Ok(n as u32)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

impl Backend for M68kBackend {
    fn name(&self) -> &str {
        "m68k"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_to_bytes(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!("m68k backend is emit-only; load bytes into m68k-simulator to execute");
    }
}
