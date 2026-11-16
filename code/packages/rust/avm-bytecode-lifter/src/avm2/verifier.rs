//! Mandatory CFG and Abstract Stack Verification for Flash AVM2 ABC bytecode.

#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet, VecDeque};

use super::abc_file::AbcMethodBody;
use super::opcode::*;
use crate::error::{LifterError, MAX_OPERAND_STACK};

/// Decoded AVM2 instruction record for verification.
#[derive(Debug, Clone)]
pub struct AbcDecodedInstr {
    pub offset: usize,
    pub length: usize,
    pub op: u8,
    pub branch_offset: Option<i32>,
    pub arg_count: usize,
}

/// Verifier for AVM2 control-flow graph (CFG) validity and operand stack bounds.
pub struct Avm2Verifier;

impl Avm2Verifier {
    /// Decode and verify an ABC method body.
    pub fn a2v_verify_method_body(
        body: &AbcMethodBody,
    ) -> Result<Vec<AbcDecodedInstr>, LifterError> {
        let code = &body.code;
        if code.is_empty() {
            return Ok(Vec::new());
        }

        // 1. Decode instructions and record offsets
        let mut instrs = Vec::new();
        let mut pos = 0;
        let mut valid_offsets = HashSet::new();

        while pos < code.len() {
            let start = pos;
            valid_offsets.insert(start);
            let op = code[pos];
            pos = pos
                .checked_add(1)
                .ok_or_else(|| LifterError::InvalidBytecode("offset overflow".to_string()))?;

            let mut branch_offset = None;
            let mut arg_count = 0;

            match op {
                OP_PUSHBYTE => {
                    if pos >= code.len() {
                        return Err(LifterError::UnexpectedEof);
                    }
                    pos = pos.checked_add(1).ok_or_else(|| {
                        LifterError::InvalidBytecode("offset overflow".to_string())
                    })?;
                }
                OP_PUSHSHORT => {
                    let _ = Self::a2v_read_u30(code, &mut pos)?;
                }
                OP_PUSHINT | OP_PUSHUINT | OP_PUSHDOUBLE | OP_PUSHSTRING => {
                    let _ = Self::a2v_read_u30(code, &mut pos)?;
                }
                OP_PUSHTRUE | OP_PUSHFALSE | OP_PUSHNULL | OP_PUSHUNDEFINED | OP_PUSHNAN
                | OP_POP | OP_DUP | OP_SWAP => {}
                OP_GETLOCAL | OP_SETLOCAL => {
                    let _ = Self::a2v_read_u30(code, &mut pos)?;
                }
                OP_GETLOCAL_0 | OP_GETLOCAL_1 | OP_GETLOCAL_2 | OP_GETLOCAL_3 | OP_SETLOCAL_0
                | OP_SETLOCAL_1 | OP_SETLOCAL_2 | OP_SETLOCAL_3 => {}
                OP_ADD | OP_SUBTRACT | OP_MULTIPLY | OP_DIVIDE | OP_MODULO | OP_NEGATE
                | OP_INCREMENT | OP_DECREMENT | OP_NOT => {}
                OP_EQUALS | OP_STRICTEQUALS | OP_LESSTHAN | OP_LESSEQUALS | OP_GREATERTHAN
                | OP_GREATEREQUALS => {}
                OP_JUMP | OP_IFTRUE | OP_IFFALSE | OP_IFEQ | OP_IFNE | OP_IFLT | OP_IFLE
                | OP_IFGT | OP_IFGE => {
                    let slice = code
                        .get(pos..)
                        .and_then(|s| s.get(..3))
                        .ok_or(LifterError::UnexpectedEof)?;
                    pos = pos.checked_add(3).ok_or_else(|| {
                        LifterError::InvalidBytecode("offset overflow".to_string())
                    })?;
                    let b0 = slice[0] as i32;
                    let b1 = slice[1] as i32;
                    let b2 = slice[2] as i32;
                    let u = b0 | (b1 << 8) | (b2 << 16);
                    let s = if (u & 0x800000) != 0 {
                        u | !0xFFFFFF
                    } else {
                        u
                    };
                    branch_offset = Some(s);
                }
                OP_FINDPROPSTRICT | OP_GETPROPERTY | OP_SETPROPERTY | OP_INITPROPERTY => {
                    let _ = Self::a2v_read_u30(code, &mut pos)?;
                }
                OP_CALLPROPERTY | OP_CALLPROPVOID | OP_CONSTRUCTPROP => {
                    let _ = Self::a2v_read_u30(code, &mut pos)?;
                    arg_count = Self::a2v_read_u30(code, &mut pos)? as usize;
                    if arg_count > 256 {
                        return Err(LifterError::MethodBodyLimitExceeded);
                    }
                }
                OP_RETURNVALUE | OP_RETURNVOID => {}
                _ => {
                    // Unknown or unhandled opcode
                    return Err(LifterError::UnsupportedOpcode(op));
                }
            }

            let length = pos - start;
            instrs.push(AbcDecodedInstr {
                offset: start,
                length,
                op,
                branch_offset,
                arg_count,
            });
        }
        // End of code is also a valid target
        valid_offsets.insert(code.len());

        // 2. CFG Branch Target Validation
        let mut offset_to_idx = HashMap::new();
        for (i, instr) in instrs.iter().enumerate() {
            offset_to_idx.insert(instr.offset, i);
        }

        for instr in &instrs {
            if let Some(boff) = instr.branch_offset {
                let next_pc = instr
                    .offset
                    .checked_add(instr.length)
                    .ok_or(LifterError::InvalidBranchTarget)?;
                let target_i64 = (next_pc as i64)
                    .checked_add(boff as i64)
                    .ok_or(LifterError::InvalidBranchTarget)?;
                let target_usize =
                    usize::try_from(target_i64).map_err(|_| LifterError::InvalidBranchTarget)?;
                if !valid_offsets.contains(&target_usize) {
                    return Err(LifterError::InvalidBranchTarget);
                }
            }
        }

        // 3. Abstract Stack Simulation
        let mut entry_depths: HashMap<usize, usize> = HashMap::new();
        let mut queue: VecDeque<usize> = VecDeque::new();

        entry_depths.insert(0, 0);
        queue.push_back(0);

        let max_stack = (body.max_stack as usize).min(MAX_OPERAND_STACK);

        while let Some(idx) = queue.pop_front() {
            if idx >= instrs.len() {
                continue;
            }

            let instr = &instrs[idx];
            let current_depth = *entry_depths.get(&idx).unwrap_or(&0);

            let (pops, pushes) = Self::a2v_stack_effect(instr);
            if current_depth < pops {
                return Err(LifterError::StackUnderflow);
            }

            let new_depth = current_depth - pops + pushes;
            if new_depth > max_stack || new_depth > MAX_OPERAND_STACK {
                return Err(LifterError::StackOverflow);
            }

            // Determine successors
            let mut successors = Vec::new();
            if instr.op == OP_JUMP {
                let boff = instr.branch_offset.unwrap_or(0);
                let next_pc = instr
                    .offset
                    .checked_add(instr.length)
                    .ok_or(LifterError::InvalidBranchTarget)?;
                let target_i64 = (next_pc as i64)
                    .checked_add(boff as i64)
                    .ok_or(LifterError::InvalidBranchTarget)?;
                if let Ok(tgt) = usize::try_from(target_i64) {
                    if let Some(&tgt_idx) = offset_to_idx.get(&tgt) {
                        successors.push(tgt_idx);
                    }
                }
            } else if instr.branch_offset.is_some() {
                // Conditional jump: fallthrough + target
                if idx + 1 < instrs.len() {
                    successors.push(idx + 1);
                }
                let boff = instr.branch_offset.unwrap_or(0);
                let next_pc = instr
                    .offset
                    .checked_add(instr.length)
                    .ok_or(LifterError::InvalidBranchTarget)?;
                let target_i64 = (next_pc as i64)
                    .checked_add(boff as i64)
                    .ok_or(LifterError::InvalidBranchTarget)?;
                if let Ok(tgt) = usize::try_from(target_i64) {
                    if let Some(&tgt_idx) = offset_to_idx.get(&tgt) {
                        successors.push(tgt_idx);
                    }
                }
            } else if instr.op == OP_RETURNVALUE || instr.op == OP_RETURNVOID {
                // Terminators don't fall through
            } else {
                if idx + 1 < instrs.len() {
                    successors.push(idx + 1);
                }
            }

            for succ_idx in successors {
                if let Some(&existing_depth) = entry_depths.get(&succ_idx) {
                    if existing_depth != new_depth {
                        return Err(LifterError::InvalidBytecode(format!(
                            "stack depth mismatch at merge point instruction {}: expected {}, found {}",
                            succ_idx, existing_depth, new_depth
                        )));
                    }
                } else {
                    entry_depths.insert(succ_idx, new_depth);
                    queue.push_back(succ_idx);
                }
            }
        }

        Ok(instrs)
    }

