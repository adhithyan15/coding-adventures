//! Translation from verified AVM2 ABC bytecode into `InterpreterIR` (`LANG01`) modules.

#![deny(unsafe_code)]

use std::collections::HashSet;

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;

use super::abc_file::{AbcFile, AbcMethodBody};
use super::opcode::*;
use super::verifier::Avm2Verifier;
use crate::error::{LifterError, MAX_INSTRUCTIONS};

/// AVM2 ABC bytecode lifter targeting InterpreterIR.
pub struct Avm2Lifter {
    module_name: String,
    instruction_count: usize,
    temp_counter: usize,
}

impl Avm2Lifter {
    /// Create a new AVM2 lifter.
    pub fn a2l_new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            instruction_count: 0,
            temp_counter: 0,
        }
    }

    /// Lift an entire ABC file into an `IIRModule`.
    pub fn a2l_lift_file(&mut self, abc: &AbcFile) -> Result<IIRModule, LifterError> {
        let mut module = IIRModule::new(&self.module_name, "flash_avm2");
        module.entry_point = Some("main".to_string());

        // Find entry method index
        let entry_method_idx = abc.scripts.first().map(|s| s.init_method).unwrap_or(0);

        for (body_idx, body) in abc.method_bodies.iter().enumerate() {
            let fn_name = if body.method_index == entry_method_idx {
                "main".to_string()
            } else {
                format!("method_{}", body.method_index)
            };

            let instrs = self.a2l_lift_body(abc, body, &fn_name, body_idx)?;
            let f = IIRFunction::new(&fn_name, vec![], "any", instrs);
            module.add_or_replace(f);
        }

        // If no method bodies were present, ensure a dummy main function
        if module.get_function("main").is_none() {
            let empty_main = IIRFunction::new(
                "main",
                vec![],
                "void",
                vec![IIRInstr::new("ret_void", None, vec![], "void")],
            );
            module.add_or_replace(empty_main);
        }

        let val_errs = module.validate();
        if !val_errs.is_empty() {
            return Err(LifterError::InvalidBytecode(format!(
                "AVM2 module validation failed: {}",
                val_errs.join("; ")
            )));
        }

        Ok(module)
    }

    fn a2l_fresh_temp(&mut self) -> String {
        let t = format!("%a2t{}", self.temp_counter);
        self.temp_counter += 1;
        t
    }

    fn a2l_emit(&mut self, out: &mut Vec<IIRInstr>, instr: IIRInstr) -> Result<(), LifterError> {
        self.instruction_count += 1;
        if self.instruction_count > MAX_INSTRUCTIONS {
            return Err(LifterError::InstructionLimitExceeded);
        }
        out.push(instr);
        Ok(())
    }

    fn a2l_lift_body(
        &mut self,
        abc: &AbcFile,
        body: &AbcMethodBody,
        fn_name: &str,
        _body_idx: usize,
    ) -> Result<Vec<IIRInstr>, LifterError> {
        let decoded = Avm2Verifier::a2v_verify_method_body(body)?;
        let mut instrs = Vec::new();

        // Identify jump targets with checked arithmetic
        let mut jump_targets = HashSet::new();
        for dinstr in &decoded {
            if let Some(boff) = dinstr.branch_offset {
                if let Some(next_pc) = dinstr.offset.checked_add(dinstr.length) {
                    if let Some(tgt_i64) = (next_pc as i64).checked_add(boff as i64) {
                        if let Ok(tgt) = usize::try_from(tgt_i64) {
                            jump_targets.insert(tgt);
                        }
                    }
                }
            }
        }

        let code = &body.code;
        let mut stack_depth: usize = 0;

        for dinstr in &decoded {
            if jump_targets.contains(&dinstr.offset) {
                let lbl = format!("{}_off_{}", fn_name, dinstr.offset);
                self.a2l_emit(
                    &mut instrs,
                    IIRInstr::new("label", None, vec![Operand::Var(lbl)], "void"),
                )?;
            }

            let mut pos = dinstr.offset + 1; // start reading operands

            match dinstr.op {
                OP_PUSHBYTE => {
                    let b = code[pos] as i8 as i64;
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("const", Some(dest), vec![Operand::Int(b)], "any"),
                    )?;
                }
                OP_PUSHSHORT => {
                    let s = Self::a2l_read_u30(code, &mut pos)? as i16 as i64;
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("const", Some(dest), vec![Operand::Int(s)], "any"),
                    )?;
                }
                OP_PUSHINT => {
                    let idx = Self::a2l_read_u30(code, &mut pos)? as usize;
                    let val = abc.constant_pool.cpool_get_int(idx) as i64;
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("const", Some(dest), vec![Operand::Int(val)], "any"),
                    )?;
                }
                OP_PUSHUINT => {
                    let idx = Self::a2l_read_u30(code, &mut pos)? as usize;
                    let val = abc.constant_pool.cpool_get_uint(idx) as i64;
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("const", Some(dest), vec![Operand::Int(val)], "any"),
                    )?;
                }
                OP_PUSHDOUBLE => {
                    let idx = Self::a2l_read_u30(code, &mut pos)? as usize;
                    let val = abc.constant_pool.cpool_get_double(idx);
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("const", Some(dest), vec![Operand::Float(val)], "any"),
                    )?;
                }
                OP_PUSHSTRING => {
                    let idx = Self::a2l_read_u30(code, &mut pos)? as usize;
                    let val = abc.constant_pool.cpool_get_string(idx).to_string();
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("const", Some(dest), vec![Operand::Str(val)], "any"),
                    )?;
                }
                OP_PUSHTRUE => {
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("const", Some(dest), vec![Operand::Bool(true)], "any"),
                    )?;
                }
                OP_PUSHFALSE => {
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("const", Some(dest), vec![Operand::Bool(false)], "any"),
                    )?;
                }
                OP_PUSHNULL => {
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new(
                            "call_builtin",
                            Some(dest),
                            vec![Operand::Var("flash_null".to_string())],
                            "any",
                        ),
                    )?;
                }
                OP_PUSHUNDEFINED => {
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new(
                            "call_builtin",
                            Some(dest),
                            vec![Operand::Var("flash_undefined".to_string())],
                            "any",
                        ),
                    )?;
                }
                OP_PUSHNAN => {
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("const", Some(dest), vec![Operand::Float(f64::NAN)], "any"),
                    )?;
                }
                OP_POP => {
                    stack_depth = stack_depth.saturating_sub(1);
                }
                OP_DUP => {
                    if stack_depth > 0 {
                        let top = format!("%s{}", stack_depth - 1);
                        let dest = format!("%s{}", stack_depth);
                        stack_depth += 1;
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new("mov", Some(dest), vec![Operand::Var(top)], "any"),
                        )?;
                    }
                }
                OP_SWAP => {
                    if stack_depth >= 2 {
                        let top = format!("%s{}", stack_depth - 1);
                        let under = format!("%s{}", stack_depth - 2);
                        let tmp = self.a2l_fresh_temp();
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "mov",
                                Some(tmp.clone()),
                                vec![Operand::Var(top.clone())],
                                "any",
                            ),
                        )?;
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "mov",
                                Some(top),
                                vec![Operand::Var(under.clone())],
                                "any",
                            ),
                        )?;
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new("mov", Some(under), vec![Operand::Var(tmp)], "any"),
                        )?;
                    }
                }
                OP_GETLOCAL => {
                    let reg = Self::a2l_read_u30(code, &mut pos)?;
                    let src = format!("%local_{}", reg);
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("mov", Some(dest), vec![Operand::Var(src)], "any"),
                    )?;
                }
                OP_SETLOCAL => {
                    let reg = Self::a2l_read_u30(code, &mut pos)?;
                    if stack_depth > 0 {
                        stack_depth -= 1;
                        let src = format!("%s{}", stack_depth);
                        let dest = format!("%local_{}", reg);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new("mov", Some(dest), vec![Operand::Var(src)], "any"),
                        )?;
                    }
                }
                OP_GETLOCAL_0 | OP_GETLOCAL_1 | OP_GETLOCAL_2 | OP_GETLOCAL_3 => {
                    let reg = dinstr.op - OP_GETLOCAL_0;
                    let src = format!("%local_{}", reg);
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("mov", Some(dest), vec![Operand::Var(src)], "any"),
                    )?;
                }
                OP_SETLOCAL_0 | OP_SETLOCAL_1 | OP_SETLOCAL_2 | OP_SETLOCAL_3 => {
                    let reg = dinstr.op - OP_SETLOCAL_0;
                    if stack_depth > 0 {
                        stack_depth -= 1;
                        let src = format!("%s{}", stack_depth);
                        let dest = format!("%local_{}", reg);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new("mov", Some(dest), vec![Operand::Var(src)], "any"),
                        )?;
                    }
                }
                OP_ADD => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_add2".to_string()),
                                    Operand::Var(lhs),
                                    Operand::Var(rhs),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_SUBTRACT => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "sub",
                                Some(dest),
                                vec![Operand::Var(lhs), Operand::Var(rhs)],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_MULTIPLY => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "mul",
                                Some(dest),
                                vec![Operand::Var(lhs), Operand::Var(rhs)],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_DIVIDE => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "div",
                                Some(dest),
                                vec![Operand::Var(lhs), Operand::Var(rhs)],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_MODULO => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_modulo".to_string()),
                                    Operand::Var(lhs),
                                    Operand::Var(rhs),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_NEGATE => {
                    if stack_depth >= 1 {
                        let operand = format!("%s{}", stack_depth - 1);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(operand.clone()),
                                vec![
                                    Operand::Var("flash_negate".to_string()),
                                    Operand::Var(operand),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_INCREMENT => {
                    if stack_depth >= 1 {
                        let operand = format!("%s{}", stack_depth - 1);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(operand.clone()),
                                vec![
                                    Operand::Var("flash_increment".to_string()),
                                    Operand::Var(operand),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_DECREMENT => {
                    if stack_depth >= 1 {
                        let operand = format!("%s{}", stack_depth - 1);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(operand.clone()),
                                vec![
                                    Operand::Var("flash_decrement".to_string()),
                                    Operand::Var(operand),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_NOT => {
                    if stack_depth >= 1 {
                        let operand = format!("%s{}", stack_depth - 1);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(operand.clone()),
                                vec![Operand::Var("flash_not".to_string()), Operand::Var(operand)],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_EQUALS | OP_STRICTEQUALS => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_equals2".to_string()),
                                    Operand::Var(lhs),
                                    Operand::Var(rhs),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_LESSTHAN => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_less2".to_string()),
                                    Operand::Var(lhs),
                                    Operand::Var(rhs),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_LESSEQUALS => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_le2".to_string()),
                                    Operand::Var(lhs),
                                    Operand::Var(rhs),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_GREATERTHAN => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_gt2".to_string()),
                                    Operand::Var(lhs),
                                    Operand::Var(rhs),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_GREATEREQUALS => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_ge2".to_string()),
                                    Operand::Var(lhs),
                                    Operand::Var(rhs),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_JUMP => {
                    let boff = dinstr.branch_offset.unwrap_or(0);
                    let next_pc = dinstr
                        .offset
                        .checked_add(dinstr.length)
                        .ok_or(LifterError::InvalidBranchTarget)?;
                    let tgt_i64 = (next_pc as i64)
                        .checked_add(boff as i64)
                        .ok_or(LifterError::InvalidBranchTarget)?;
                    let tgt =
                        usize::try_from(tgt_i64).map_err(|_| LifterError::InvalidBranchTarget)?;
                    let target_lbl = format!("{}_off_{}", fn_name, tgt);
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new("jmp", None, vec![Operand::Var(target_lbl)], "void"),
                    )?;
                }
                OP_IFTRUE => {
                    if stack_depth >= 1 {
                        let cond = format!("%s{}", stack_depth - 1);
                        stack_depth -= 1;
                        let boff = dinstr.branch_offset.unwrap_or(0);
                        let next_pc = dinstr
                            .offset
                            .checked_add(dinstr.length)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt_i64 = (next_pc as i64)
                            .checked_add(boff as i64)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt = usize::try_from(tgt_i64)
                            .map_err(|_| LifterError::InvalidBranchTarget)?;
                        let target_lbl = format!("{}_off_{}", fn_name, tgt);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "jmp_if_true",
                                None,
                                vec![Operand::Var(cond), Operand::Var(target_lbl)],
                                "void",
                            ),
                        )?;
                    }
                }
                OP_IFFALSE => {
                    if stack_depth >= 1 {
                        let cond = format!("%s{}", stack_depth - 1);
                        stack_depth -= 1;
                        let boff = dinstr.branch_offset.unwrap_or(0);
                        let next_pc = dinstr
                            .offset
                            .checked_add(dinstr.length)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt_i64 = (next_pc as i64)
                            .checked_add(boff as i64)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt = usize::try_from(tgt_i64)
                            .map_err(|_| LifterError::InvalidBranchTarget)?;
                        let target_lbl = format!("{}_off_{}", fn_name, tgt);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "jmp_if_false",
                                None,
                                vec![Operand::Var(cond), Operand::Var(target_lbl)],
                                "void",
                            ),
                        )?;
                    }
                }
                OP_IFLT => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 2;
                        let cond_temp = format!("%s{}", stack_depth);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "cmp_lt",
                                Some(cond_temp.clone()),
                                vec![Operand::Var(lhs), Operand::Var(rhs)],
                                "bool",
                            ),
                        )?;
                        let boff = dinstr.branch_offset.unwrap_or(0);
                        let next_pc = dinstr
                            .offset
                            .checked_add(dinstr.length)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt_i64 = (next_pc as i64)
                            .checked_add(boff as i64)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt = usize::try_from(tgt_i64)
                            .map_err(|_| LifterError::InvalidBranchTarget)?;
                        let target_lbl = format!("{}_off_{}", fn_name, tgt);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "jmp_if_true",
                                None,
                                vec![Operand::Var(cond_temp), Operand::Var(target_lbl)],
                                "void",
                            ),
                        )?;
                    }
                }
                OP_IFLE => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 2;
                        let cond_temp = format!("%s{}", stack_depth);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "cmp_le",
                                Some(cond_temp.clone()),
                                vec![Operand::Var(lhs), Operand::Var(rhs)],
                                "bool",
                            ),
                        )?;
                        let boff = dinstr.branch_offset.unwrap_or(0);
                        let next_pc = dinstr
                            .offset
                            .checked_add(dinstr.length)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt_i64 = (next_pc as i64)
                            .checked_add(boff as i64)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt = usize::try_from(tgt_i64)
                            .map_err(|_| LifterError::InvalidBranchTarget)?;
                        let target_lbl = format!("{}_off_{}", fn_name, tgt);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "jmp_if_true",
                                None,
                                vec![Operand::Var(cond_temp), Operand::Var(target_lbl)],
                                "void",
                            ),
                        )?;
                    }
                }
                OP_IFGT => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 2;
                        let cond_temp = format!("%s{}", stack_depth);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "cmp_gt",
                                Some(cond_temp.clone()),
                                vec![Operand::Var(lhs), Operand::Var(rhs)],
                                "bool",
                            ),
                        )?;
                        let boff = dinstr.branch_offset.unwrap_or(0);
                        let next_pc = dinstr
                            .offset
                            .checked_add(dinstr.length)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt_i64 = (next_pc as i64)
                            .checked_add(boff as i64)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt = usize::try_from(tgt_i64)
                            .map_err(|_| LifterError::InvalidBranchTarget)?;
                        let target_lbl = format!("{}_off_{}", fn_name, tgt);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "jmp_if_true",
                                None,
                                vec![Operand::Var(cond_temp), Operand::Var(target_lbl)],
                                "void",
                            ),
                        )?;
                    }
                }
                OP_IFGE => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 2;
                        let cond_temp = format!("%s{}", stack_depth);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "cmp_ge",
                                Some(cond_temp.clone()),
                                vec![Operand::Var(lhs), Operand::Var(rhs)],
                                "bool",
                            ),
                        )?;
                        let boff = dinstr.branch_offset.unwrap_or(0);
                        let next_pc = dinstr
                            .offset
                            .checked_add(dinstr.length)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt_i64 = (next_pc as i64)
                            .checked_add(boff as i64)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt = usize::try_from(tgt_i64)
                            .map_err(|_| LifterError::InvalidBranchTarget)?;
                        let target_lbl = format!("{}_off_{}", fn_name, tgt);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "jmp_if_true",
                                None,
                                vec![Operand::Var(cond_temp), Operand::Var(target_lbl)],
                                "void",
                            ),
                        )?;
                    }
                }
                OP_IFEQ => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 2;
                        let cond_temp = format!("%s{}", stack_depth);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "cmp_eq",
                                Some(cond_temp.clone()),
                                vec![Operand::Var(lhs), Operand::Var(rhs)],
                                "bool",
                            ),
                        )?;
                        let boff = dinstr.branch_offset.unwrap_or(0);
                        let next_pc = dinstr
                            .offset
                            .checked_add(dinstr.length)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt_i64 = (next_pc as i64)
                            .checked_add(boff as i64)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt = usize::try_from(tgt_i64)
                            .map_err(|_| LifterError::InvalidBranchTarget)?;
                        let target_lbl = format!("{}_off_{}", fn_name, tgt);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "jmp_if_true",
                                None,
                                vec![Operand::Var(cond_temp), Operand::Var(target_lbl)],
                                "void",
                            ),
                        )?;
                    }
                }
                OP_IFNE => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 2;
                        let cond_temp = format!("%s{}", stack_depth);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "cmp_ne",
                                Some(cond_temp.clone()),
                                vec![Operand::Var(lhs), Operand::Var(rhs)],
                                "bool",
                            ),
                        )?;
                        let boff = dinstr.branch_offset.unwrap_or(0);
                        let next_pc = dinstr
                            .offset
                            .checked_add(dinstr.length)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt_i64 = (next_pc as i64)
                            .checked_add(boff as i64)
                            .ok_or(LifterError::InvalidBranchTarget)?;
                        let tgt = usize::try_from(tgt_i64)
                            .map_err(|_| LifterError::InvalidBranchTarget)?;
                        let target_lbl = format!("{}_off_{}", fn_name, tgt);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "jmp_if_true",
                                None,
                                vec![Operand::Var(cond_temp), Operand::Var(target_lbl)],
                                "void",
                            ),
                        )?;
                    }
                }
                OP_FINDPROPSTRICT => {
                    let mn_idx = Self::a2l_read_u30(code, &mut pos)? as usize;
                    let prop_name = abc
                        .constant_pool
                        .cpool_get_multiname_name(mn_idx)
                        .to_string();
                    let dest = format!("%s{}", stack_depth);
                    stack_depth += 1;
                    self.a2l_emit(
                        &mut instrs,
                        IIRInstr::new(
                            "call_builtin",
                            Some(dest),
                            vec![
                                Operand::Var("flash_find_property".to_string()),
                                Operand::Str(prop_name),
                            ],
                            "any",
                        ),
                    )?;
                }
                OP_GETPROPERTY => {
                    let mn_idx = Self::a2l_read_u30(code, &mut pos)? as usize;
                    let prop_name = abc
                        .constant_pool
                        .cpool_get_multiname_name(mn_idx)
                        .to_string();
                    if stack_depth >= 1 {
                        let obj = format!("%s{}", stack_depth - 1);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(obj.clone()),
                                vec![
                                    Operand::Var("flash_get_member".to_string()),
                                    Operand::Var(obj),
                                    Operand::Str(prop_name),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_SETPROPERTY | OP_INITPROPERTY => {
                    let mn_idx = Self::a2l_read_u30(code, &mut pos)? as usize;
                    let prop_name = abc
                        .constant_pool
                        .cpool_get_multiname_name(mn_idx)
                        .to_string();
                    if stack_depth >= 2 {
                        let val = format!("%s{}", stack_depth - 1);
                        let obj = format!("%s{}", stack_depth - 2);
                        stack_depth -= 2;
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                None,
                                vec![
                                    Operand::Var("flash_set_member".to_string()),
                                    Operand::Var(obj),
                                    Operand::Str(prop_name),
                                    Operand::Var(val),
                                ],
                                "void",
                            ),
                        )?;
                    }
                }
                OP_CALLPROPERTY => {
                    let mn_idx = Self::a2l_read_u30(code, &mut pos)? as usize;
                    let method_name = abc
                        .constant_pool
                        .cpool_get_multiname_name(mn_idx)
                        .to_string();
                    let arg_count = dinstr.arg_count;
                    if stack_depth > arg_count {
                        stack_depth -= 1 + arg_count;
                        let dest = format!("%s{}", stack_depth);
                        stack_depth += 1;
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_call_property".to_string()),
                                    Operand::Str(method_name),
                                    Operand::Int(arg_count as i64),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_CALLPROPVOID => {
                    let mn_idx = Self::a2l_read_u30(code, &mut pos)? as usize;
                    let method_name = abc
                        .constant_pool
                        .cpool_get_multiname_name(mn_idx)
                        .to_string();
                    let arg_count = dinstr.arg_count;
                    if stack_depth > arg_count {
                        stack_depth -= 1 + arg_count;
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                None,
                                vec![
                                    Operand::Var("flash_call_propvoid".to_string()),
                                    Operand::Str(method_name),
                                    Operand::Int(arg_count as i64),
                                ],
                                "void",
                            ),
                        )?;
                    }
                }
                OP_CONSTRUCTPROP => {
                    let mn_idx = Self::a2l_read_u30(code, &mut pos)? as usize;
                    let class_name = abc
                        .constant_pool
                        .cpool_get_multiname_name(mn_idx)
                        .to_string();
                    let arg_count = dinstr.arg_count;
                    if stack_depth > arg_count {
                        stack_depth -= 1 + arg_count;
                        let dest = format!("%s{}", stack_depth);
                        stack_depth += 1;
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_construct_property".to_string()),
                                    Operand::Str(class_name),
                                    Operand::Int(arg_count as i64),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                OP_RETURNVALUE => {
                    if stack_depth >= 1 {
                        let ret_val = format!("%s{}", stack_depth - 1);
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new("ret", None, vec![Operand::Var(ret_val)], "any"),
                        )?;
                    } else {
                        self.a2l_emit(
                            &mut instrs,
                            IIRInstr::new("ret_void", None, vec![], "void"),
                        )?;
                    }
                }
                OP_RETURNVOID => {
                    self.a2l_emit(&mut instrs, IIRInstr::new("ret_void", None, vec![], "void"))?;
                }
                _ => {}
            }
        }

        if jump_targets.contains(&code.len()) {
            let end_lbl = format!("{}_off_{}", fn_name, code.len());
            self.a2l_emit(
                &mut instrs,
                IIRInstr::new("label", None, vec![Operand::Var(end_lbl)], "void"),
            )?;
        }

        let ends_with_ret = instrs
            .last()
            .is_some_and(|i| i.op == "ret" || i.op == "ret_void");
        if !ends_with_ret {
            self.a2l_emit(&mut instrs, IIRInstr::new("ret_void", None, vec![], "void"))?;
        }

        Ok(instrs)
    }

    fn a2l_read_u30(code: &[u8], pos: &mut usize) -> Result<u32, LifterError> {
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
}
