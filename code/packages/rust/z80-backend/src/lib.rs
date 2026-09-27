//! # `z80-backend` — Zilog Z80 backend for jit-core / aot-core.
//!
//! Seventh lane of the 9-architecture expansion. Mirror of
//! `intel8080-backend` in shape — the Z80 is the 8080's direct
//! architectural successor and a full superset of its opcode set, so
//! the "const → accumulator, ret → HALT" backend shape maps almost
//! directly (unlike MIPS/ARM1, which needed different return-mechanism
//! handling).
//!
//! ## Current scope — WORD02
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
//! | `const_u8`, `const_bool` | ✓ → `LD A/B, n` |
//! | `const_u16` | ✓ → `LD HL/DE, nn` |
//! | wrapping arithmetic and bitwise `u8`/`u16` ops | ✓ → native or bytewise Z80 ALU sequences |
//! | matching typed returns, `ret_void` | ✓ → `HALT` (entry-function exit) |
//! | Anything else | returns `None` |
//!
//! Per the GUIDING CONSTRAINT (see
//! `code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`), the architectural
//! correctness win (IIR → CIR via the `Backend` trait) is delivered
//! regardless of op-set parity. Future increments to `z80-backend` can
//! port richer op coverage (`CP`/branches/calls/
//! the alternate register bank/`CB`-prefixed bit ops/IX-IY addressing)
//! using the fuller ISA `z80-simulator` already implements.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::fmt;
use vm_core::value::Value;
use z80_encoder::{
    encode_add_hl_rp, encode_alu_reg, encode_ld_a_n, encode_ld_r_n, encode_ld_r_r, encode_ld_rp_nn,
    ALU_ADD, ALU_AND, ALU_OR, ALU_SBC, ALU_SUB, ALU_XOR, CPL, HALT, LD_A_N_MAX, PAIR_DE, PAIR_HL,
    REG_A, REG_B, REG_C, REG_D, REG_E, REG_H, REG_L,
};

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
    let mut live_values: Vec<LiveValue> = Vec::new();
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
            let Some(value) = live_values.iter().find(|value| value.name == src_name) else {
                return Err(BackendError::UndefinedVariable(src_name));
            };
            if value.width != expected_width {
                return Err(BackendError::UnsupportedOp(format!(
                    "{op} cannot return the current {}-bit value",
                    value.width.bits()
                )));
            }
            materialize_primary(&mut bytes, value.width, value.location);
            bytes.push(HALT);
            terminated = true;
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let width = result_width_for_const(op)
                .ok_or_else(|| BackendError::UnsupportedOp(op.to_string()))?;
            let location = allocate_location(&live_values, width)?;
            match (width, location) {
                (ResultWidth::Byte, ValueLocation::Primary) => {
                    let imm = encode_byte_immediate(op, instr.srcs.first())?;
                    bytes.extend_from_slice(&encode_ld_a_n(imm));
                }
                (ResultWidth::Byte, ValueLocation::Secondary) => {
                    let imm = encode_byte_immediate(op, instr.srcs.first())?;
                    bytes.extend_from_slice(&encode_ld_r_n(REG_B, imm));
                }
                (ResultWidth::Word, ValueLocation::Primary) => {
                    let imm =
                        encode_unsigned_immediate(instr.srcs.first(), u16::MAX as i64, false)?;
                    bytes.extend_from_slice(&encode_ld_rp_nn(PAIR_HL, imm as u16));
                }
                (ResultWidth::Word, ValueLocation::Secondary) => {
                    let imm =
                        encode_unsigned_immediate(instr.srcs.first(), u16::MAX as i64, false)?;
                    bytes.extend_from_slice(&encode_ld_rp_nn(PAIR_DE, imm as u16));
                }
            }
            live_values.push(LiveValue {
                name: dest.to_string(),
                width,
                location,
            });
            terminated = false;
            continue;
        }

        if let Some((width, binary_op)) = binary_op(op) {
            let dest = require_dest(instr, op)?;
            let lhs_name = parse_var_src(instr, 0, op)?;
            let rhs_name = parse_var_src(instr, 1, op)?;
            let lhs = find_live_value(&live_values, &lhs_name, width, op)?;
            let rhs = find_live_value(&live_values, &rhs_name, width, op)?;
            emit_binary(&mut bytes, width, binary_op, lhs.location, rhs.location);
            live_values = vec![LiveValue {
                name: dest.to_string(),
                width,
                location: ValueLocation::Primary,
            }];
            terminated = false;
            continue;
        }

        if let Some(width) = unary_not_width(op) {
            let dest = require_dest(instr, op)?;
            let src_name = parse_var_src(instr, 0, op)?;
            let src = find_live_value(&live_values, &src_name, width, op)?;
            materialize_primary(&mut bytes, width, src.location);
            emit_not(&mut bytes, width);
            live_values = vec![LiveValue {
                name: dest.to_string(),
                width,
                location: ValueLocation::Primary,
            }];
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
struct LiveValue {
    name: String,
    width: ResultWidth,
    location: ValueLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueLocation {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOp {
    Add,
    Sub,
    And,
    Or,
    Xor,
}

fn allocate_location(
    live_values: &[LiveValue],
    width: ResultWidth,
) -> Result<ValueLocation, BackendError> {
    match live_values {
        [] => Ok(ValueLocation::Primary),
        [value] if value.width == width => Ok(ValueLocation::Secondary),
        [value] => Err(BackendError::UnsupportedOp(format!(
            "cannot keep a {}-bit and {}-bit value live together in WORD02",
            value.width.bits(),
            width.bits()
        ))),
        _ => Err(BackendError::UnsupportedOp(
            "WORD02 supports at most two live values".into(),
        )),
    }
}

fn find_live_value<'a>(
    live_values: &'a [LiveValue],
    name: &str,
    width: ResultWidth,
    op: &str,
) -> Result<&'a LiveValue, BackendError> {
    let value = live_values
        .iter()
        .find(|value| value.name == name)
        .ok_or_else(|| BackendError::UndefinedVariable(name.to_string()))?;
    if value.width == width {
        Ok(value)
    } else {
        Err(BackendError::UnsupportedOp(format!(
            "{op} requires {}-bit operands",
            width.bits()
        )))
    }
}

fn binary_op(op: &str) -> Option<(ResultWidth, BinaryOp)> {
    let (name, width) = op.rsplit_once('_')?;
    let width = match width {
        "u8" => ResultWidth::Byte,
        "u16" => ResultWidth::Word,
        _ => return None,
    };
    let operation = match name {
        "add" => BinaryOp::Add,
        "sub" => BinaryOp::Sub,
        "and" => BinaryOp::And,
        "or" => BinaryOp::Or,
        "xor" => BinaryOp::Xor,
        _ => return None,
    };
    Some((width, operation))
}

fn unary_not_width(op: &str) -> Option<ResultWidth> {
    match op {
        "not_u8" => Some(ResultWidth::Byte),
        "not_u16" => Some(ResultWidth::Word),
        _ => None,
    }
}

fn materialize_primary(bytes: &mut Vec<u8>, width: ResultWidth, location: ValueLocation) {
    if location == ValueLocation::Primary {
        return;
    }
    match width {
        ResultWidth::Byte => bytes.push(encode_ld_r_r(REG_A, REG_B)),
        ResultWidth::Word => {
            bytes.push(encode_ld_r_r(REG_H, REG_D));
            bytes.push(encode_ld_r_r(REG_L, REG_E));
        }
    }
}

fn emit_binary(
    bytes: &mut Vec<u8>,
    width: ResultWidth,
    op: BinaryOp,
    lhs: ValueLocation,
    rhs: ValueLocation,
) {
    if lhs == rhs {
        materialize_primary(bytes, width, lhs);
        emit_same_value_binary(bytes, width, op);
        return;
    }
    let reversed = lhs == ValueLocation::Secondary;
    match width {
        ResultWidth::Byte => emit_byte_binary(bytes, op, reversed),
        ResultWidth::Word => emit_word_binary(bytes, op, reversed),
    }
}

fn emit_same_value_binary(bytes: &mut Vec<u8>, width: ResultWidth, op: BinaryOp) {
    match width {
        ResultWidth::Byte => {
            let alu = match op {
                BinaryOp::Add => ALU_ADD,
                BinaryOp::Sub => ALU_SUB,
                BinaryOp::And => ALU_AND,
                BinaryOp::Or => ALU_OR,
                BinaryOp::Xor => ALU_XOR,
            };
            bytes.push(encode_alu_reg(alu, REG_A));
        }
        ResultWidth::Word => match op {
            BinaryOp::Add => bytes.push(encode_add_hl_rp(PAIR_HL)),
            _ => emit_word_bytewise(bytes, op, REG_L, REG_H),
        },
    }
}

fn emit_byte_binary(bytes: &mut Vec<u8>, op: BinaryOp, reversed: bool) {
    if reversed && op == BinaryOp::Sub {
        bytes.push(encode_ld_r_r(REG_C, REG_A));
        bytes.push(encode_ld_r_r(REG_A, REG_B));
        bytes.push(encode_alu_reg(ALU_SUB, REG_C));
        return;
    }
    let alu = match op {
        BinaryOp::Add => ALU_ADD,
        BinaryOp::Sub => ALU_SUB,
        BinaryOp::And => ALU_AND,
        BinaryOp::Or => ALU_OR,
        BinaryOp::Xor => ALU_XOR,
    };
    bytes.push(encode_alu_reg(alu, REG_B));
}

fn emit_word_binary(bytes: &mut Vec<u8>, op: BinaryOp, reversed: bool) {
    if reversed && op == BinaryOp::Sub {
        bytes.push(encode_ld_r_r(REG_B, REG_H));
        bytes.push(encode_ld_r_r(REG_C, REG_L));
        bytes.push(encode_ld_r_r(REG_H, REG_D));
        bytes.push(encode_ld_r_r(REG_L, REG_E));
        emit_word_bytewise(bytes, BinaryOp::Sub, REG_C, REG_B);
        return;
    }
    match op {
        BinaryOp::Add => bytes.push(encode_add_hl_rp(PAIR_DE)),
        _ => emit_word_bytewise(bytes, op, REG_E, REG_D),
    }
}

fn emit_word_bytewise(bytes: &mut Vec<u8>, op: BinaryOp, low: u8, high: u8) {
    let (low_op, high_op) = match op {
        BinaryOp::Sub => (ALU_SUB, ALU_SBC),
        BinaryOp::And => (ALU_AND, ALU_AND),
        BinaryOp::Or => (ALU_OR, ALU_OR),
        BinaryOp::Xor => (ALU_XOR, ALU_XOR),
        BinaryOp::Add => unreachable!("word addition uses ADD HL,rp"),
    };
    bytes.push(encode_ld_r_r(REG_A, REG_L));
    bytes.push(encode_alu_reg(low_op, low));
    bytes.push(encode_ld_r_r(REG_L, REG_A));
    bytes.push(encode_ld_r_r(REG_A, REG_H));
    bytes.push(encode_alu_reg(high_op, high));
    bytes.push(encode_ld_r_r(REG_H, REG_A));
}

fn emit_not(bytes: &mut Vec<u8>, width: ResultWidth) {
    match width {
        ResultWidth::Byte => bytes.push(CPL),
        ResultWidth::Word => {
            bytes.push(encode_ld_r_r(REG_A, REG_L));
            bytes.push(CPL);
            bytes.push(encode_ld_r_r(REG_L, REG_A));
            bytes.push(encode_ld_r_r(REG_A, REG_H));
            bytes.push(CPL);
            bytes.push(encode_ld_r_r(REG_H, REG_A));
        }
    }
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