    fn a2v_read_u30(code: &[u8], pos: &mut usize) -> Result<u32, LifterError> {
        let mut result: u32 = 0;
        let mut shift = 0;
        for _ in 0..5 {
            if *pos >= code.len() {
                return Err(LifterError::UnexpectedEof);
            }
            let b = code[*pos];
            *pos = pos
                .checked_add(1)
                .ok_or_else(|| LifterError::InvalidBytecode("offset overflow".to_string()))?;
            result |= ((b & 0x7F) as u32) << shift;
            if (b & 0x80) == 0 {
                if result > 0x3FFF_FFFF {
                    return Err(LifterError::InvalidBytecode(
                        "u30 value exceeds 30 bits (0x3FFF_FFFF)".to_string(),
                    ));
                }
                return Ok(result);
            }
            shift += 7;
        }
        Err(LifterError::InvalidBytecode(
            "u30 LEB128 exceeded 5 bytes".to_string(),
        ))
    }

    fn a2v_stack_effect(instr: &AbcDecodedInstr) -> (usize, usize) {
        match instr.op {
            OP_PUSHBYTE | OP_PUSHSHORT | OP_PUSHINT | OP_PUSHUINT | OP_PUSHDOUBLE
            | OP_PUSHSTRING | OP_PUSHTRUE | OP_PUSHFALSE | OP_PUSHNULL | OP_PUSHUNDEFINED
            | OP_PUSHNAN => (0, 1),
            OP_GETLOCAL | OP_GETLOCAL_0 | OP_GETLOCAL_1 | OP_GETLOCAL_2 | OP_GETLOCAL_3 => (0, 1),
            OP_SETLOCAL | OP_SETLOCAL_0 | OP_SETLOCAL_1 | OP_SETLOCAL_2 | OP_SETLOCAL_3 => (1, 0),
            OP_ADD | OP_SUBTRACT | OP_MULTIPLY | OP_DIVIDE | OP_MODULO => (2, 1),
            OP_EQUALS | OP_STRICTEQUALS | OP_LESSTHAN | OP_LESSEQUALS | OP_GREATERTHAN
            | OP_GREATEREQUALS => (2, 1),
            OP_NEGATE | OP_INCREMENT | OP_DECREMENT | OP_NOT => (1, 1),
            OP_POP => (1, 0),
            OP_DUP => (1, 2),
            OP_SWAP => (2, 2),
            OP_FINDPROPSTRICT => (0, 1),
            OP_GETPROPERTY => (1, 1),
            OP_SETPROPERTY | OP_INITPROPERTY => (2, 0),
            OP_CALLPROPERTY => (1usize.saturating_add(instr.arg_count), 1),
            OP_CALLPROPVOID => (1usize.saturating_add(instr.arg_count), 0),
            OP_CONSTRUCTPROP => (1usize.saturating_add(instr.arg_count), 1),
            OP_JUMP => (0, 0),
            OP_IFTRUE | OP_IFFALSE => (1, 0),
            OP_IFEQ | OP_IFNE | OP_IFLT | OP_IFLE | OP_IFGT | OP_IFGE => (2, 0),
            OP_RETURNVALUE => (1, 0),
            OP_RETURNVOID => (0, 0),
            _ => (0, 0),
        }
    }
}
