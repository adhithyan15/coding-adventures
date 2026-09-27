//! # `z80-backend` — Zilog Z80 backend for jit-core / aot-core.
//!
//! Seventh lane of the 9-architecture expansion. Mirror of
//! `intel8080-backend` in shape — the Z80 is the 8080's direct
//! architectural successor and a full superset of its opcode set, so
//! the "const → accumulator, ret → HALT" backend shape maps almost
//! directly (unlike MIPS/ARM1, which needed different return-mechanism
//! handling).
//!
//! ## WORD02 scope — bounded arithmetic
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
//! | two-live `add/sub/and/or/xor` on `u8`/`u16` | ✓ → register ALU |
//! | Anything else | returns `None` |
//!
//! Per the GUIDING CONSTRAINT (see
//! `code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`), the architectural
//! correctness win (IIR → CIR via the `Backend` trait) is delivered
//! regardless of op-set parity. Future increments to `z80-backend` can
//! port richer op coverage (`LD r,r'`/`ADD`/`SUB`/`CP`/branches/calls/
//! the alternate register bank/`CB`-prefixed bit ops/IX-IY addressing)
//! using the fuller ISA `z80-simulator` already implements. WORD02 explicitly
//! limits live values to two of one width; WORD03+ will grow that contract.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::collections::HashSet;
use std::fmt;
use vm_core::value::Value;
use z80_encoder::{
    encode_add_hl_rp, encode_alu_reg, encode_ld_a_n, encode_ld_r_n, encode_ld_r_r, encode_ld_rp_nn,
    ALU_ADC, ALU_ADD, ALU_AND, ALU_OR, ALU_SBC, ALU_SUB, ALU_XOR, HALT, LD_A_N_MAX, PAIR_DE,
    PAIR_HL, REG_A, REG_B, REG_C, REG_D, REG_E, REG_H, REG_L,
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
    // WORD02 has two source-visible slots. BC and A are scratch during word
    // operations; scratch never becomes a third live virtual value.
    let mut slots: [Option<CurrentValue>; 2] = [None, None];
    let (live_before, live_after) = liveness(cir)?;
    // Tracks whether a REAL HALT was emitted -- NOT whether `bytes` is
    // non-empty. CIR that ends in `const_*` with no following `ret_*`
    // would otherwise fall through with `bytes` non-empty (the LD A,n
    // bytes) but no terminator, leaving the compiled program to run
    // into whatever follows in memory instead of halting.
    let mut terminated = false;

    for (index, instr) in cir.iter().enumerate() {
        for slot in &mut slots {
            if slot
                .as_ref()
                .is_some_and(|value| !live_before[index].contains(&value.name))
            {
                *slot = None;
            }
        }
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
            let Some(slot) = find_slot(&slots, &src_name) else {
                return Err(BackendError::UndefinedVariable(src_name));
            };
            let value = slots[slot].as_ref().expect("located slot");
            if value.width != expected_width {
                return Err(BackendError::UnsupportedOp(format!(
                    "{op} cannot return the current {}-bit value",
                    value.width.bits()
                )));
            }
            if slot == 1 {
                if expected_width == ResultWidth::Byte {
                    bytes.push(encode_ld_r_r(REG_A, REG_D));
                } else {
                    bytes.push(encode_ld_r_r(REG_L, REG_E));
                    bytes.push(encode_ld_r_r(REG_H, REG_D));
                }
            }
            bytes.push(HALT);
            terminated = true;
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let width = result_width_for_const(op)
                .ok_or_else(|| BackendError::UnsupportedOp(op.to_string()))?;
            if matches!(op, "const_u8" | "const_u16") && instr.ty != width.name() {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} requires {} type",
                    width.name()
                )));
            }
            if slots.iter().flatten().any(|value| value.width != width) {
                return Err(BackendError::UnsupportedOp(
                    "mixed-width live values wait for a later Word rung".into(),
                ));
            }
            let slot = slots.iter().position(Option::is_none).ok_or_else(|| {
                BackendError::UnsupportedOp("WORD02 has only two live-value slots".into())
            })?;
            match width {
                ResultWidth::Byte => {
                    let imm = encode_byte_immediate(op, instr.srcs.first())?;
                    if slot == 0 {
                        bytes.extend_from_slice(&encode_ld_a_n(imm));
                    } else {
                        bytes.extend_from_slice(&encode_ld_r_n(REG_D, imm));
                    }
                }
                ResultWidth::Word => {
                    let imm =
                        encode_unsigned_immediate(instr.srcs.first(), u16::MAX as i64, false)?;
                    bytes.extend_from_slice(&encode_ld_rp_nn(
                        if slot == 0 { PAIR_HL } else { PAIR_DE },
                        imm as u16,
                    ));
                }
            }
            slots[slot] = Some(CurrentValue {
                name: dest.to_string(),
                width,
            });
            terminated = false;
            continue;
        }

        if let Some((operation, width)) = binary_operation(op) {
            let dest = require_dest(instr, op)?;
            if instr.ty != width.name() || instr.srcs.len() != 2 {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} requires two variables and {} type",
                    width.name()
                )));
            }
            let left = parse_var_src(instr, 0, op)?;
            let right = parse_var_src(instr, 1, op)?;
            let left_slot = find_slot(&slots, &left)
                .ok_or_else(|| BackendError::UndefinedVariable(left.clone()))?;
            let right_slot = find_slot(&slots, &right)
                .ok_or_else(|| BackendError::UndefinedVariable(right.clone()))?;
            if slots[left_slot].as_ref().expect("located slot").width != width
                || slots[right_slot].as_ref().expect("located slot").width != width
            {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} source width mismatch"
                )));
            }
            let target = slots
                .iter()
                .position(|slot| {
                    slot.as_ref().is_none_or(|value| {
                        value.name == dest || !live_after[index].contains(&value.name)
                    })
                })
                .ok_or_else(|| {
                    BackendError::UnsupportedOp(
                        "WORD02 requires a third live value; spilling is deferred".into(),
                    )
                })?;
            if width == ResultWidth::Byte {
                emit_byte_binary(&mut bytes, operation, left_slot, right_slot, target);
            } else {
                emit_word_binary(&mut bytes, operation, left_slot, right_slot, target);
            }
            slots[target] = Some(CurrentValue {
                name: dest.into(),
                width,
            });
            for (slot_index, slot) in slots.iter_mut().enumerate() {
                if slot_index != target
                    && slot
                        .as_ref()
                        .is_some_and(|value| !live_after[index].contains(&value.name))
                {
                    *slot = None;
                }
            }
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

