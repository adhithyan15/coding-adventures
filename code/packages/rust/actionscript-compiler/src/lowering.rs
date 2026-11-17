//! ActionScript AST to InterpreterIR lowering.

#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;

use crate::ast::{AssignOp, BinaryOp, ClassDef, Expr, FunctionDef, Program, Stmt, UnaryOp};
use crate::dialect::AsDialect;
use crate::error::{AsCompilerError, MAX_INSTRUCTIONS};

/// Compiler lowering state machine.
pub struct AsLowering {
    dialect: AsDialect,
    module_name: String,
    temp_counter: usize,
    label_counter: usize,
    instruction_count: usize,
    current_instructions: Vec<IIRInstr>,
    functions_to_emit: Vec<IIRFunction>,
    loop_stack: Vec<(String, String)>, // (step/start label, exit label)
    known_functions: HashSet<String>,
    method_to_class: HashMap<String, String>,
}

impl AsLowering {
    /// Create a new lowering context.
    pub fn lowering_new(dialect: AsDialect, module_name: String) -> Self {
        Self {
            dialect,
            module_name,
            temp_counter: 0,
            label_counter: 0,
            instruction_count: 0,
            current_instructions: Vec::new(),
            functions_to_emit: Vec::new(),
            loop_stack: Vec::new(),
            known_functions: HashSet::new(),
            method_to_class: HashMap::new(),
        }
    }

    /// Return the dialect being targeted by this lowering pass.
    pub fn lowering_dialect(&self) -> AsDialect {
        self.dialect
    }

    /// Lower an ActionScript program into an InterpreterIR module.
    pub fn lowering_compile_program(
        &mut self,
        program: &Program,
    ) -> Result<IIRModule, AsCompilerError> {
        let mut module = IIRModule::new(&self.module_name, "actionscript");
        module.entry_point = Some("main".to_string());

        // First pass: collect function and class names
        for stmt in &program.statements {
            self.lowering_collect_declarations(stmt);
        }

        // Lower top-level statements into the "main" function
        self.current_instructions.clear();
        for stmt in &program.statements {
            self.lowering_lower_statement(stmt)?;
        }

        // Ensure "main" terminates with ret_void if not already terminating
        let main_needs_ret = self
            .current_instructions
            .last()
            .is_none_or(|i| i.op != "ret" && i.op != "ret_void");
        if main_needs_ret {
            self.lowering_emit_instr(IIRInstr::new("ret_void", None, vec![], "void"))?;
        }

        let main_fn = IIRFunction::new(
            "main",
            vec![],
            "void",
            std::mem::take(&mut self.current_instructions),
        );
        module.add_or_replace(main_fn);

        // Add all collected function definitions
        for f in std::mem::take(&mut self.functions_to_emit) {
            module.add_or_replace(f);
        }

        // Validate module
        let errors = module.validate();
        if !errors.is_empty() {
            return Err(AsCompilerError::LoweringError(format!(
                "module validation failed: {}",
                errors.join("; ")
            )));
        }

        Ok(module)
    }

