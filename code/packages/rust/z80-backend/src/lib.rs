//! # `z80-backend` — Zilog Z80 backend for jit-core / aot-core.
//!
//! Seventh lane of the 9-architecture expansion. Mirror of
//! `intel8080-backend` in shape — the Z80 is the 8080's direct
//! architectural successor and a full superset of its opcode set, so
//! the "const → accumulator, ret → HALT" backend shape maps almost
//! directly (unlike MIPS/ARM1, which needed different return-mechanism
//! handling).
//!
//! ## WORD03 scope — comparisons and structured control
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
//! | unsigned comparisons, labels, branches, loops | ✓ → normalized bool + `JP` |
//! | Anything else | returns `None` |
//!
//! Per the GUIDING CONSTRAINT (see
//! `code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`), the architectural
//! correctness win (IIR → CIR via the `Backend` trait) is delivered
//! regardless of op-set parity. Future increments to `z80-backend` can
//! port richer op coverage (`LD r,r'`/`ADD`/`SUB`/`CP`/branches/calls/
//! the alternate register bank/`CB`-prefixed bit ops/IX-IY addressing)
//! using the fuller ISA `z80-simulator` already implements. WORD03 retains
//! the two-value bound while making liveness aware of control-flow edges.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::collections::{HashMap, HashSet};
use std::fmt;
use vm_core::value::Value;
use z80_encoder::{
    encode_add_hl_rp, encode_alu_reg, encode_ld_a_n, encode_ld_r_n, encode_ld_r_r, encode_ld_rp_nn,
    ALU_ADC, ALU_ADD, ALU_AND, ALU_OR, ALU_SBC, ALU_SUB, ALU_XOR, CPL, HALT, LD_A_N_MAX, PAIR_DE,
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
    UndefinedLabel(String),
    DuplicateLabel(String),
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
            Self::UndefinedLabel(n) => write!(f, "z80-backend: undefined label {n:?}"),
            Self::DuplicateLabel(n) => write!(f, "z80-backend: duplicate label {n:?}"),
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
    let mut labels = HashMap::new();
    let mut fixups: Vec<(usize, String)> = Vec::new();
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

        if op == "label" {
            let name = control_label(instr, 0, op)?;
            if instr.srcs.len() != 1 {
                return Err(BackendError::InvalidOperand(
                    "label requires one source".into(),
                ));
            }
            if labels.insert(name.clone(), bytes.len()).is_some() {
                return Err(BackendError::DuplicateLabel(name));
            }
            continue;
        }

        if op == "jmp" {
            let target = control_label(instr, 0, op)?;
            if instr.srcs.len() != 1 {
                return Err(BackendError::InvalidOperand(
                    "jmp requires one source".into(),
                ));
            }
            bytes.push(0xC3); // JP nn
            let patch = bytes.len();
            bytes.extend_from_slice(&[0, 0]);
            fixups.push((patch, target));
            terminated = false;
            continue;
        }

        if matches!(op, "jmp_if_true" | "jmp_if_false") {
            if instr.dest.is_some() || instr.srcs.len() != 2 || instr.ty != "void" {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} requires condition and label sources, no dest, and void type"
                )));
            }
            let condition = parse_var_src(instr, 0, op)?;
            let target = control_label(instr, 1, op)?;
            let slot = find_slot(&slots, &condition)
                .ok_or_else(|| BackendError::UndefinedVariable(condition.clone()))?;
            let value = slots[slot].as_ref().expect("located slot");
            if value.ty != "bool" || value.width != ResultWidth::Byte {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} condition must be bool"
                )));
            }
            if slot == 0 {
                bytes.extend_from_slice(&[0xFE, 0x00]); // CP 0
            } else {
                bytes.push(encode_ld_r_r(REG_C, REG_A));
                bytes.push(encode_ld_r_r(REG_A, REG_D));
                bytes.extend_from_slice(&[0xFE, 0x00]); // CP 0
                bytes.push(encode_ld_r_r(REG_A, REG_C)); // LD preserves flags
            }
            bytes.push(if op == "jmp_if_true" { 0xC2 } else { 0xCA }); // JP NZ/Z,nn
            let patch = bytes.len();
            bytes.extend_from_slice(&[0, 0]);
            fixups.push((patch, target));
            terminated = false;
            continue;
        }

        if op == "ret_void" {
            if instr.dest.is_some() || !instr.srcs.is_empty() || instr.ty != "void" {
                return Err(BackendError::InvalidOperand(
                    "ret_void requires no dest or sources and void type".into(),
                ));
            }
            bytes.push(HALT);
            terminated = true;
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let expected_width = result_width_for_ret(op)
                .ok_or_else(|| BackendError::UnsupportedOp(op.to_string()))?;
            let expected_ty = op.strip_prefix("ret_").expect("typed return");
            if instr.dest.is_some() || instr.srcs.len() != 1 || instr.ty != expected_ty {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} requires one source, no dest, and {expected_ty} type"
                )));
            }
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
            if value.ty != expected_ty {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} source type mismatch"
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
            let expected_ty = op.strip_prefix("const_").expect("constant");
            if instr.ty != expected_ty || instr.srcs.len() != 1 {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} requires {expected_ty} type and one source"
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
                ty: expected_ty.to_string(),
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
                || slots[left_slot].as_ref().expect("located slot").ty != width.name()
                || slots[right_slot].as_ref().expect("located slot").ty != width.name()
            {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} source width mismatch or source type mismatch"
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
                ty: width.name().to_string(),
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

        if let Some(width) = unary_not_width(op) {
            let dest = require_dest(instr, op)?;
            if instr.ty != width.name() || instr.srcs.len() != 1 {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} requires one variable and {} type",
                    width.name()
                )));
            }
            let src = parse_var_src(instr, 0, op)?;
            let source = find_slot(&slots, &src)
                .ok_or_else(|| BackendError::UndefinedVariable(src.clone()))?;
            let value = slots[source].as_ref().expect("located slot");
            if value.width != width || value.ty != width.name() {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} source type mismatch"
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
                bytes.push(encode_ld_r_r(REG_C, REG_A));
                if source == 1 {
                    bytes.push(encode_ld_r_r(REG_A, REG_D));
                }
                bytes.push(CPL);
                if target == 1 {
                    bytes.push(encode_ld_r_r(REG_D, REG_A));
                    bytes.push(encode_ld_r_r(REG_A, REG_C));
                }
            } else {
                let (low, high) = if source == 0 {
                    (REG_L, REG_H)
                } else {
                    (REG_E, REG_D)
                };
                bytes.push(encode_ld_r_r(REG_A, low));
                bytes.push(CPL);
                bytes.push(encode_ld_r_r(REG_C, REG_A));
                bytes.push(encode_ld_r_r(REG_A, high));
                bytes.push(CPL);
                bytes.push(encode_ld_r_r(REG_B, REG_A));
                let (low, high) = if target == 0 {
                    (REG_L, REG_H)
                } else {
                    (REG_E, REG_D)
                };
                bytes.push(encode_ld_r_r(low, REG_C));
                bytes.push(encode_ld_r_r(high, REG_B));
            }
            slots[target] = Some(CurrentValue {
                name: dest.into(),
                width,
                ty: width.name().into(),
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

        if let Some((relation, width)) = comparison(op) {
            let dest = require_dest(instr, op)?;
            if instr.ty != "bool" || instr.srcs.len() != 2 {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} requires two variables and bool result type"
                )));
            }
            let left = parse_var_src(instr, 0, op)?;
            let right = parse_var_src(instr, 1, op)?;
            let left_slot = find_slot(&slots, &left)
                .ok_or_else(|| BackendError::UndefinedVariable(left.clone()))?;
            let right_slot = find_slot(&slots, &right)
                .ok_or_else(|| BackendError::UndefinedVariable(right.clone()))?;
            for slot in [left_slot, right_slot] {
                let value = slots[slot].as_ref().expect("located slot");
                if value.width != width || value.ty != width.name() {
                    return Err(BackendError::InvalidOperand(format!(
                        "{op} source width mismatch or source type mismatch"
                    )));
                }
            }
            let target = slots
                .iter()
                .position(|slot| {
                    slot.as_ref().is_none_or(|value| {
                        value.name == dest || !live_after[index].contains(&value.name)
                    })
                })
                .ok_or_else(|| {
                    BackendError::UnsupportedOp("WORD03 comparison needs a free result slot".into())
                })?;
            emit_comparison(&mut bytes, relation, width, left_slot, right_slot, target);
            slots[target] = Some(CurrentValue {
                name: dest.into(),
                width: ResultWidth::Byte,
                ty: "bool".into(),
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
    for (patch, target) in fixups {
        let address = *labels
            .get(&target)
            .ok_or_else(|| BackendError::UndefinedLabel(target.clone()))?;
        let address = u16::try_from(address)
            .map_err(|_| BackendError::UnsupportedOp("branch target exceeds 16 bits".into()))?;
        bytes[patch..patch + 2].copy_from_slice(&address.to_le_bytes());
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

fn control_label(instr: &CIRInstr, idx: usize, op: &str) -> Result<String, BackendError> {
    if instr.dest.is_some() || instr.ty != "void" {
        return Err(BackendError::InvalidOperand(format!(
            "{op} requires no dest and void type"
        )));
    }
    parse_var_src(instr, idx, op)
}

type LiveSets = (Vec<HashSet<String>>, Vec<HashSet<String>>);

fn liveness(cir: &[CIRInstr]) -> Result<LiveSets, BackendError> {
    let mut label_indices = HashMap::new();
    for (index, instr) in cir.iter().enumerate() {
        if instr.op == "label" {
            let name = instr
                .srcs
                .first()
                .and_then(CIROperand::as_var)
                .ok_or_else(|| BackendError::InvalidOperand("label srcs[0] must be Var".into()))?;
            if label_indices.insert(name.to_string(), index).is_some() {
                return Err(BackendError::DuplicateLabel(name.to_string()));
            }
        }
    }
    let mut before = vec![HashSet::new(); cir.len()];
    let mut after = before.clone();
    loop {
        let old_before = before.clone();
        for index in (0..cir.len()).rev() {
            let instr = &cir[index];
            let mut successors = Vec::new();
            match instr.op.as_str() {
                op if op.starts_with("ret_") => {}
                "jmp" => successors.push(branch_target_index(instr, 0, &label_indices)?),
                "jmp_if_true" | "jmp_if_false" => {
                    successors.push(branch_target_index(instr, 1, &label_indices)?);
                    if index + 1 < cir.len() {
                        successors.push(index + 1);
                    }
                }
                _ if index + 1 < cir.len() => successors.push(index + 1),
                _ => {}
            }
            after[index] = successors
                .into_iter()
                .flat_map(|successor| before[successor].iter().cloned())
                .collect();
            let mut live = after[index].clone();
            if let Some(dest) = &instr.dest {
                live.remove(dest);
            }
            let value_sources = match instr.op.as_str() {
                "label" | "jmp" => 0,
                "jmp_if_true" | "jmp_if_false" => 1,
                _ => instr.srcs.len(),
            };
            for source in instr.srcs.iter().take(value_sources) {
                if let CIROperand::Var(name) = source {
                    live.insert(name.clone());
                }
            }
            if live.len() > 2 {
                return Err(BackendError::UnsupportedOp(
                    "WORD03 requires a third live value; spilling is deferred".into(),
                ));
            }
            before[index] = live;
        }
        if before == old_before {
            break;
        }
    }
    Ok((before, after))
}

fn branch_target_index(
    instr: &CIRInstr,
    source: usize,
    labels: &HashMap<String, usize>,
) -> Result<usize, BackendError> {
    let target = instr
        .srcs
        .get(source)
        .and_then(CIROperand::as_var)
        .ok_or_else(|| BackendError::InvalidOperand(format!("{} target must be Var", instr.op)))?;
    labels
        .get(target)
        .copied()
        .ok_or_else(|| BackendError::UndefinedLabel(target.to_string()))
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

fn unary_not_width(op: &str) -> Option<ResultWidth> {
    match op {
        "not_u8" => Some(ResultWidth::Byte),
        "not_u16" => Some(ResultWidth::Word),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Comparison {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

fn comparison(op: &str) -> Option<(Comparison, ResultWidth)> {
    let suffix = op.strip_prefix("cmp_")?;
    let (relation, width) = suffix.rsplit_once('_')?;
    let relation = match relation {
        "eq" => Comparison::Eq,
        "ne" => Comparison::Ne,
        "lt" => Comparison::Lt,
        "le" => Comparison::Le,
        "gt" => Comparison::Gt,
        "ge" => Comparison::Ge,
        _ => return None,
    };
    let width = match width {
        "u8" => ResultWidth::Byte,
        "u16" => ResultWidth::Word,
        _ => return None,
    };
    Some((relation, width))
}

fn emit_comparison(
    bytes: &mut Vec<u8>,
    relation: Comparison,
    width: ResultWidth,
    left: usize,
    right: usize,
    target: usize,
) {
    if width == ResultWidth::Byte {
        bytes.push(encode_ld_r_r(REG_C, REG_A));
        if left == 1 {
            bytes.push(encode_ld_r_r(REG_A, REG_D));
        }
        bytes.push(encode_alu_reg(7, if right == 0 { REG_C } else { REG_D })); // CP r
    } else {
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
        bytes.push(encode_ld_r_r(REG_A, left_high));
        bytes.push(encode_alu_reg(7, right_high)); // CP r
        bytes.extend_from_slice(&[0x20, 0x02]); // JR NZ,+2: high byte decides
        bytes.push(encode_ld_r_r(REG_A, left_low));
        bytes.push(encode_alu_reg(7, right_low));
    }

    bytes.extend_from_slice(&encode_ld_a_n(0)); // LD does not change comparison flags
    match relation {
        Comparison::Eq => bytes.extend_from_slice(&[0x20, 0x01]), // JR NZ,skip
        Comparison::Ne => bytes.extend_from_slice(&[0x28, 0x01]), // JR Z,skip
        Comparison::Lt => bytes.extend_from_slice(&[0x30, 0x01]), // JR NC,skip
        Comparison::Ge => bytes.extend_from_slice(&[0x38, 0x01]), // JR C,skip
        Comparison::Le => bytes.extend_from_slice(&[0x38, 0x02, 0x20, 0x01]),
        Comparison::Gt => bytes.extend_from_slice(&[0x38, 0x03, 0x28, 0x01]),
    }
    bytes.push(0x3C); // INC A, producing the normalized true value 1

    if target == 1 {
        bytes.push(encode_ld_r_r(REG_D, REG_A));
        if width == ResultWidth::Byte {
            bytes.push(encode_ld_r_r(REG_A, REG_C));
        }
    }
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
    ty: String,
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
