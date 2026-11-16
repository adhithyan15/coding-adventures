//! Translation from verified AVM1 actions into `InterpreterIR` (`LANG01`) modules.

#![deny(unsafe_code)]

use std::collections::HashSet;

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;

use super::action::{Avm1Action, Avm1ActionKind};
use super::value::Avm1Value;
use super::verifier::Avm1Verifier;
use crate::error::{LifterError, MAX_INSTRUCTIONS};

/// AVM1 bytecode lifter targeting InterpreterIR.
pub struct Avm1Lifter {
    module_name: String,
    constant_pool: Vec<String>,
    temp_counter: usize,
    instruction_count: usize,
    functions_to_emit: Vec<IIRFunction>,
}

impl Avm1Lifter {
    /// Create a new AVM1 lifter.
    pub fn a1l_new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            constant_pool: Vec::new(),
            temp_counter: 0,
            instruction_count: 0,
            functions_to_emit: Vec::new(),
        }
    }

    /// Lift verified AVM1 actions into an `IIRModule`.
    pub fn a1l_lift_module(
        &mut self,
        actions: &[Avm1Action],
        total_len: usize,
    ) -> Result<IIRModule, LifterError> {
        // Run mandatory verification first
        Avm1Verifier::a1v_verify(actions, total_len)?;

        let mut module = IIRModule::new(&self.module_name, "flash_avm1");
        module.entry_point = Some("main".to_string());

        // Extract constant pool declarations upfront if any
        for action in actions {
            if let Avm1ActionKind::ConstantPool(pool) = &action.kind {
                self.constant_pool = pool.clone();
                break;
            }
        }

        // Lift main body
        let main_instrs = self.a1l_lift_actions(actions, total_len, "main")?;
        let main_fn = IIRFunction::new("main", vec![], "void", main_instrs);
        module.add_or_replace(main_fn);

        // Add lifted nested functions
        for f in std::mem::take(&mut self.functions_to_emit) {
            module.add_or_replace(f);
        }

        // Validate emitted module
        let val_errs = module.validate();
        if !val_errs.is_empty() {
            return Err(LifterError::InvalidBytecode(format!(
                "module validation failed: {}",
                val_errs.join("; ")
            )));
        }

        Ok(module)
    }

    fn a1l_fresh_temp(&mut self) -> String {
        let t = format!("%t{}", self.temp_counter);
        self.temp_counter += 1;
        t
    }

    fn a1l_emit(&mut self, out: &mut Vec<IIRInstr>, instr: IIRInstr) -> Result<(), LifterError> {
        self.instruction_count += 1;
        if self.instruction_count > MAX_INSTRUCTIONS {
            return Err(LifterError::InstructionLimitExceeded);
        }
        out.push(instr);
        Ok(())
    }

    fn a1l_lift_actions(
        &mut self,
        actions: &[Avm1Action],
        total_len: usize,
        fn_name: &str,
    ) -> Result<Vec<IIRInstr>, LifterError> {
        let mut instrs = Vec::new();

        // Identify which offsets are jump targets
        let mut jump_targets = HashSet::new();
        for action in actions {
            if let Avm1ActionKind::BranchAlways(off) | Avm1ActionKind::BranchIfTrue(off) =
                &action.kind
            {
                let next_off = action.offset + action.length;
                let tgt = ((next_off as isize) + (*off as isize)) as usize;
                jump_targets.insert(tgt);
            }
        }

        let mut stack_depth: usize = 0;

        for action in actions {
            // If this offset is a jump target, emit its label
            if jump_targets.contains(&action.offset) {
                let lbl = format!("{}_off_{}", fn_name, action.offset);
                self.a1l_emit(
                    &mut instrs,
                    IIRInstr::new("label", None, vec![Operand::Var(lbl)], "void"),
                )?;
            }

            match &action.kind {
                Avm1ActionKind::End => {
                    self.a1l_emit(&mut instrs, IIRInstr::new("ret_void", None, vec![], "void"))?;
                }
                Avm1ActionKind::ConstantPool(pool) => {
                    self.constant_pool = pool.clone();
                }
                Avm1ActionKind::Push(values) => {
                    for val in values {
                        let dest = format!("%s{}", stack_depth);
                        stack_depth += 1;
                        match val {
                            Avm1Value::String(s) => {
                                self.a1l_emit(
                                    &mut instrs,
                                    IIRInstr::new(
                                        "const",
                                        Some(dest),
                                        vec![Operand::Str(s.clone())],
                                        "any",
                                    ),
                                )?;
                            }
                            Avm1Value::Float(f) => {
                                self.a1l_emit(
                                    &mut instrs,
                                    IIRInstr::new(
                                        "const",
                                        Some(dest),
                                        vec![Operand::Float(*f as f64)],
                                        "any",
                                    ),
                                )?;
                            }
                            Avm1Value::Double(d) => {
                                self.a1l_emit(
                                    &mut instrs,
                                    IIRInstr::new(
                                        "const",
                                        Some(dest),
                                        vec![Operand::Float(*d)],
                                        "any",
                                    ),
                                )?;
                            }
                            Avm1Value::Integer(i) => {
                                self.a1l_emit(
                                    &mut instrs,
                                    IIRInstr::new(
                                        "const",
                                        Some(dest),
                                        vec![Operand::Int(*i as i64)],
                                        "any",
                                    ),
                                )?;
                            }
                            Avm1Value::Boolean(b) => {
                                self.a1l_emit(
                                    &mut instrs,
                                    IIRInstr::new(
                                        "const",
                                        Some(dest),
                                        vec![Operand::Bool(*b)],
                                        "any",
                                    ),
                                )?;
                            }
                            Avm1Value::Null => {
                                self.a1l_emit(
                                    &mut instrs,
                                    IIRInstr::new(
                                        "call_builtin",
                                        Some(dest),
                                        vec![Operand::Var("flash_null".to_string())],
                                        "any",
                                    ),
                                )?;
                            }
                            Avm1Value::Undefined => {
                                self.a1l_emit(
                                    &mut instrs,
                                    IIRInstr::new(
                                        "call_builtin",
                                        Some(dest),
                                        vec![Operand::Var("flash_undefined".to_string())],
                                        "any",
                                    ),
                                )?;
                            }
                            Avm1Value::Register(r) => {
                                let reg_var = format!("%r{}", r);
                                self.a1l_emit(
                                    &mut instrs,
                                    IIRInstr::new(
                                        "mov",
                                        Some(dest),
                                        vec![Operand::Var(reg_var)],
                                        "any",
                                    ),
                                )?;
                            }
                            Avm1Value::Constant8(idx) => {
                                let s = self
                                    .constant_pool
                                    .get(*idx as usize)
                                    .cloned()
                                    .unwrap_or_default();
                                self.a1l_emit(
                                    &mut instrs,
                                    IIRInstr::new(
                                        "const",
                                        Some(dest),
                                        vec![Operand::Str(s)],
                                        "any",
                                    ),
                                )?;
                            }
                            Avm1Value::Constant16(idx) => {
                                let s = self
                                    .constant_pool
                                    .get(*idx as usize)
                                    .cloned()
                                    .unwrap_or_default();
                                self.a1l_emit(
                                    &mut instrs,
                                    IIRInstr::new(
                                        "const",
                                        Some(dest),
                                        vec![Operand::Str(s)],
                                        "any",
                                    ),
                                )?;
                            }
                        }
                    }
                }
                Avm1ActionKind::Pop => {
                    stack_depth = stack_depth.saturating_sub(1);
                }
                Avm1ActionKind::StoreRegister(r) => {
                    if stack_depth > 0 {
                        let top = format!("%s{}", stack_depth - 1);
                        let reg_var = format!("%r{}", r);
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new("mov", Some(reg_var), vec![Operand::Var(top)], "any"),
                        )?;
                    }
                }
                Avm1ActionKind::Add | Avm1ActionKind::Add2 => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1; // pops rhs, result in lhs
                        let dest = lhs.clone();
                        self.a1l_emit(
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
                Avm1ActionKind::Subtract => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a1l_emit(
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
                Avm1ActionKind::Multiply => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a1l_emit(
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
                Avm1ActionKind::Divide => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a1l_emit(
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
                Avm1ActionKind::Equals | Avm1ActionKind::Equals2 => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a1l_emit(
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
                Avm1ActionKind::Less | Avm1ActionKind::Less2 => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a1l_emit(
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
                Avm1ActionKind::And => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "and",
                                Some(dest),
                                vec![Operand::Var(lhs), Operand::Var(rhs)],
                                "any",
                            ),
                        )?;
                    }
                }
                Avm1ActionKind::Or => {
                    if stack_depth >= 2 {
                        let rhs = format!("%s{}", stack_depth - 1);
                        let lhs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1;
                        let dest = lhs.clone();
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "or",
                                Some(dest),
                                vec![Operand::Var(lhs), Operand::Var(rhs)],
                                "any",
                            ),
                        )?;
                    }
                }
                Avm1ActionKind::Not => {
                    if stack_depth >= 1 {
                        let top = format!("%s{}", stack_depth - 1);
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new("not", Some(top.clone()), vec![Operand::Var(top)], "any"),
                        )?;
                    }
                }
                Avm1ActionKind::GetVariable => {
                    if stack_depth >= 1 {
                        let name_var = format!("%s{}", stack_depth - 1);
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(name_var.clone()),
                                vec![
                                    Operand::Var("flash_get_variable".to_string()),
                                    Operand::Var(name_var),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                Avm1ActionKind::SetVariable => {
                    if stack_depth >= 2 {
                        let val = format!("%s{}", stack_depth - 1);
                        let name = format!("%s{}", stack_depth - 2);
                        stack_depth -= 2;
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                None,
                                vec![
                                    Operand::Var("flash_set_variable".to_string()),
                                    Operand::Var(name),
                                    Operand::Var(val),
                                ],
                                "void",
                            ),
                        )?;
                    }
                }
                Avm1ActionKind::GetMember => {
                    if stack_depth >= 2 {
                        let prop = format!("%s{}", stack_depth - 1);
                        let obj = format!("%s{}", stack_depth - 2);
                        stack_depth -= 1; // result overwrites obj
                        let dest = obj.clone();
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_get_member".to_string()),
                                    Operand::Var(obj),
                                    Operand::Var(prop),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                Avm1ActionKind::SetMember => {
                    if stack_depth >= 3 {
                        let val = format!("%s{}", stack_depth - 1);
                        let prop = format!("%s{}", stack_depth - 2);
                        let obj = format!("%s{}", stack_depth - 3);
                        stack_depth -= 3;
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                None,
                                vec![
                                    Operand::Var("flash_set_member".to_string()),
                                    Operand::Var(obj),
                                    Operand::Var(prop),
                                    Operand::Var(val),
                                ],
                                "void",
                            ),
                        )?;
                    }
                }
                Avm1ActionKind::CallFunction => {
                    // Pops name + arg_count + args...
                    if stack_depth >= 2 {
                        let fname = format!("%s{}", stack_depth - 1);
                        let nargs_temp = format!("%s{}", stack_depth - 2);
                        stack_depth -= 2; // at least 2 popped
                        let dest = format!("%s{}", stack_depth);
                        stack_depth += 1; // pushes return value
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_call_function".to_string()),
                                    Operand::Var(fname),
                                    Operand::Var(nargs_temp),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                Avm1ActionKind::CallMethod => {
                    if stack_depth >= 3 {
                        let mname = format!("%s{}", stack_depth - 1);
                        let obj = format!("%s{}", stack_depth - 2);
                        let nargs = format!("%s{}", stack_depth - 3);
                        stack_depth -= 3;
                        let dest = format!("%s{}", stack_depth);
                        stack_depth += 1;
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_call_method".to_string()),
                                    Operand::Var(obj),
                                    Operand::Var(mname),
                                    Operand::Var(nargs),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                Avm1ActionKind::NewObject => {
                    if stack_depth >= 2 {
                        let ctor = format!("%s{}", stack_depth - 1);
                        let nargs = format!("%s{}", stack_depth - 2);
                        stack_depth -= 2;
                        let dest = format!("%s{}", stack_depth);
                        stack_depth += 1;
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                Some(dest),
                                vec![
                                    Operand::Var("flash_new_object".to_string()),
                                    Operand::Var(ctor),
                                    Operand::Var(nargs),
                                ],
                                "any",
                            ),
                        )?;
                    }
                }
                Avm1ActionKind::Trace => {
                    if stack_depth >= 1 {
                        let val = format!("%s{}", stack_depth - 1);
                        stack_depth -= 1;
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                None,
                                vec![Operand::Var("flash_trace".to_string()), Operand::Var(val)],
                                "void",
                            ),
                        )?;
                    }
                }
                Avm1ActionKind::Stop => {
                    self.a1l_emit(
                        &mut instrs,
                        IIRInstr::new(
                            "call_builtin",
                            None,
                            vec![Operand::Var("flash_stop".to_string())],
                            "void",
                        ),
                    )?;
                }
                Avm1ActionKind::Play => {
                    self.a1l_emit(
                        &mut instrs,
                        IIRInstr::new(
                            "call_builtin",
                            None,
                            vec![Operand::Var("flash_play".to_string())],
                            "void",
                        ),
                    )?;
                }
                Avm1ActionKind::GotoFrame(frame) => {
                    self.a1l_emit(
                        &mut instrs,
                        IIRInstr::new(
                            "call_builtin",
                            None,
                            vec![
                                Operand::Var("flash_goto_frame".to_string()),
                                Operand::Int(*frame as i64),
                            ],
                            "void",
                        ),
                    )?;
                }
                Avm1ActionKind::BranchAlways(off) => {
                    let next_off = action.offset + action.length;
                    let tgt = ((next_off as isize) + (*off as isize)) as usize;
                    let target_lbl = format!("{}_off_{}", fn_name, tgt);
                    self.a1l_emit(
                        &mut instrs,
                        IIRInstr::new("jmp", None, vec![Operand::Var(target_lbl)], "void"),
                    )?;
                }
                Avm1ActionKind::BranchIfTrue(off) => {
                    if stack_depth >= 1 {
                        let cond = format!("%s{}", stack_depth - 1);
                        stack_depth -= 1;
                        let next_off = action.offset + action.length;
                        let tgt = ((next_off as isize) + (*off as isize)) as usize;
                        let target_lbl = format!("{}_off_{}", fn_name, tgt);
                        self.a1l_emit(
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
                Avm1ActionKind::Return => {
                    if stack_depth >= 1 {
                        let ret_val = format!("%s{}", stack_depth - 1);
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new("ret", None, vec![Operand::Var(ret_val)], "any"),
                        )?;
                    } else {
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new("ret_void", None, vec![], "void"),
                        )?;
                    }
                }
                Avm1ActionKind::DefineFunction {
                    name,
                    params,
                    body,
                    body_len,
                } => {
                    let func_sym = if name.is_empty() {
                        self.a1l_fresh_temp()
                    } else {
                        name.clone()
                    };
                    let nested_instrs = self.a1l_lift_actions(body, *body_len, &func_sym)?;
                    let param_entries: Vec<(String, String)> = params
                        .iter()
                        .map(|p| (p.clone(), "any".to_string()))
                        .collect();
                    let f = IIRFunction::new(&func_sym, param_entries, "any", nested_instrs);
                    self.functions_to_emit.push(f);

                    if name.is_empty() {
                        let dest = format!("%s{}", stack_depth);
                        stack_depth += 1;
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new("const", Some(dest), vec![Operand::Str(func_sym)], "any"),
                        )?;
                    } else {
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                None,
                                vec![
                                    Operand::Var("flash_set_variable".to_string()),
                                    Operand::Str(name.clone()),
                                    Operand::Str(name.clone()),
                                ],
                                "void",
                            ),
                        )?;
                    }
                }
                Avm1ActionKind::DefineFunction2 {
                    name,
                    params,
                    body,
                    body_len,
                    ..
                } => {
                    let func_sym = if name.is_empty() {
                        self.a1l_fresh_temp()
                    } else {
                        name.clone()
                    };
                    let nested_instrs = self.a1l_lift_actions(body, *body_len, &func_sym)?;
                    let param_entries: Vec<(String, String)> = params
                        .iter()
                        .map(|(_, pname)| (pname.clone(), "any".to_string()))
                        .collect();
                    let f = IIRFunction::new(&func_sym, param_entries, "any", nested_instrs);
                    self.functions_to_emit.push(f);

                    if name.is_empty() {
                        let dest = format!("%s{}", stack_depth);
                        stack_depth += 1;
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new("const", Some(dest), vec![Operand::Str(func_sym)], "any"),
                        )?;
                    } else {
                        self.a1l_emit(
                            &mut instrs,
                            IIRInstr::new(
                                "call_builtin",
                                None,
                                vec![
                                    Operand::Var("flash_set_variable".to_string()),
                                    Operand::Str(name.clone()),
                                    Operand::Str(name.clone()),
                                ],
                                "void",
                            ),
                        )?;
                    }
                }
                _ => {}
            }
        }

        // If the end of the action block is a jump target, emit its label
        if jump_targets.contains(&total_len) {
            let end_lbl = format!("{}_off_{}", fn_name, total_len);
            self.a1l_emit(
                &mut instrs,
                IIRInstr::new("label", None, vec![Operand::Var(end_lbl)], "void"),
            )?;
        }

        // Ensure function ends with a return if not already terminated
        let ends_with_ret = instrs
            .last()
            .is_some_and(|i| i.op == "ret" || i.op == "ret_void");
        if !ends_with_ret {
            self.a1l_emit(&mut instrs, IIRInstr::new("ret_void", None, vec![], "void"))?;
        }

        Ok(instrs)
    }
}
