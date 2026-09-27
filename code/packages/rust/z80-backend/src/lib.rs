//! # `z80-backend` — Zilog Z80 backend for jit-core / aot-core.
//!
//! Seventh lane of the 9-architecture expansion. Mirror of
//! `intel8080-backend` in shape — the Z80 is the 8080's direct
//! architectural successor and a full superset of its opcode set, so
//! the "const → accumulator, ret → HALT" backend shape maps almost
//! directly (unlike MIPS/ARM1, which needed different return-mechanism
//! handling).
//!
//! ## v0.1.0 scope — minimal viable
//!
//! Same scope as `intel8080-backend` v0.1.0: just enough to compile the
//! trivial IIR program `const 42; ret` to real Zilog Z80 machine code
//! bytes, byte-for-byte (`LD A, 42; HALT` = `[0x3E, 0x2A, 0x76]`) — and,
//! since the Z80 is source/binary-compatible with the 8080 for exactly
//! this instruction pair, **byte-identical** to what `intel8080-backend`
//! emits for the same program.
//!
//! | CIR family | Status |
//! |------------|--------|
//! | `const_u8`, `const_bool` (single-var case) | ✓ → `LD A, n` |
//! | `const_u16` (single-var case) | ✓ → `LD HL, nn` |
//! | matching typed returns, `ret_void` | ✓ → `HALT` (entry-function exit) |
//! | Anything else | returns `None` |
//!
//! Per the GUIDING CONSTRAINT (see
//! `code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`), the architectural
//! correctness win (IIR → CIR via the `Backend` trait) is delivered
//! regardless of op-set parity. Future increments to `z80-backend` can
//! port richer op coverage (`LD r,r'`/`ADD`/`SUB`/`CP`/branches/calls/
//! the alternate register bank/`CB`-prefixed bit ops/IX-IY addressing)
//! using the fuller ISA `z80-simulator` already implements.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::fmt;
use vm_core::value::Value;
use z80_encoder::{encode_ld_a_n, encode_ld_rp_nn, HALT, LD_A_N_MAX, PAIR_HL};

#[derive(Debug, Default, Clone, Copy)]
pub struct Z80Backend;

impl Z80Backend {
    pub fn new() -> Self {
        Z80Backend
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
            Self::UnsupportedOp(op) => write!(f, "z80-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "z80-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "z80-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "z80-backend: const {n} is outside the selected unsigned result width"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_single_function(cir)
}

fn compile_single_function(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    if cir.is_empty() {
        return Ok(vec![HALT]);
    }

    let mut bytes = Vec::new();
    let mut current_value: Option<CurrentValue> = None;
    // Tracks whether a REAL HALT was emitted -- NOT whether `bytes` is
    // non-empty. CIR that ends in `const_*` with no following `ret_*`
    // would otherwise fall through with `bytes` non-empty (the LD A,n
    // bytes) but no terminator, leaving the compiled program to run
    // into whatever follows in memory instead of halting.
    let mut terminated = false;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            bytes.push(HALT);
            terminated = true;
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let expected_width = result_width_for_ret(op)
                .ok_or_else(|| BackendError::UnsupportedOp(op.to_string()))?;
            let src_name = parse_var_src(instr, 0, op)?;
            let Some(value) = current_value.as_ref() else {
                return Err(BackendError::UndefinedVariable(src_name));
            };
            if value.name != src_name {
                return Err(BackendError::UnsupportedOp(format!(
                    "ret of {src_name:?} which is not the current accumulator var; \
                     multi-register allocation lands in a future increment"
                )));
            }
            if value.width != expected_width {
                return Err(BackendError::UnsupportedOp(format!(
                    "{op} cannot return the current {}-bit value",
                    value.width.bits()
                )));
            }
            bytes.push(HALT);
            terminated = true;
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let width = result_width_for_const(op)
                .ok_or_else(|| BackendError::UnsupportedOp(op.to_string()))?;
            match width {
                ResultWidth::Byte => {
                    let imm = encode_byte_immediate(op, instr.srcs.first())?;
                    bytes.extend_from_slice(&encode_ld_a_n(imm));
                }
                ResultWidth::Word => {
                    let imm =
                        encode_unsigned_immediate(instr.srcs.first(), u16::MAX as i64, false)?;
                    bytes.extend_from_slice(&encode_ld_rp_nn(PAIR_HL, imm as u16));
                }
            }
            current_value = Some(CurrentValue {
                name: dest.to_string(),
                width,
            });
            terminated = false;
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    if !terminated {
        bytes.push(HALT);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultWidth {
    Byte,
    Word,
}

impl ResultWidth {
    fn bits(self) -> u8 {
        match self {
            Self::Byte => 8,
            Self::Word => 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CurrentValue {
    name: String,
    width: ResultWidth,
}

fn result_width_for_const(op: &str) -> Option<ResultWidth> {
    match op {
        "const_i64" | "const_u8" | "const_bool" => Some(ResultWidth::Byte),
        "const_u16" => Some(ResultWidth::Word),
        _ => None,
    }
}

fn result_width_for_ret(op: &str) -> Option<ResultWidth> {
    match op {
        "ret_i64" | "ret_u8" | "ret_bool" => Some(ResultWidth::Byte),
        "ret_u16" => Some(ResultWidth::Word),
        _ => None,
    }
}

fn encode_byte_immediate(op: &str, operand: Option<&CIROperand>) -> Result<u8, BackendError> {
    if op == "const_bool" {
        return match operand {
            Some(CIROperand::Bool(value)) => Ok(u8::from(*value)),
            _ => Err(BackendError::InvalidOperand(
                "const_bool srcs[0] must be Bool".into(),
            )),
        };
    }
    encode_unsigned_immediate(operand, LD_A_N_MAX as i64, op == "const_i64")
        .map(|value| value as u8)
}

fn encode_unsigned_immediate(
    operand: Option<&CIROperand>,
    max: i64,
    allow_bool: bool,
) -> Result<i64, BackendError> {
    let value = match operand {
        Some(CIROperand::Int(value)) => *value,
        Some(CIROperand::Bool(value)) if allow_bool => i64::from(*value),
        _ => {
            return Err(BackendError::InvalidOperand(
                "integer const srcs[0] must be Int".into(),
            ));
        }
    };
    if (0..=max).contains(&value) {
        Ok(value)
    } else {
        Err(BackendError::ImmediateOutOfRange(value))
    }
}

impl Backend for Z80Backend {
    fn name(&self) -> &str {
        "z80"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_single_function(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!("z80 backend is emit-only; load bytes into z80-simulator to execute");
    }
}
