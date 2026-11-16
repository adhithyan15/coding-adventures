//! Mandatory CFG and Abstract Stack Verification for Flash AVM1 bytecode.

#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet, VecDeque};

use super::action::{Avm1Action, Avm1ActionKind};
use crate::error::{LifterError, MAX_ACTIONS, MAX_FUNCTION_DEPTH, MAX_OPERAND_STACK};

/// Verifier for AVM1 control-flow graph (CFG) validity and operand stack safety.
pub struct Avm1Verifier;

impl Avm1Verifier {
    /// Verify CFG branch targets and simulate operand stack safety for a slice of AVM1 actions.
    pub fn a1v_verify(actions: &[Avm1Action], total_byte_len: usize) -> Result<(), LifterError> {
        let mut total_instructions = 0;
        Self::a1v_verify_inner(actions, total_byte_len, 0, &mut total_instructions)
    }

    fn a1v_verify_inner(
        actions: &[Avm1Action],
        total_byte_len: usize,
        depth: usize,
        total_instructions: &mut usize,
    ) -> Result<(), LifterError> {
        if depth > MAX_FUNCTION_DEPTH {
            return Err(LifterError::FunctionLimitExceeded);
        }

        if actions.is_empty() {
            return Ok(());
        }

        // 1. Build map of byte offset -> action index and track total instructions
        let mut offset_to_idx = HashMap::new();
        let mut valid_offsets = HashSet::new();

        for (idx, action) in actions.iter().enumerate() {
            *total_instructions = total_instructions
                .checked_add(1)
                .ok_or(LifterError::InstructionLimitExceeded)?;
            if *total_instructions > MAX_ACTIONS {
                return Err(LifterError::InstructionLimitExceeded);
            }
            offset_to_idx.insert(action.offset, idx);
            valid_offsets.insert(action.offset);
        }
        // The end of bytecode is also a valid jump target for jumping past all actions
        valid_offsets.insert(total_byte_len);

        // 2. CFG Branch Target Validation
        for action in actions {
            match &action.kind {
                Avm1ActionKind::BranchAlways(offset) | Avm1ActionKind::BranchIfTrue(offset) => {
                    let next_pc = action
                        .offset
                        .checked_add(action.length)
                        .ok_or(LifterError::InvalidBranchTarget)?;
                    let target_i64 = (next_pc as i64)
                        .checked_add(*offset as i64)
                        .ok_or(LifterError::InvalidBranchTarget)?;
                    let target_usize = usize::try_from(target_i64)
                        .map_err(|_| LifterError::InvalidBranchTarget)?;
                    if !valid_offsets.contains(&target_usize) {
                        return Err(LifterError::InvalidBranchTarget);
                    }
                }
                Avm1ActionKind::DefineFunction { body, body_len, .. }
                | Avm1ActionKind::DefineFunction2 { body, body_len, .. } => {
                    // Recursively verify nested function bodies with depth limit and shared instruction budget
                    Self::a1v_verify_inner(body, *body_len, depth + 1, total_instructions)?;
                }
                _ => {}
            }
        }

        // 3. Abstract Stack Simulation
        let mut entry_depths: HashMap<usize, usize> = HashMap::new();
        let mut queue: VecDeque<usize> = VecDeque::new();

        entry_depths.insert(0, 0);
        queue.push_back(0);

        while let Some(idx) = queue.pop_front() {
            if idx >= actions.len() {
                continue;
            }

            let action = &actions[idx];
            let current_depth = *entry_depths.get(&idx).unwrap_or(&0);

            let (pops, pushes) = Self::a1v_stack_effect(action);
            if current_depth < pops {
                return Err(LifterError::StackUnderflow);
            }

            let new_depth = current_depth - pops + pushes;
            if new_depth > MAX_OPERAND_STACK {
                return Err(LifterError::StackOverflow);
            }

            // Determine CFG successors
            let mut successors = Vec::new();
            match &action.kind {
                Avm1ActionKind::BranchAlways(offset) => {
                    let next_pc = action
                        .offset
                        .checked_add(action.length)
                        .ok_or(LifterError::InvalidBranchTarget)?;
                    let target_i64 = (next_pc as i64)
                        .checked_add(*offset as i64)
                        .ok_or(LifterError::InvalidBranchTarget)?;
                    if let Ok(target_usize) = usize::try_from(target_i64) {
                        if let Some(&tgt_idx) = offset_to_idx.get(&target_usize) {
                            successors.push(tgt_idx);
                        }
                    }
                }
                Avm1ActionKind::BranchIfTrue(offset) => {
                    // Fallthrough
                    if idx + 1 < actions.len() {
                        successors.push(idx + 1);
                    }
                    // Branch target
                    let next_pc = action
                        .offset
                        .checked_add(action.length)
                        .ok_or(LifterError::InvalidBranchTarget)?;
                    let target_i64 = (next_pc as i64)
                        .checked_add(*offset as i64)
                        .ok_or(LifterError::InvalidBranchTarget)?;
                    if let Ok(target_usize) = usize::try_from(target_i64) {
                        if let Some(&tgt_idx) = offset_to_idx.get(&target_usize) {
                            successors.push(tgt_idx);
                        }
                    }
                }
                Avm1ActionKind::Return | Avm1ActionKind::End => {
                    // Terminators do not fall through
                }
                _ => {
                    // Normal instruction falls through to next
                    if idx + 1 < actions.len() {
                        successors.push(idx + 1);
                    }
                }
            }

            // Propagate stack depth to successors and reject stack height mismatches at merge points
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

        // 4. Also simulate any unreached basic blocks starting with depth 0
        for start_idx in 0..actions.len() {
            if let std::collections::hash_map::Entry::Vacant(e) = entry_depths.entry(start_idx) {
                e.insert(0);
                queue.push_back(start_idx);

                while let Some(curr) = queue.pop_front() {
                    if curr >= actions.len() {
                        continue;
                    }
                    let action = &actions[curr];
                    let current_depth = *entry_depths.get(&curr).unwrap_or(&0);
                    let (pops, pushes) = Self::a1v_stack_effect(action);
                    if current_depth < pops {
                        return Err(LifterError::StackUnderflow);
                    }
                    let new_depth = current_depth - pops + pushes;
                    if new_depth > MAX_OPERAND_STACK {
                        return Err(LifterError::StackOverflow);
                    }
                    let mut succs = Vec::new();
                    match &action.kind {
                        Avm1ActionKind::BranchAlways(offset) => {
                            let next_pc = action
                                .offset
                                .checked_add(action.length)
                                .ok_or(LifterError::InvalidBranchTarget)?;
                            let target_i64 = (next_pc as i64)
                                .checked_add(*offset as i64)
                                .ok_or(LifterError::InvalidBranchTarget)?;
                            if let Ok(target_usize) = usize::try_from(target_i64) {
                                if let Some(&tgt_idx) = offset_to_idx.get(&target_usize) {
                                    succs.push(tgt_idx);
                                }
                            }
                        }
                        Avm1ActionKind::BranchIfTrue(offset) => {
                            if curr + 1 < actions.len() {
                                succs.push(curr + 1);
                            }
                            let next_pc = action
                                .offset
                                .checked_add(action.length)
                                .ok_or(LifterError::InvalidBranchTarget)?;
                            let target_i64 = (next_pc as i64)
                                .checked_add(*offset as i64)
                                .ok_or(LifterError::InvalidBranchTarget)?;
                            if let Ok(target_usize) = usize::try_from(target_i64) {
                                if let Some(&tgt_idx) = offset_to_idx.get(&target_usize) {
                                    succs.push(tgt_idx);
                                }
                            }
                        }
                        Avm1ActionKind::Return | Avm1ActionKind::End => {}
                        _ => {
                            if curr + 1 < actions.len() {
                                succs.push(curr + 1);
                            }
                        }
                    }
                    for s in succs {
                        if let Some(&existing_depth) = entry_depths.get(&s) {
                            if existing_depth != new_depth {
                                return Err(LifterError::InvalidBytecode(format!(
                                    "stack depth mismatch at unreached merge point instruction {}: expected {}, found {}",
                                    s, existing_depth, new_depth
                                )));
                            }
                        } else {
                            entry_depths.insert(s, new_depth);
                            queue.push_back(s);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn a1v_stack_effect(action: &Avm1Action) -> (usize, usize) {
        match &action.kind {
            Avm1ActionKind::End => (0, 0),
            Avm1ActionKind::NextFrame
            | Avm1ActionKind::PreviousFrame
            | Avm1ActionKind::Play
            | Avm1ActionKind::Stop
            | Avm1ActionKind::ToggleQuality
            | Avm1ActionKind::StopSounds => (0, 0),
            Avm1ActionKind::Add
            | Avm1ActionKind::Subtract
            | Avm1ActionKind::Multiply
            | Avm1ActionKind::Divide
            | Avm1ActionKind::Equals
            | Avm1ActionKind::Less
            | Avm1ActionKind::And
            | Avm1ActionKind::Or
            | Avm1ActionKind::StringEquals
            | Avm1ActionKind::StringAdd
            | Avm1ActionKind::Add2
            | Avm1ActionKind::Less2
            | Avm1ActionKind::Equals2 => (2, 1),
            Avm1ActionKind::Not
            | Avm1ActionKind::StringLength
            | Avm1ActionKind::ToInteger
            | Avm1ActionKind::ToNumber
            | Avm1ActionKind::ToString
            | Avm1ActionKind::CharToAscii
            | Avm1ActionKind::AsciiToChar => (1, 1),
            Avm1ActionKind::StringExtract => (3, 1),
            Avm1ActionKind::Pop => (1, 0),
            Avm1ActionKind::Push(values) => (0, values.len()),
            Avm1ActionKind::GetVariable => (1, 1),
            Avm1ActionKind::SetVariable => (2, 0),
            Avm1ActionKind::GetMember => (2, 1),
            Avm1ActionKind::SetMember => (3, 0),
            Avm1ActionKind::Trace => (1, 0),
            Avm1ActionKind::GetTime => (0, 1),
            Avm1ActionKind::CallFunction => (2, 1),
            Avm1ActionKind::CallMethod => (3, 1),
            Avm1ActionKind::NewObject => (2, 1),
            Avm1ActionKind::GotoFrame(_) => (0, 0),
            Avm1ActionKind::StoreRegister(_) => (0, 0),
            Avm1ActionKind::ConstantPool(_) => (0, 0),
            Avm1ActionKind::BranchAlways(_) => (0, 0),
            Avm1ActionKind::BranchIfTrue(_) => (1, 0),
            Avm1ActionKind::Return => (0, 0),
            Avm1ActionKind::DefineFunction { name, .. }
            | Avm1ActionKind::DefineFunction2 { name, .. } => {
                // Anonymous functions push the function object to the stack; named functions assign to variable in current scope
                if name.is_empty() {
                    (0, 1)
                } else {
                    (0, 0)
                }
            }
        }
    }
}