type LiveSets = (Vec<HashSet<String>>, Vec<HashSet<String>>);

fn liveness(cir: &[CIRInstr]) -> Result<LiveSets, BackendError> {
    let mut before = vec![HashSet::new(); cir.len()];
    let mut after = before.clone();
    let mut live = HashSet::new();
    for (index, instr) in cir.iter().enumerate().rev() {
        after[index] = live.clone();
        if let Some(dest) = &instr.dest {
            live.remove(dest);
        }
        for source in &instr.srcs {
            if let CIROperand::Var(name) = source {
                live.insert(name.clone());
            }
        }
        if live.len() > 2 {
            return Err(BackendError::UnsupportedOp(
                "WORD02 requires a third live value; spilling is deferred".into(),
            ));
        }
        before[index] = live.clone();
    }
    Ok((before, after))
}

fn find_slot(slots: &[Option<CurrentValue>; 2], name: &str) -> Option<usize> {
    slots
        .iter()
        .position(|slot| slot.as_ref().is_some_and(|value| value.name == name))
}

fn binary_operation(op: &str) -> Option<(u8, ResultWidth)> {
    let (name, width) = op.rsplit_once('_')?;
    let width = match width {
        "u8" => ResultWidth::Byte,
        "u16" => ResultWidth::Word,
        _ => return None,
    };
    let operation = match name {
        "add" => ALU_ADD,
        "sub" => ALU_SUB,
        "and" => ALU_AND,
        "or" => ALU_OR,
        "xor" => ALU_XOR,
        _ => return None,
    };
    Some((operation, width))
}

/// A holds the operation result. C preserves the old A while D is the
/// second live byte; therefore a result placed in D cannot corrupt A.
fn emit_byte_binary(bytes: &mut Vec<u8>, operation: u8, left: usize, right: usize, target: usize) {
    bytes.push(encode_ld_r_r(REG_C, REG_A));
    if left == 1 {
        bytes.push(encode_ld_r_r(REG_A, REG_D));
    }
    bytes.push(encode_alu_reg(
        operation,
        if right == 0 { REG_C } else { REG_D },
    ));
    if target == 1 {
        bytes.push(encode_ld_r_r(REG_D, REG_A));
        bytes.push(encode_ld_r_r(REG_A, REG_C));
    }
}

/// The native `ADD HL,DE` path proves pair arithmetic in the common case.
/// Other operations compute low then high byte into BC; LD does not change
/// carry, so ADC/SBC propagates the low-byte carry or borrow correctly.
fn emit_word_binary(bytes: &mut Vec<u8>, operation: u8, left: usize, right: usize, target: usize) {
    if operation == ALU_ADD && left == 0 && right == 1 && target == 0 {
        bytes.push(encode_add_hl_rp(PAIR_DE));
        return;
    }
    let (left_low, left_high) = if left == 0 {
        (REG_L, REG_H)
    } else {
        (REG_E, REG_D)
    };
    let (right_low, right_high) = if right == 0 {
        (REG_L, REG_H)
    } else {
        (REG_E, REG_D)
    };
    bytes.push(encode_ld_r_r(REG_A, left_low));
    bytes.push(encode_alu_reg(operation, right_low));
    bytes.push(encode_ld_r_r(REG_C, REG_A));
    bytes.push(encode_ld_r_r(REG_A, left_high));
    let high_operation = match operation {
        ALU_ADD => ALU_ADC,
        ALU_SUB => ALU_SBC,
        other => other,
    };
    bytes.push(encode_alu_reg(high_operation, right_high));
    bytes.push(encode_ld_r_r(REG_B, REG_A));
    let (target_low, target_high) = if target == 0 {
        (REG_L, REG_H)
    } else {
        (REG_E, REG_D)
    };
    bytes.push(encode_ld_r_r(target_low, REG_C));
    bytes.push(encode_ld_r_r(target_high, REG_B));
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
    fn name(self) -> &'static str {
        match self {
            Self::Byte => "u8",
            Self::Word => "u16",
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
