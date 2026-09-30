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
//! ## Scope (WORD03b)
//!
//! The original trivial-ROM case (`const_*` immediate + `ret_*`) remains
//! supported alongside the typed arithmetic and control-flow rungs:
//!
//! | CIR op | Lowering |
//! |--------|----------|
//! | `const_u8`, `const_bool` | `MOV AX, #imm16` with `AH = 0` |
//! | `const_u16` | `MOV AX, #imm16` |
//! | matching typed returns, `ret_void` | `HLT` (a genuine hardware halt — see below) |
//! | two-live `add/sub/and/or/xor` on `u8`/`u16` | register ALU |
//! | typed unsigned `cmp_{eq,ne,lt,le,gt,ge}_{u8,u16}` | normalized `bool` |
//! | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` | `Jcc` + near `JMP` fixups |
//! | Anything else | returns `None` |
//!
//! Reverse liveness allocates at most two same-width values in `AX`/`BX`
//! (or `AL`/`BL`) and uses `CX`/`CL` as a transient scratch. A third
//! value and simultaneous byte and word values remain explicit errors.
//! Control-flow-aware liveness follows forward branches and loop back edges.
//! Returns copy the selected value to `AX`.
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

use intel8086_encoder::{
    encode_alu_reg_reg, encode_hlt, encode_jcc_short, encode_jmp_near, encode_mov_reg_imm16,
    encode_mov_reg_imm8, encode_mov_reg_reg16, encode_mov_reg_reg8, REG_AH, REG_AL, REG_AX, REG_BH,
    REG_BL, REG_BX, REG_CL, REG_CX,
};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::collections::{HashMap, HashSet, VecDeque};
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
    UndefinedLabel(String),
    DuplicateLabel(String),
    BranchOutOfRange(String),
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
            Self::UndefinedLabel(n) => write!(f, "intel8086-backend: undefined label {n:?}"),
            Self::DuplicateLabel(n) => write!(f, "intel8086-backend: duplicate label {n:?}"),
            Self::BranchOutOfRange(n) => {
                write!(f, "intel8086-backend: branch to {n:?} is out of range")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "intel8086-backend: const {n} is outside the selected unsigned result width"
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
    // WORD02 uses AX/BX for at most two live values. CX is a transient
    // expression scratch and never stores a source-visible value.
    let mut slots: [Option<CurrentValue>; 2] = [None, None];
    let (live_before, live_after) = liveness(cir)?;
    let mut labels = HashMap::new();
    let mut fixups: Vec<(usize, String)> = Vec::new();
    let mut label_slots: HashMap<String, (HashSet<String>, [Option<CurrentValue>; 2])> =
        HashMap::new();
    let mut branch_slots: Vec<(String, [Option<CurrentValue>; 2])> = Vec::new();

    // Tracks "has a genuine halt-convention instruction (HLT) already
    // been pushed?" -- an explicit boolean, NOT a trailing-byte-value
    // comparison. See this module's doc for the exact byte-collision bug
    // class this avoids (fixed in four prior lanes of this campaign:
    // Intel 8051, Intel 8080, MOS 6502, Zilog Z80).
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
            require_control_shape(instr, op, 1)?;
            let name = parse_var_src(instr, 0, op)?;
            if labels.insert(name.clone(), bytes.len()).is_some() {
                return Err(BackendError::DuplicateLabel(name));
            }
            label_slots.insert(name, (live_before[index].clone(), slots.clone()));
            terminated = false;
            continue;
        }

        if op == "jmp" {
            require_control_shape(instr, op, 1)?;
            let target = parse_var_src(instr, 0, op)?;
            let patch = bytes.len() + 1;
            bytes.extend_from_slice(&encode_jmp_near(0));
            fixups.push((patch, target.clone()));
            branch_slots.push((target, slots.clone()));
            terminated = false;
            continue;
        }

        if matches!(op, "jmp_if_true" | "jmp_if_false") {
            require_control_shape(instr, op, 2)?;
            let condition = parse_var_src(instr, 0, op)?;
            let target = parse_var_src(instr, 1, op)?;
            let slot = find_slot(&slots, &condition)
                .ok_or_else(|| BackendError::UndefinedVariable(condition.clone()))?;
            let value = slots[slot].as_ref().expect("located slot");
            if value.ty != "bool" || value.width != ResultWidth::Byte {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} condition must be bool"
                )));
            }
            bytes.extend_from_slice(&encode_mov_reg_imm8(REG_CL, 0));
            bytes.extend_from_slice(&encode_alu_reg_reg(
                7,
                false,
                [REG_AL, REG_BL][slot],
                REG_CL,
            ));
            bytes.extend_from_slice(&encode_jcc_short(
                if op == "jmp_if_true" { 4 } else { 5 },
                3,
            ));
            let patch = bytes.len() + 1;
            bytes.extend_from_slice(&encode_jmp_near(0));
            fixups.push((patch, target.clone()));
            branch_slots.push((target, slots.clone()));
            terminated = false;
            continue;
        }

        if op == "ret_void" {
            if instr.dest.is_some() || !instr.srcs.is_empty() || instr.ty != "void" {
                return Err(BackendError::InvalidOperand(
                    "ret_void requires no dest or sources and void type".into(),
                ));
            }
            bytes.extend_from_slice(&encode_hlt());
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
            // The second live value may be in BX; copy it to the ABI
            // register before the outermost halt.
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
                bytes.extend_from_slice(&encode_mov_reg_reg16(REG_AX, REG_BX));
            }
            bytes.extend_from_slice(&encode_hlt());
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
            let imm = encode_typed_immediate(op, instr.srcs.first())?;
            if slots.iter().flatten().any(|value| value.width != width) {
                return Err(BackendError::UnsupportedOp(
                    "mixed-width live values wait for a later Word rung".into(),
                ));
            }
            let slot = slots.iter().position(Option::is_none).ok_or_else(|| {
                BackendError::UnsupportedOp("WORD02 has only two live-value slots".into())
            })?;
            bytes.extend_from_slice(&encode_mov_reg_imm16(
                if slot == 0 { REG_AX } else { REG_BX },
                imm,
            ));
            slots[slot] = Some(CurrentValue {
                name: dest.to_string(),
                width,
                ty: expected_ty.to_string(),
            });
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
            let (left_reg, right_reg, scratch_reg, target_reg) = if width == ResultWidth::Word {
                (
                    [REG_AX, REG_BX][left_slot],
                    [REG_AX, REG_BX][right_slot],
                    REG_CX,
                    [REG_AX, REG_BX][target],
                )
            } else {
                (
                    [REG_AL, REG_BL][left_slot],
                    [REG_AL, REG_BL][right_slot],
                    REG_CL,
                    [REG_AL, REG_BL][target],
                )
            };
            if width == ResultWidth::Word {
                bytes.extend_from_slice(&encode_mov_reg_reg16(scratch_reg, left_reg));
            } else {
                bytes.extend_from_slice(&encode_mov_reg_reg8(scratch_reg, left_reg));
            }
            bytes.extend_from_slice(&encode_alu_reg_reg(
                operation,
                width == ResultWidth::Word,
                scratch_reg,
                right_reg,
            ));
            if width == ResultWidth::Word {
                bytes.extend_from_slice(&encode_mov_reg_reg16(target_reg, scratch_reg));
            } else {
                bytes.extend_from_slice(&encode_mov_reg_reg8(target_reg, scratch_reg));
                bytes.extend_from_slice(&encode_mov_reg_imm8(
                    if target == 0 { REG_AH } else { REG_BH },
                    0,
                ));
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

        if let Some((relation, width)) = comparison_operation(op) {
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
            if [left_slot, right_slot].iter().any(|&slot| {
                let value = slots[slot].as_ref().expect("located slot");
                value.width != width || value.ty != width.name()
            }) {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} source width or type mismatch"
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
                        "WORD03a requires a third live value; spilling is deferred".into(),
                    )
                })?;
            emit_comparison_bool(&mut bytes, relation, width, left_slot, right_slot, target);
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
            let word = width == ResultWidth::Word;
            let mask = if word { 0xffff } else { 0x00ff };
            let source_reg = if word {
                [REG_AX, REG_BX][source]
            } else {
                [REG_AL, REG_BL][source]
            };
            let target_reg = if word {
                [REG_AX, REG_BX][target]
            } else {
                [REG_AL, REG_BL][target]
            };
            bytes.extend_from_slice(&encode_mov_reg_imm16(REG_CX, mask));
            bytes.extend_from_slice(&encode_alu_reg_reg(
                6,
                word,
                if word { REG_CX } else { REG_CL },
                source_reg,
            ));
            if word {
                bytes.extend_from_slice(&encode_mov_reg_reg16(target_reg, REG_CX));
            } else {
                bytes.extend_from_slice(&encode_mov_reg_reg8(target_reg, REG_CL));
                bytes.extend_from_slice(&encode_mov_reg_imm8(
                    if target == 0 { REG_AH } else { REG_BH },
                    0,
                ));
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
    for (patch, target) in fixups {
        let address = *labels
            .get(&target)
            .ok_or_else(|| BackendError::UndefinedLabel(target.clone()))?;
        let next = patch + 2;
        let displacement = address as i64 - next as i64;
        let displacement = i16::try_from(displacement)
            .map_err(|_| BackendError::BranchOutOfRange(target.clone()))?;
        bytes[patch..patch + 2].copy_from_slice(&displacement.to_le_bytes());
    }
    validate_branch_slots(&label_slots, &branch_slots)?;
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

fn require_control_shape(
    instr: &CIRInstr,
    op: &str,
    source_count: usize,
) -> Result<(), BackendError> {
    if instr.dest.is_some() || instr.srcs.len() != source_count || instr.ty != "void" {
        return Err(BackendError::InvalidOperand(format!(
            "{op} requires {source_count} source(s), no dest, and void type"
        )));
    }
    Ok(())
}

type LiveSets = (Vec<HashSet<String>>, Vec<HashSet<String>>);

fn liveness(cir: &[CIRInstr]) -> Result<LiveSets, BackendError> {
    let mut label_indices = HashMap::new();
    for (index, instr) in cir.iter().enumerate() {
        if instr.op == "label" {
            require_control_shape(instr, "label", 1)?;
            let name = parse_var_src(instr, 0, "label")?;
            if label_indices.insert(name.clone(), index).is_some() {
                return Err(BackendError::DuplicateLabel(name));
            }
        }
    }

    let mut successors = vec![Vec::new(); cir.len()];
    let mut predecessors = vec![Vec::new(); cir.len()];
    for (index, instr) in cir.iter().enumerate() {
        match instr.op.as_str() {
            op if op.starts_with("ret_") => {}
            "jmp" => successors[index].push(branch_target_index(instr, 0, &label_indices)?),
            "jmp_if_true" | "jmp_if_false" => {
                successors[index].push(branch_target_index(instr, 1, &label_indices)?);
                if index + 1 < cir.len() {
                    successors[index].push(index + 1);
                }
            }
            _ if index + 1 < cir.len() => successors[index].push(index + 1),
            _ => {}
        }
        for &next in &successors[index] {
            predecessors[next].push(index);
        }
    }
    let mut before = vec![HashSet::new(); cir.len()];
    let mut after = before.clone();
    let mut pending: VecDeque<usize> = (0..cir.len()).rev().collect();
    let mut queued = vec![true; cir.len()];
    while let Some(index) = pending.pop_front() {
        queued[index] = false;
        let instr = &cir[index];
        after[index] = successors[index]
            .iter()
            .flat_map(|&next| before[next].iter().cloned())
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
                "WORD03b requires a third live value; spilling is deferred".into(),
            ));
        }
        if live != before[index] {
            before[index] = live;
            for &prior in &predecessors[index] {
                if !queued[prior] {
                    pending.push_back(prior);
                    queued[prior] = true;
                }
            }
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

fn validate_branch_slots(
    labels: &HashMap<String, (HashSet<String>, [Option<CurrentValue>; 2])>,
    branches: &[(String, [Option<CurrentValue>; 2])],
) -> Result<(), BackendError> {
    for (target, source_slots) in branches {
        let (live, target_slots) = labels
            .get(target)
            .ok_or_else(|| BackendError::UndefinedLabel(target.clone()))?;
        for name in live {
            let source = find_slot(source_slots, name);
            let destination = find_slot(target_slots, name);
            if source != destination
                || source.is_none_or(|slot| source_slots[slot] != target_slots[slot])
            {
                return Err(BackendError::UnsupportedOp(format!(
                    "WORD03b branch to {target:?} has divergent live register mapping"
                )));
            }
        }
    }
    Ok(())
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
        "add" => 0,
        "or" => 1,
        "and" => 4,
        "sub" => 5,
        "xor" => 6,
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

fn comparison_operation(op: &str) -> Option<(&str, ResultWidth)> {
    let (name, width) = op.rsplit_once('_')?;
    let width = match width {
        "u8" => ResultWidth::Byte,
        "u16" => ResultWidth::Word,
        _ => return None,
    };
    let relation = name.strip_prefix("cmp_")?;
    matches!(relation, "eq" | "ne" | "lt" | "le" | "gt" | "ge").then_some((relation, width))
}

fn emit_comparison_bool(
    bytes: &mut Vec<u8>,
    relation: &str,
    width: ResultWidth,
    left: usize,
    right: usize,
    target: usize,
) {
    let regs = if width == ResultWidth::Word {
        [REG_AX, REG_BX]
    } else {
        [REG_AL, REG_BL]
    };
    bytes.extend_from_slice(&encode_alu_reg_reg(
        7,
        width == ResultWidth::Word,
        regs[left],
        regs[right],
    ));
    // MOV leaves CMP flags intact; the inverted short jump skips MOV 1.
    bytes.extend_from_slice(&encode_mov_reg_imm16([REG_AX, REG_BX][target], 0));
    let inverse_condition = match relation {
        "eq" => 5, // JNE
        "ne" => 4, // JE
        "lt" => 3, // JAE
        "le" => 7, // JA
        "gt" => 6, // JBE
        "ge" => 2, // JB
        _ => unreachable!(),
    };
    bytes.extend_from_slice(&encode_jcc_short(inverse_condition, 3));
    bytes.extend_from_slice(&encode_mov_reg_imm16([REG_AX, REG_BX][target], 1));
}

/// Width-preserving materialization through `MOV AX,#imm16`. Byte values are
/// range-checked before being zero-extended into AX; word values accept the
/// full unsigned 16-bit range.
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
        "const_u8" | "const_bool" => Some(ResultWidth::Byte),
        "const_i64" | "const_u16" => Some(ResultWidth::Word),
        _ => None,
    }
}

fn result_width_for_ret(op: &str) -> Option<ResultWidth> {
    match op {
        "ret_u8" | "ret_bool" => Some(ResultWidth::Byte),
        "ret_i64" | "ret_u16" => Some(ResultWidth::Word),
        _ => None,
    }
}

fn encode_typed_immediate(op: &str, operand: Option<&CIROperand>) -> Result<u16, BackendError> {
    if op == "const_bool" {
        return match operand {
            Some(CIROperand::Bool(value)) => Ok(u16::from(*value)),
            _ => Err(BackendError::InvalidOperand(
                "const_bool srcs[0] must be Bool".into(),
            )),
        };
    }

    let n = match operand {
        Some(CIROperand::Int(n)) => *n,
        Some(CIROperand::Bool(b)) if op == "const_i64" => i64::from(*b),
        _ => {
            return Err(BackendError::InvalidOperand(format!(
                "{op} srcs[0] must be Int"
            )));
        }
    };
    let max = if op == "const_u8" { 0xFF } else { 0xFFFF };
    if (0..=max).contains(&n) {
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