    fn lowering_collect_declarations(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FunctionDecl(f) => {
                self.known_functions.insert(f.name.clone());
            }
            Stmt::ClassDecl(cls) => {
                self.known_functions.insert(cls.name.clone());
                for m in &cls.methods {
                    let full_name = format!("{}_{}", cls.name, m.name);
                    self.known_functions.insert(full_name.clone());
                    self.method_to_class.insert(m.name.clone(), full_name);
                }
            }
            Stmt::PackageDecl(pkg) => {
                for s in &pkg.statements {
                    self.lowering_collect_declarations(s);
                }
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.lowering_collect_declarations(s);
                }
            }
            _ => {}
        }
    }

    fn lowering_fresh_temp(&mut self) -> String {
        let t = format!("%t{}", self.temp_counter);
        self.temp_counter += 1;
        t
    }

    fn lowering_fresh_label(&mut self, prefix: &str) -> String {
        let l = format!("{}_{}", prefix, self.label_counter);
        self.label_counter += 1;
        l
    }

    fn lowering_emit_instr(&mut self, instr: IIRInstr) -> Result<(), AsCompilerError> {
        self.instruction_count += 1;
        if self.instruction_count > MAX_INSTRUCTIONS {
            return Err(AsCompilerError::InstructionLimitExceeded);
        }
        self.current_instructions.push(instr);
        Ok(())
    }

    fn lowering_emit_label(&mut self, label: &str) -> Result<(), AsCompilerError> {
        self.lowering_emit_instr(IIRInstr::new(
            "label",
            None,
            vec![Operand::Var(label.to_string())],
            "void",
        ))
    }

    // ------------------------------------------------------------------------
    // Statement Lowering
    // ------------------------------------------------------------------------

    fn lowering_lower_statement(&mut self, stmt: &Stmt) -> Result<(), AsCompilerError> {
        match stmt {
            Stmt::Expr(expr) => {
                let _ = self.lowering_lower_expression(expr)?;
            }
            Stmt::VarDecl { name, init, .. } => {
                if let Some(init_expr) = init {
                    let val = self.lowering_lower_expression(init_expr)?;
                    self.lowering_emit_instr(IIRInstr::new(
                        "mov",
                        Some(name.clone()),
                        vec![Operand::Var(val)],
                        "any",
                    ))?;
                } else {
                    let zero = self.lowering_fresh_temp();
                    self.lowering_emit_instr(IIRInstr::new(
                        "const",
                        Some(zero.clone()),
                        vec![Operand::Int(0)],
                        "any",
                    ))?;
                    self.lowering_emit_instr(IIRInstr::new(
                        "mov",
                        Some(name.clone()),
                        vec![Operand::Var(zero)],
                        "any",
                    ))?;
                }
            }
            Stmt::ConstDecl { name, init, .. } => {
                let val = self.lowering_lower_expression(init)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "mov",
                    Some(name.clone()),
                    vec![Operand::Var(val)],
                    "any",
                ))?;
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.lowering_lower_statement(s)?;
                }
            }
            Stmt::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.lowering_lower_expression(cond)?;
                let else_lbl = self.lowering_fresh_label("else");
                let end_lbl = self.lowering_fresh_label("endif");

                self.lowering_emit_instr(IIRInstr::new(
                    "jmp_if_false",
                    None,
                    vec![
                        Operand::Var(cond_val),
                        Operand::Var(if else_branch.is_some() {
                            else_lbl.clone()
                        } else {
                            end_lbl.clone()
                        }),
                    ],
                    "void",
                ))?;

                self.lowering_lower_statement(then_branch)?;

                if let Some(else_stmt) = else_branch {
                    self.lowering_emit_instr(IIRInstr::new(
                        "jmp",
                        None,
                        vec![Operand::Var(end_lbl.clone())],
                        "void",
                    ))?;
                    self.lowering_emit_label(&else_lbl)?;
                    self.lowering_lower_statement(else_stmt)?;
                }

                self.lowering_emit_label(&end_lbl)?;
            }
            Stmt::While { cond, body } => {
                let start_lbl = self.lowering_fresh_label("while_start");
                let end_lbl = self.lowering_fresh_label("while_end");

                self.loop_stack.push((start_lbl.clone(), end_lbl.clone()));
                self.lowering_emit_label(&start_lbl)?;

                let cond_val = self.lowering_lower_expression(cond)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "jmp_if_false",
                    None,
                    vec![Operand::Var(cond_val), Operand::Var(end_lbl.clone())],
                    "void",
                ))?;

                self.lowering_lower_statement(body)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "jmp",
                    None,
                    vec![Operand::Var(start_lbl)],
                    "void",
                ))?;

                self.lowering_emit_label(&end_lbl)?;
                self.loop_stack.pop();
            }
            Stmt::DoWhile { body, cond } => {
                let start_lbl = self.lowering_fresh_label("do_start");
                let cond_lbl = self.lowering_fresh_label("do_cond");
                let end_lbl = self.lowering_fresh_label("do_end");

                self.loop_stack.push((cond_lbl.clone(), end_lbl.clone()));
                self.lowering_emit_label(&start_lbl)?;

                self.lowering_lower_statement(body)?;

                self.lowering_emit_label(&cond_lbl)?;
                let cond_val = self.lowering_lower_expression(cond)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "jmp_if_true",
                    None,
                    vec![Operand::Var(cond_val), Operand::Var(start_lbl)],
                    "void",
                ))?;

                self.lowering_emit_label(&end_lbl)?;
                self.loop_stack.pop();
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
            } => {
                if let Some(init_stmt) = init {
                    self.lowering_lower_statement(init_stmt)?;
                }

                let start_lbl = self.lowering_fresh_label("for_start");
                let step_lbl = self.lowering_fresh_label("for_step");
                let end_lbl = self.lowering_fresh_label("for_end");

                self.loop_stack.push((step_lbl.clone(), end_lbl.clone()));
                self.lowering_emit_label(&start_lbl)?;

                if let Some(cond_expr) = cond {
                    let cond_val = self.lowering_lower_expression(cond_expr)?;
                    self.lowering_emit_instr(IIRInstr::new(
                        "jmp_if_false",
                        None,
                        vec![Operand::Var(cond_val), Operand::Var(end_lbl.clone())],
                        "void",
                    ))?;
                }

                self.lowering_lower_statement(body)?;

                self.lowering_emit_label(&step_lbl)?;
                if let Some(step_expr) = step {
                    let _ = self.lowering_lower_expression(step_expr)?;
                }
                self.lowering_emit_instr(IIRInstr::new(
                    "jmp",
                    None,
                    vec![Operand::Var(start_lbl)],
                    "void",
                ))?;

                self.lowering_emit_label(&end_lbl)?;
                self.loop_stack.pop();
            }
            Stmt::ForIn {
                var_name,
                obj,
                body,
            } => {
                // Lower for..in by querying keys via builtin flash_keys
                let obj_val = self.lowering_lower_expression(obj)?;
                let keys_arr = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    Some(keys_arr.clone()),
                    vec![
                        Operand::Var("flash_get_keys".to_string()),
                        Operand::Var(obj_val),
                    ],
                    "any",
                ))?;

                let keys_len = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "array_len",
                    Some(keys_len.clone()),
                    vec![Operand::Var(keys_arr.clone())],
                    "i64",
                ))?;

                let idx_var = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "const",
                    Some(idx_var.clone()),
                    vec![Operand::Int(0)],
                    "i64",
                ))?;

                let start_lbl = self.lowering_fresh_label("forin_start");
                let step_lbl = self.lowering_fresh_label("forin_step");
                let end_lbl = self.lowering_fresh_label("forin_end");

                self.loop_stack.push((step_lbl.clone(), end_lbl.clone()));
                self.lowering_emit_label(&start_lbl)?;

                let cmp = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "cmp_lt",
                    Some(cmp.clone()),
                    vec![Operand::Var(idx_var.clone()), Operand::Var(keys_len)],
                    "bool",
                ))?;
                self.lowering_emit_instr(IIRInstr::new(
                    "jmp_if_false",
                    None,
                    vec![Operand::Var(cmp), Operand::Var(end_lbl.clone())],
                    "void",
                ))?;

                let key_elem = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "array_get",
                    Some(key_elem.clone()),
                    vec![Operand::Var(keys_arr), Operand::Var(idx_var.clone())],
                    "any",
                ))?;
                self.lowering_emit_instr(IIRInstr::new(
                    "mov",
                    Some(var_name.clone()),
                    vec![Operand::Var(key_elem)],
                    "any",
                ))?;

                self.lowering_lower_statement(body)?;

                self.lowering_emit_label(&step_lbl)?;
                let one = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "const",
                    Some(one.clone()),
                    vec![Operand::Int(1)],
                    "i64",
                ))?;
                self.lowering_emit_instr(IIRInstr::new(
                    "add",
                    Some(idx_var.clone()),
                    vec![Operand::Var(idx_var), Operand::Var(one)],
                    "i64",
                ))?;
                self.lowering_emit_instr(IIRInstr::new(
                    "jmp",
                    None,
                    vec![Operand::Var(start_lbl)],
                    "void",
                ))?;

                self.lowering_emit_label(&end_lbl)?;
                self.loop_stack.pop();
            }
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    let val = self.lowering_lower_expression(e)?;
                    self.lowering_emit_instr(IIRInstr::new(
                        "ret",
                        None,
                        vec![Operand::Var(val)],
                        "any",
                    ))?;
                } else {
                    self.lowering_emit_instr(IIRInstr::new("ret_void", None, vec![], "void"))?;
                }
            }
            Stmt::Break => {
                if let Some((_, end_lbl)) = self.loop_stack.last() {
                    let end_lbl = end_lbl.clone();
                    self.lowering_emit_instr(IIRInstr::new(
                        "jmp",
                        None,
                        vec![Operand::Var(end_lbl)],
                        "void",
                    ))?;
                } else {
                    return Err(AsCompilerError::LoweringError(
                        "break outside of loop".into(),
                    ));
                }
            }
            Stmt::Continue => {
                if let Some((step_lbl, _)) = self.loop_stack.last() {
                    let step_lbl = step_lbl.clone();
                    self.lowering_emit_instr(IIRInstr::new(
                        "jmp",
                        None,
                        vec![Operand::Var(step_lbl)],
                        "void",
                    ))?;
                } else {
                    return Err(AsCompilerError::LoweringError(
                        "continue outside of loop".into(),
                    ));
                }
            }
            Stmt::FunctionDecl(f) => {
                self.lowering_lower_function(f)?;
            }
            Stmt::ClassDecl(cls) => {
                self.lowering_lower_class(cls)?;
            }
            Stmt::InterfaceDecl(_) => {
                // Interfaces are compile-time type verification contracts; no IR emitted
            }
            Stmt::PackageDecl(pkg) => {
                for s in &pkg.statements {
                    self.lowering_lower_statement(s)?;
                }
            }
            Stmt::ImportDecl(_) | Stmt::NamespaceDecl(_) => {
                // Static metadata; no IR emitted
            }
            Stmt::TryCatchFinally {
                try_block,
                catches,
                finally_block,
            } => {
                for s in try_block {
                    self.lowering_lower_statement(s)?;
                }
                for c in catches {
                    for s in &c.body {
                        self.lowering_lower_statement(s)?;
                    }
                }
                if let Some(fin) = finally_block {
                    for s in fin {
                        self.lowering_lower_statement(s)?;
                    }
                }
            }
            Stmt::Throw(expr) => {
                let val = self.lowering_lower_expression(expr)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![Operand::Var("flash_throw".to_string()), Operand::Var(val)],
                    "void",
                ))?;
            }
            // Flash Timeline Verbs
            Stmt::Stop => {
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![Operand::Var("flash_stop".to_string())],
                    "void",
                ))?;
            }
            Stmt::Play => {
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![Operand::Var("flash_play".to_string())],
                    "void",
                ))?;
            }
            Stmt::GotoAndPlay(expr) => {
                let frame = self.lowering_lower_expression(expr)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![
                        Operand::Var("flash_goto_and_play".to_string()),
                        Operand::Var(frame),
                    ],
                    "void",
                ))?;
            }
            Stmt::GotoAndStop(expr) => {
                let frame = self.lowering_lower_expression(expr)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![
                        Operand::Var("flash_goto_and_stop".to_string()),
                        Operand::Var(frame),
                    ],
                    "void",
                ))?;
            }
            Stmt::TellTarget { target, body } => {
                let target_val = self.lowering_lower_expression(target)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![
                        Operand::Var("flash_tell_target".to_string()),
                        Operand::Var(target_val),
                    ],
                    "void",
                ))?;
                for s in body {
                    self.lowering_lower_statement(s)?;
                }
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![Operand::Var("flash_end_tell_target".to_string())],
                    "void",
                ))?;
            }
            Stmt::CallAction(expr) => {
                let arg = self.lowering_lower_expression(expr)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![Operand::Var("flash_call".to_string()), Operand::Var(arg)],
                    "void",
                ))?;
            }
            Stmt::OnClipEvent { event, body } => {
                let event_fn_name = format!("__onClipEvent_{}", event);
                let saved_instructions = std::mem::take(&mut self.current_instructions);
                for s in body {
                    self.lowering_lower_statement(s)?;
                }
                let needs_ret = self
                    .current_instructions
                    .last()
                    .is_none_or(|i| i.op != "ret" && i.op != "ret_void");
                if needs_ret {
                    self.lowering_emit_instr(IIRInstr::new("ret_void", None, vec![], "void"))?;
                }
                let event_fn = IIRFunction::new(
                    &event_fn_name,
                    vec![],
                    "void",
                    std::mem::take(&mut self.current_instructions),
                );
                self.functions_to_emit.push(event_fn);
                self.current_instructions = saved_instructions;

                let ev_str = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "str_const",
                    Some(ev_str.clone()),
                    vec![Operand::Str(event.clone())],
                    "str",
                ))?;
                let fn_str = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "str_const",
                    Some(fn_str.clone()),
                    vec![Operand::Str(event_fn_name)],
                    "str",
                ))?;
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![
                        Operand::Var("flash_register_clip_event".to_string()),
                        Operand::Var(ev_str),
                        Operand::Var(fn_str),
                    ],
                    "void",
                ))?;
            }
            Stmt::Trace(expr) => {
                let val = self.lowering_lower_expression(expr)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    None,
                    vec![Operand::Var("flash_trace".to_string()), Operand::Var(val)],
                    "void",
                ))?;
            }
        }
        Ok(())
    }

    fn lowering_lower_function(&mut self, f: &FunctionDef) -> Result<(), AsCompilerError> {
        let saved_instructions = std::mem::take(&mut self.current_instructions);
        let params: Vec<(String, String)> = f
            .params
            .iter()
            .map(|p| (p.name.clone(), "any".to_string()))
            .collect();

        for s in &f.body {
            self.lowering_lower_statement(s)?;
        }

        let needs_ret = self
            .current_instructions
            .last()
            .is_none_or(|i| i.op != "ret" && i.op != "ret_void");
        if needs_ret {
            self.lowering_emit_instr(IIRInstr::new("ret_void", None, vec![], "void"))?;
        }

        let fn_ir = IIRFunction::new(
            &f.name,
            params,
            "any",
            std::mem::take(&mut self.current_instructions),
        );
        self.functions_to_emit.push(fn_ir);
        self.current_instructions = saved_instructions;
        Ok(())
    }

    fn lowering_lower_class(&mut self, cls: &ClassDef) -> Result<(), AsCompilerError> {
        // AS2/AS3 class lowering:
        // 1. Emit constructor function: `cls.name(this, params...)`
        // 2. Emit methods: `cls.name_method(this, params...)`
        let ctor_opt = cls.methods.iter().find(|m| m.name == cls.name);
        let ctor_params = if let Some(ctor) = ctor_opt {
            ctor.params.clone()
        } else {
            vec![]
        };

        let saved_instructions = std::mem::take(&mut self.current_instructions);

        // Constructor function
        let mut ir_params = vec![("this".to_string(), "any".to_string())];
        for p in &ctor_params {
            ir_params.push((p.name.clone(), "any".to_string()));
        }

        // Initialize fields inside constructor
        for f in &cls.fields {
            if !f.is_static {
                if let Some(init) = &f.init {
                    let val = self.lowering_lower_expression(init)?;
                    let prop_name = self.lowering_fresh_temp();
                    self.lowering_emit_instr(IIRInstr::new(
                        "str_const",
                        Some(prop_name.clone()),
                        vec![Operand::Str(f.name.clone())],
                        "str",
                    ))?;
                    self.lowering_emit_instr(IIRInstr::new(
                        "call_builtin",
                        None,
                        vec![
                            Operand::Var("flash_set_property".to_string()),
                            Operand::Var("this".to_string()),
                            Operand::Var(prop_name),
                            Operand::Var(val),
                        ],
                        "void",
                    ))?;
                }
            }
        }

        if let Some(ctor) = ctor_opt {
            for s in &ctor.body {
                self.lowering_lower_statement(s)?;
            }
        }

        // Return `this` from constructor
        self.lowering_emit_instr(IIRInstr::new(
            "ret",
            None,
            vec![Operand::Var("this".to_string())],
            "any",
        ))?;

        let ctor_fn = IIRFunction::new(
            &cls.name,
            ir_params,
            "any",
            std::mem::take(&mut self.current_instructions),
        );
        self.functions_to_emit.push(ctor_fn);

        // Lower non-constructor methods
        for m in &cls.methods {
            if m.name == cls.name {
                continue;
            }
            let method_name = format!("{}_{}", cls.name, m.name);
            let mut m_params = Vec::new();
            if !m.is_static {
                m_params.push(("this".to_string(), "any".to_string()));
            }
            for p in &m.params {
                m_params.push((p.name.clone(), "any".to_string()));
            }

            for s in &m.body {
                self.lowering_lower_statement(s)?;
            }

            let needs_ret = self
                .current_instructions
                .last()
                .is_none_or(|i| i.op != "ret" && i.op != "ret_void");
            if needs_ret {
                self.lowering_emit_instr(IIRInstr::new("ret_void", None, vec![], "void"))?;
            }

            let method_fn = IIRFunction::new(
                &method_name,
                m_params,
                "any",
                std::mem::take(&mut self.current_instructions),
            );
            self.functions_to_emit.push(method_fn);
        }

        self.current_instructions = saved_instructions;
        Ok(())
    }

    // ------------------------------------------------------------------------
    // Expression Lowering
    // ------------------------------------------------------------------------

    fn lowering_lower_expression(&mut self, expr: &Expr) -> Result<String, AsCompilerError> {
        match expr {
            Expr::Number(val) => {
                let dest = self.lowering_fresh_temp();
                if val.fract() == 0.0 && *val >= (i64::MIN as f64) && *val <= (i64::MAX as f64) {
                    self.lowering_emit_instr(IIRInstr::new(
                        "const",
                        Some(dest.clone()),
                        vec![Operand::Int(*val as i64)],
                        "i64",
                    ))?;
                } else {
                    self.lowering_emit_instr(IIRInstr::new(
                        "const",
                        Some(dest.clone()),
                        vec![Operand::Float(*val)],
                        "f64",
                    ))?;
                }
                Ok(dest)
            }
            Expr::StringLit(s) => {
                let dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "str_const",
                    Some(dest.clone()),
                    vec![Operand::Str(s.clone())],
                    "str",
                ))?;
                Ok(dest)
            }
            Expr::BoolLit(b) => {
                let dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "const",
                    Some(dest.clone()),
                    vec![Operand::Bool(*b)],
                    "bool",
                ))?;
                Ok(dest)
            }
            Expr::Null | Expr::Undefined => {
                let dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    Some(dest.clone()),
                    vec![Operand::Var("dyn_nil".to_string())],
                    "any",
                ))?;
                Ok(dest)
            }
            Expr::This => {
                let dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    Some(dest.clone()),
                    vec![Operand::Var("flash_get_this".to_string())],
                    "any",
                ))?;
                Ok(dest)
            }
            Expr::Root => {
                let dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    Some(dest.clone()),
                    vec![Operand::Var("flash_get_root".to_string())],
                    "any",
                ))?;
                Ok(dest)
            }
            Expr::Parent => {
                let dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    Some(dest.clone()),
                    vec![Operand::Var("flash_get_parent".to_string())],
                    "any",
                ))?;
                Ok(dest)
            }
            Expr::Global => {
                let dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    Some(dest.clone()),
                    vec![Operand::Var("flash_get_global".to_string())],
                    "any",
                ))?;
                Ok(dest)
            }
            Expr::Ident(name) => {
                let dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "mov",
                    Some(dest.clone()),
                    vec![Operand::Var(name.clone())],
                    "any",
                ))?;
                Ok(dest)
            }
            Expr::ArrayLit(items) => {
                let count = items.len();
                let count_val = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "const",
                    Some(count_val.clone()),
                    vec![Operand::Int(count as i64)],
                    "i64",
                ))?;
                let arr_dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "alloc_array",
                    Some(arr_dest.clone()),
                    vec![Operand::Var(count_val)],
                    "array<any>",
                ))?;

                for (idx, item) in items.iter().enumerate() {
                    let item_val = self.lowering_lower_expression(item)?;
                    let idx_val = self.lowering_fresh_temp();
                    self.lowering_emit_instr(IIRInstr::new(
                        "const",
                        Some(idx_val.clone()),
                        vec![Operand::Int(idx as i64)],
                        "i64",
                    ))?;
                    self.lowering_emit_instr(IIRInstr::new(
                        "array_set",
                        None,
                        vec![
                            Operand::Var(arr_dest.clone()),
                            Operand::Var(idx_val),
                            Operand::Var(item_val),
                        ],
                        "void",
                    ))?;
                }
                Ok(arr_dest)
            }
            Expr::ObjectLit(props) => {
                let obj_dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    Some(obj_dest.clone()),
                    vec![Operand::Var("flash_new_object".to_string())],
                    "any",
                ))?;

                for (key, val_expr) in props {
                    let val = self.lowering_lower_expression(val_expr)?;
                    let key_val = self.lowering_fresh_temp();
                    self.lowering_emit_instr(IIRInstr::new(
                        "str_const",
                        Some(key_val.clone()),
                        vec![Operand::Str(key.clone())],
                        "str",
                    ))?;
                    self.lowering_emit_instr(IIRInstr::new(
                        "call_builtin",
                        None,
                        vec![
                            Operand::Var("flash_set_property".to_string()),
                            Operand::Var(obj_dest.clone()),
                            Operand::Var(key_val),
                            Operand::Var(val),
                        ],
                        "void",
                    ))?;
                }
                Ok(obj_dest)
            }
            Expr::Binary { op, left, right } => self.lowering_lower_binary_op(*op, left, right),
            Expr::Unary { op, expr } => self.lowering_lower_unary_op(*op, expr),
            Expr::Assign { op, target, value } => {
                let rhs_val = self.lowering_lower_expression(value)?;
                let final_val = match op {
                    AssignOp::Assign => rhs_val,
                    AssignOp::AddAssign => {
                        let cur = self.lowering_lower_expression(target)?;
                        let dest = self.lowering_fresh_temp();
                        self.lowering_emit_instr(IIRInstr::new(
                            "add",
                            Some(dest.clone()),
                            vec![Operand::Var(cur), Operand::Var(rhs_val)],
                            "any",
                        ))?;
                        dest
                    }
                    AssignOp::SubAssign => {
                        let cur = self.lowering_lower_expression(target)?;
                        let dest = self.lowering_fresh_temp();
                        self.lowering_emit_instr(IIRInstr::new(
                            "sub",
                            Some(dest.clone()),
                            vec![Operand::Var(cur), Operand::Var(rhs_val)],
                            "any",
                        ))?;
                        dest
                    }
                    AssignOp::MulAssign => {
                        let cur = self.lowering_lower_expression(target)?;
                        let dest = self.lowering_fresh_temp();
                        self.lowering_emit_instr(IIRInstr::new(
                            "mul",
                            Some(dest.clone()),
                            vec![Operand::Var(cur), Operand::Var(rhs_val)],
                            "any",
                        ))?;
                        dest
                    }
                    AssignOp::DivAssign => {
                        let cur = self.lowering_lower_expression(target)?;
                        let dest = self.lowering_fresh_temp();
                        self.lowering_emit_instr(IIRInstr::new(
                            "div",
                            Some(dest.clone()),
                            vec![Operand::Var(cur), Operand::Var(rhs_val)],
                            "any",
                        ))?;
                        dest
                    }
                    AssignOp::ModAssign => {
                        let cur = self.lowering_lower_expression(target)?;
                        let dest = self.lowering_fresh_temp();
                        self.lowering_emit_instr(IIRInstr::new(
                            "mod",
                            Some(dest.clone()),
                            vec![Operand::Var(cur), Operand::Var(rhs_val)],
                            "any",
                        ))?;
                        dest
                    }
                };

                match &**target {
                    Expr::Ident(name) => {
                        self.lowering_emit_instr(IIRInstr::new(
                            "mov",
                            Some(name.clone()),
                            vec![Operand::Var(final_val.clone())],
                            "any",
                        ))?;
                    }
                    Expr::Member { object, property } => {
                        let obj_val = self.lowering_lower_expression(object)?;
                        let prop_str = self.lowering_fresh_temp();
                        self.lowering_emit_instr(IIRInstr::new(
                            "str_const",
                            Some(prop_str.clone()),
                            vec![Operand::Str(property.clone())],
                            "str",
                        ))?;
                        self.lowering_emit_instr(IIRInstr::new(
                            "call_builtin",
                            None,
                            vec![
                                Operand::Var("flash_set_property".to_string()),
                                Operand::Var(obj_val),
                                Operand::Var(prop_str),
                                Operand::Var(final_val.clone()),
                            ],
                            "void",
                        ))?;
                    }
                    Expr::Index { object, index } => {
                        let obj_val = self.lowering_lower_expression(object)?;
                        let idx_val = self.lowering_lower_expression(index)?;
                        self.lowering_emit_instr(IIRInstr::new(
                            "array_set",
                            None,
                            vec![
                                Operand::Var(obj_val),
                                Operand::Var(idx_val),
                                Operand::Var(final_val.clone()),
                            ],
                            "void",
                        ))?;
                    }
                    _ => {
                        return Err(AsCompilerError::LoweringError(
                            "invalid assignment target".into(),
                        ));
                    }
                }
                Ok(final_val)
            }
            Expr::Call { callee, args } => {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.lowering_lower_expression(a)?);
                }

                match &**callee {
                    Expr::Ident(name) => {
                        let dest = self.lowering_fresh_temp();
                        if self.known_functions.contains(name) {
                            let mut srcs = vec![Operand::Var(name.clone())];
                            for a in arg_vals {
                                srcs.push(Operand::Var(a));
                            }
                            self.lowering_emit_instr(IIRInstr::new(
                                "call",
                                Some(dest.clone()),
                                srcs,
                                "any",
                            ))?;
                        } else {
                            // Call builtin / global function
                            let mut srcs = vec![Operand::Var(name.clone())];
                            for a in arg_vals {
                                srcs.push(Operand::Var(a));
                            }
                            self.lowering_emit_instr(IIRInstr::new(
                                "call_builtin",
                                Some(dest.clone()),
                                srcs,
                                "any",
                            ))?;
                        }
                        Ok(dest)
                    }
                    Expr::Member { object, property } => {
                        let obj_val = self.lowering_lower_expression(object)?;
                        let dest = self.lowering_fresh_temp();

                        if let Some(target_fn) = self.method_to_class.get(property) {
                            let mut srcs =
                                vec![Operand::Var(target_fn.clone()), Operand::Var(obj_val)];
                            for a in arg_vals {
                                srcs.push(Operand::Var(a));
                            }
                            self.lowering_emit_instr(IIRInstr::new(
                                "call",
                                Some(dest.clone()),
                                srcs,
                                "any",
                            ))?;
                        } else {
                            let prop_str = self.lowering_fresh_temp();
                            self.lowering_emit_instr(IIRInstr::new(
                                "str_const",
                                Some(prop_str.clone()),
                                vec![Operand::Str(property.clone())],
                                "str",
                            ))?;

                            let mut srcs = vec![
                                Operand::Var("flash_invoke_method".to_string()),
                                Operand::Var(obj_val),
                                Operand::Var(prop_str),
                            ];
                            for a in arg_vals {
                                srcs.push(Operand::Var(a));
                            }
                            self.lowering_emit_instr(IIRInstr::new(
                                "call_builtin",
                                Some(dest.clone()),
                                srcs,
                                "any",
                            ))?;
                        }
                        Ok(dest)
                    }
                    _ => {
                        let callee_val = self.lowering_lower_expression(callee)?;
                        let dest = self.lowering_fresh_temp();
                        let mut srcs = vec![
                            Operand::Var("flash_call_function".to_string()),
                            Operand::Var(callee_val),
                        ];
                        for a in arg_vals {
                            srcs.push(Operand::Var(a));
                        }
                        self.lowering_emit_instr(IIRInstr::new(
                            "call_builtin",
                            Some(dest.clone()),
                            srcs,
                            "any",
                        ))?;
                        Ok(dest)
                    }
                }
            }
            Expr::Member { object, property } => {
                let obj_val = self.lowering_lower_expression(object)?;
                let prop_str = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "str_const",
                    Some(prop_str.clone()),
                    vec![Operand::Str(property.clone())],
                    "str",
                ))?;
                let dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    Some(dest.clone()),
                    vec![
                        Operand::Var("flash_get_property".to_string()),
                        Operand::Var(obj_val),
                        Operand::Var(prop_str),
                    ],
                    "any",
                ))?;
                Ok(dest)
            }
            Expr::Index { object, index } => {
                let obj_val = self.lowering_lower_expression(object)?;
                let idx_val = self.lowering_lower_expression(index)?;
                let dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "array_get",
                    Some(dest.clone()),
                    vec![Operand::Var(obj_val), Operand::Var(idx_val)],
                    "any",
                ))?;
                Ok(dest)
            }
            Expr::New { callee, args } => {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.lowering_lower_expression(a)?);
                }

                if let Expr::Ident(class_name) = &**callee {
                    let instance = self.lowering_fresh_temp();
                    let cls_str = self.lowering_fresh_temp();
                    self.lowering_emit_instr(IIRInstr::new(
                        "str_const",
                        Some(cls_str.clone()),
                        vec![Operand::Str(class_name.clone())],
                        "str",
                    ))?;
                    self.lowering_emit_instr(IIRInstr::new(
                        "call_builtin",
                        Some(instance.clone()),
                        vec![
                            Operand::Var("flash_new_instance".to_string()),
                            Operand::Var(cls_str),
                        ],
                        "any",
                    ))?;

                    if self.known_functions.contains(class_name) {
                        let mut srcs = vec![
                            Operand::Var(class_name.clone()),
                            Operand::Var(instance.clone()),
                        ];
                        for a in arg_vals {
                            srcs.push(Operand::Var(a));
                        }
                        let _ = self.lowering_fresh_temp();
                        self.lowering_emit_instr(IIRInstr::new("call", None, srcs, "any"))?;
                    }
                    Ok(instance)
                } else {
                    let callee_val = self.lowering_lower_expression(callee)?;
                    let dest = self.lowering_fresh_temp();
                    let mut srcs = vec![
                        Operand::Var("flash_construct".to_string()),
                        Operand::Var(callee_val),
                    ];
                    for a in arg_vals {
                        srcs.push(Operand::Var(a));
                    }
                    self.lowering_emit_instr(IIRInstr::new(
                        "call_builtin",
                        Some(dest.clone()),
                        srcs,
                        "any",
                    ))?;
                    Ok(dest)
                }
            }
            Expr::Conditional {
                cond,
                then_expr,
                else_expr,
            } => {
                let cond_val = self.lowering_lower_expression(cond)?;
                let else_lbl = self.lowering_fresh_label("cond_else");
                let end_lbl = self.lowering_fresh_label("cond_end");
                let dest = self.lowering_fresh_temp();

                self.lowering_emit_instr(IIRInstr::new(
                    "jmp_if_false",
                    None,
                    vec![Operand::Var(cond_val), Operand::Var(else_lbl.clone())],
                    "void",
                ))?;

                let then_val = self.lowering_lower_expression(then_expr)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "mov",
                    Some(dest.clone()),
                    vec![Operand::Var(then_val)],
                    "any",
                ))?;
                self.lowering_emit_instr(IIRInstr::new(
                    "jmp",
                    None,
                    vec![Operand::Var(end_lbl.clone())],
                    "void",
                ))?;

                self.lowering_emit_label(&else_lbl)?;
                let else_val = self.lowering_lower_expression(else_expr)?;
                self.lowering_emit_instr(IIRInstr::new(
                    "mov",
                    Some(dest.clone()),
                    vec![Operand::Var(else_val)],
                    "any",
                ))?;

                self.lowering_emit_label(&end_lbl)?;
                Ok(dest)
            }
            Expr::VectorLit { item_type, items } => {
                let type_name = format!("{:?}", item_type);
                let type_str = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "str_const",
                    Some(type_str.clone()),
                    vec![Operand::Str(type_name)],
                    "str",
                ))?;

                let count = items.len();
                let count_val = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "const",
                    Some(count_val.clone()),
                    vec![Operand::Int(count as i64)],
                    "i64",
                ))?;

                let vec_dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "alloc_array",
                    Some(vec_dest.clone()),
                    vec![Operand::Var(count_val)],
                    "array<any>",
                ))?;

                for (idx, item) in items.iter().enumerate() {
                    let item_val = self.lowering_lower_expression(item)?;
                    let idx_val = self.lowering_fresh_temp();
                    self.lowering_emit_instr(IIRInstr::new(
                        "const",
                        Some(idx_val.clone()),
                        vec![Operand::Int(idx as i64)],
                        "i64",
                    ))?;
                    self.lowering_emit_instr(IIRInstr::new(
                        "array_set",
                        None,
                        vec![
                            Operand::Var(vec_dest.clone()),
                            Operand::Var(idx_val),
                            Operand::Var(item_val),
                        ],
                        "void",
                    ))?;
                }
                Ok(vec_dest)
            }
            Expr::Eval(arg) => {
                let arg_val = self.lowering_lower_expression(arg)?;
                let dest = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    Some(dest.clone()),
                    vec![
                        Operand::Var("flash_eval".to_string()),
                        Operand::Var(arg_val),
                    ],
                    "any",
                ))?;
                Ok(dest)
            }
        }
    }

    fn lowering_lower_binary_op(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<String, AsCompilerError> {
        // Short-circuiting logical operations
        if op == BinaryOp::LogAnd {
            let left_val = self.lowering_lower_expression(left)?;
            let dest = self.lowering_fresh_temp();
            let false_lbl = self.lowering_fresh_label("land_false");
            let end_lbl = self.lowering_fresh_label("land_end");

            self.lowering_emit_instr(IIRInstr::new(
                "jmp_if_false",
                None,
                vec![Operand::Var(left_val), Operand::Var(false_lbl.clone())],
                "void",
            ))?;

            let right_val = self.lowering_lower_expression(right)?;
            self.lowering_emit_instr(IIRInstr::new(
                "mov",
                Some(dest.clone()),
                vec![Operand::Var(right_val)],
                "any",
            ))?;
            self.lowering_emit_instr(IIRInstr::new(
                "jmp",
                None,
                vec![Operand::Var(end_lbl.clone())],
                "void",
            ))?;

            self.lowering_emit_label(&false_lbl)?;
            let false_val = self.lowering_fresh_temp();
            self.lowering_emit_instr(IIRInstr::new(
                "const",
                Some(false_val.clone()),
                vec![Operand::Bool(false)],
                "bool",
            ))?;
            self.lowering_emit_instr(IIRInstr::new(
                "mov",
                Some(dest.clone()),
                vec![Operand::Var(false_val)],
                "any",
            ))?;

            self.lowering_emit_label(&end_lbl)?;
            return Ok(dest);
        }

        if op == BinaryOp::LogOr {
            let left_val = self.lowering_lower_expression(left)?;
            let dest = self.lowering_fresh_temp();
            let true_lbl = self.lowering_fresh_label("lor_true");
            let end_lbl = self.lowering_fresh_label("lor_end");

            self.lowering_emit_instr(IIRInstr::new(
                "jmp_if_true",
                None,
                vec![Operand::Var(left_val), Operand::Var(true_lbl.clone())],
                "void",
            ))?;

            let right_val = self.lowering_lower_expression(right)?;
            self.lowering_emit_instr(IIRInstr::new(
                "mov",
                Some(dest.clone()),
                vec![Operand::Var(right_val)],
                "any",
            ))?;
            self.lowering_emit_instr(IIRInstr::new(
                "jmp",
                None,
                vec![Operand::Var(end_lbl.clone())],
                "void",
            ))?;

            self.lowering_emit_label(&true_lbl)?;
            let true_val = self.lowering_fresh_temp();
            self.lowering_emit_instr(IIRInstr::new(
                "const",
                Some(true_val.clone()),
                vec![Operand::Bool(true)],
                "bool",
            ))?;
            self.lowering_emit_instr(IIRInstr::new(
                "mov",
                Some(dest.clone()),
                vec![Operand::Var(true_val)],
                "any",
            ))?;

            self.lowering_emit_label(&end_lbl)?;
            return Ok(dest);
        }

        // AS3 is / as operators
        if op == BinaryOp::Is {
            let left_val = self.lowering_lower_expression(left)?;
            let right_name = match right {
                Expr::Ident(name) => name.clone(),
                other => format!("{:?}", other),
            };
            let type_str = self.lowering_fresh_temp();
            self.lowering_emit_instr(IIRInstr::new(
                "str_const",
                Some(type_str.clone()),
                vec![Operand::Str(right_name)],
                "str",
            ))?;
            let dest = self.lowering_fresh_temp();
            self.lowering_emit_instr(IIRInstr::new(
                "call_builtin",
                Some(dest.clone()),
                vec![
                    Operand::Var("flash_is".to_string()),
                    Operand::Var(left_val),
                    Operand::Var(type_str),
                ],
                "bool",
            ))?;
            return Ok(dest);
        }

        if op == BinaryOp::As {
            let left_val = self.lowering_lower_expression(left)?;
            let right_name = match right {
                Expr::Ident(name) => name.clone(),
                other => format!("{:?}", other),
            };
            let type_str = self.lowering_fresh_temp();
            self.lowering_emit_instr(IIRInstr::new(
                "str_const",
                Some(type_str.clone()),
                vec![Operand::Str(right_name)],
                "str",
            ))?;
            let dest = self.lowering_fresh_temp();
            self.lowering_emit_instr(IIRInstr::new(
                "call_builtin",
                Some(dest.clone()),
                vec![
                    Operand::Var("flash_as".to_string()),
                    Operand::Var(left_val),
                    Operand::Var(type_str),
                ],
                "any",
            ))?;
            return Ok(dest);
        }

        let left_val = self.lowering_lower_expression(left)?;
        let right_val = self.lowering_lower_expression(right)?;
        let dest = self.lowering_fresh_temp();

        let (mnemonic, hint) = match op {
            BinaryOp::Add => ("add", "any"),
            BinaryOp::Sub => ("sub", "any"),
            BinaryOp::Mul => ("mul", "any"),
            BinaryOp::Div => ("div", "any"),
            BinaryOp::Mod => ("mod", "any"),
            BinaryOp::BitAnd => ("and", "i64"),
            BinaryOp::BitOr => ("or", "i64"),
            BinaryOp::BitXor => ("xor", "i64"),
            BinaryOp::Shl => ("shl", "i64"),
            BinaryOp::Shr => ("shr", "i64"),
            BinaryOp::Ushr => ("shr", "i64"),
            BinaryOp::Eq => ("cmp_eq", "bool"),
            BinaryOp::NotEq => ("cmp_ne", "bool"),
            BinaryOp::StrictEq => ("cmp_eq", "bool"),
            BinaryOp::StrictNotEq => ("cmp_ne", "bool"),
            BinaryOp::Lt => ("cmp_lt", "bool"),
            BinaryOp::LtEq => ("cmp_le", "bool"),
            BinaryOp::Gt => ("cmp_gt", "bool"),
            BinaryOp::GtEq => ("cmp_ge", "bool"),
            BinaryOp::LogAnd | BinaryOp::LogOr | BinaryOp::Is | BinaryOp::As => unreachable!(),
        };

        self.lowering_emit_instr(IIRInstr::new(
            mnemonic,
            Some(dest.clone()),
            vec![Operand::Var(left_val), Operand::Var(right_val)],
            hint,
        ))?;

        Ok(dest)
    }

    fn lowering_lower_unary_op(
        &mut self,
        op: UnaryOp,
        expr: &Expr,
    ) -> Result<String, AsCompilerError> {
        let val = self.lowering_lower_expression(expr)?;
        let dest = self.lowering_fresh_temp();

        match op {
            UnaryOp::Neg => {
                self.lowering_emit_instr(IIRInstr::new(
                    "neg",
                    Some(dest.clone()),
                    vec![Operand::Var(val)],
                    "any",
                ))?;
            }
            UnaryOp::Pos => {
                self.lowering_emit_instr(IIRInstr::new(
                    "mov",
                    Some(dest.clone()),
                    vec![Operand::Var(val)],
                    "any",
                ))?;
            }
            UnaryOp::Not => {
                let false_val = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "const",
                    Some(false_val.clone()),
                    vec![Operand::Bool(false)],
                    "bool",
                ))?;
                self.lowering_emit_instr(IIRInstr::new(
                    "cmp_eq",
                    Some(dest.clone()),
                    vec![Operand::Var(val), Operand::Var(false_val)],
                    "bool",
                ))?;
            }
            UnaryOp::BitNot => {
                self.lowering_emit_instr(IIRInstr::new(
                    "not",
                    Some(dest.clone()),
                    vec![Operand::Var(val)],
                    "i64",
                ))?;
            }
            UnaryOp::PreInc => {
                let one = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "const",
                    Some(one.clone()),
                    vec![Operand::Int(1)],
                    "i64",
                ))?;
                self.lowering_emit_instr(IIRInstr::new(
                    "add",
                    Some(dest.clone()),
                    vec![Operand::Var(val.clone()), Operand::Var(one)],
                    "any",
                ))?;
                if let Expr::Ident(name) = expr {
                    self.lowering_emit_instr(IIRInstr::new(
                        "mov",
                        Some(name.clone()),
                        vec![Operand::Var(dest.clone())],
                        "any",
                    ))?;
                }
            }
            UnaryOp::PreDec => {
                let one = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "const",
                    Some(one.clone()),
                    vec![Operand::Int(1)],
                    "i64",
                ))?;
                self.lowering_emit_instr(IIRInstr::new(
                    "sub",
                    Some(dest.clone()),
                    vec![Operand::Var(val.clone()), Operand::Var(one)],
                    "any",
                ))?;
                if let Expr::Ident(name) = expr {
                    self.lowering_emit_instr(IIRInstr::new(
                        "mov",
                        Some(name.clone()),
                        vec![Operand::Var(dest.clone())],
                        "any",
                    ))?;
                }
            }
            UnaryOp::PostInc => {
                self.lowering_emit_instr(IIRInstr::new(
                    "mov",
                    Some(dest.clone()),
                    vec![Operand::Var(val.clone())],
                    "any",
                ))?;
                let one = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "const",
                    Some(one.clone()),
                    vec![Operand::Int(1)],
                    "i64",
                ))?;
                let bumped = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "add",
                    Some(bumped.clone()),
                    vec![Operand::Var(val), Operand::Var(one)],
                    "any",
                ))?;
                if let Expr::Ident(name) = expr {
                    self.lowering_emit_instr(IIRInstr::new(
                        "mov",
                        Some(name.clone()),
                        vec![Operand::Var(bumped)],
                        "any",
                    ))?;
                }
            }
            UnaryOp::PostDec => {
                self.lowering_emit_instr(IIRInstr::new(
                    "mov",
                    Some(dest.clone()),
                    vec![Operand::Var(val.clone())],
                    "any",
                ))?;
                let one = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "const",
                    Some(one.clone()),
                    vec![Operand::Int(1)],
                    "i64",
                ))?;
                let bumped = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "sub",
                    Some(bumped.clone()),
                    vec![Operand::Var(val), Operand::Var(one)],
                    "any",
                ))?;
                if let Expr::Ident(name) = expr {
                    self.lowering_emit_instr(IIRInstr::new(
                        "mov",
                        Some(name.clone()),
                        vec![Operand::Var(bumped)],
                        "any",
                    ))?;
                }
            }
            UnaryOp::Typeof => {
                self.lowering_emit_instr(IIRInstr::new(
                    "call_builtin",
                    Some(dest.clone()),
                    vec![Operand::Var("flash_typeof".to_string()), Operand::Var(val)],
                    "str",
                ))?;
            }
            UnaryOp::Delete => {
                let true_val = self.lowering_fresh_temp();
                self.lowering_emit_instr(IIRInstr::new(
                    "const",
                    Some(true_val.clone()),
                    vec![Operand::Bool(true)],
                    "bool",
                ))?;
                self.lowering_emit_instr(IIRInstr::new(
                    "mov",
                    Some(dest.clone()),
                    vec![Operand::Var(true_val)],
                    "bool",
                ))?;
            }
        }

        Ok(dest)
    }
}
