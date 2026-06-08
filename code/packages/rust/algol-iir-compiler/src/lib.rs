//! ALGOL 60 scalar frontend for the LANG VM Rust chain.
//!
//! This crate lowers parsed ALGOL 60 into [`interpreter_ir::IIRModule`], the
//! shared IR consumed by `vm-core`, `jit-core`, `aot-core`, and the direct IIR
//! backends for WASM, JVM, CLR, BEAM, and LLVM.
//!
//! The first slice is intentionally conservative: it supports scalar
//! `integer` and `boolean` programs only. ALGOL features that need a richer
//! runtime model, such as arrays, procedures, strings, reals, switches, nested
//! declaration scopes, and by-name calls, fail with explicit errors instead of
//! silently producing partial IR.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use coding_adventures_algol_parser::parse_algol;
use interpreter_ir::{FunctionTypeStatus, IIRFunction, IIRInstr, IIRModule, Operand, SourceLoc};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use vm_core::core::VMCore;
use vm_core::errors::VMError;
use vm_core::value::Value;

/// Errors raised while compiling ALGOL 60 into IIR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// The lexer/parser rejected the source.
    Parse(String),
    /// The AST shape did not match the embedded ALGOL grammar.
    Malformed(String),
    /// The program uses a valid ALGOL construct outside this scalar slice.
    Unsupported(String),
    /// Static scalar type checking failed.
    Type(String),
    /// The emitted module failed IIR validation.
    Validation(Vec<String>),
    /// The VM failed while executing compiled ALGOL.
    Runtime(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "ALGOL 60 parse failed: {msg}"),
            Self::Malformed(msg) => write!(f, "ALGOL 60 AST malformed: {msg}"),
            Self::Unsupported(msg) => write!(f, "ALGOL 60 scalar IIR does not support {msg} yet"),
            Self::Type(msg) => write!(f, "ALGOL 60 type error: {msg}"),
            Self::Validation(errs) => {
                write!(f, "emitted IIR failed validation: {}", errs.join("; "))
            }
            Self::Runtime(msg) => write!(f, "ALGOL 60 VM runtime error: {msg}"),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<VMError> for CompileError {
    fn from(value: VMError) -> Self {
        Self::Runtime(format!("{value:?}"))
    }
}

/// Compile an ALGOL 60 source string to a LANG VM [`IIRModule`].
///
/// If a scalar variable named `result` is declared, `main` returns its final
/// value. Otherwise `main` returns `i64 0`, matching the AOT exit-code
/// convention used by other LANG frontends.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, CompileError> {
    let ast = std::panic::catch_unwind(|| parse_algol(source)).map_err(|payload| {
        if let Some(msg) = payload.downcast_ref::<String>() {
            CompileError::Parse(msg.clone())
        } else if let Some(msg) = payload.downcast_ref::<&str>() {
            CompileError::Parse((*msg).to_string())
        } else {
            CompileError::Parse("parser panicked".to_string())
        }
    })?;
    compile_ast(&ast, module_name)
}

/// Compile a parsed ALGOL 60 AST into a LANG VM [`IIRModule`].
pub fn compile_ast(ast: &GrammarASTNode, module_name: &str) -> Result<IIRModule, CompileError> {
    if ast.rule_name != "program" {
        return Err(CompileError::Malformed(format!(
            "expected root rule 'program', got {:?}",
            ast.rule_name
        )));
    }

    let block = first_direct_node(ast, "block")
        .ok_or_else(|| CompileError::Malformed("program has no block child".into()))?;

    let mut compiler = Compiler::default();
    compiler.emit_block(block, true)?;
    compiler.finish(module_name)
}

/// Compile and run an ALGOL 60 source string through `vm-core`.
pub fn execute_source(source: &str, module_name: &str) -> Result<Option<Value>, CompileError> {
    let mut module = compile_source(source, module_name)?;
    let mut vm = VMCore::new();
    Ok(vm.execute(&mut module, "main", &[])?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScalarType {
    Integer,
    Boolean,
}

impl ScalarType {
    fn iir(self) -> &'static str {
        match self {
            Self::Integer => "i64",
            Self::Boolean => "bool",
        }
    }

    fn default_operand(self) -> Operand {
        match self {
            Self::Integer => Operand::Int(0),
            Self::Boolean => Operand::Bool(false),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Boolean => "boolean",
        }
    }
}

#[derive(Debug, Clone)]
struct ExprValue {
    slot: String,
    ty: ScalarType,
}

#[derive(Debug, Clone)]
enum Piece<'a> {
    Node(&'a GrammarASTNode),
    Op(String),
}

struct Compiler {
    instrs: Vec<IIRInstr>,
    source_map: Vec<SourceLoc>,
    current_loc: Cell<SourceLoc>,
    vars: HashMap<String, ScalarType>,
    temp_counter: usize,
    label_counter: usize,
    register_names: HashSet<String>,
    defined_labels: HashSet<String>,
    referenced_labels: HashSet<String>,
}

impl Default for Compiler {
    fn default() -> Self {
        Self {
            instrs: Vec::new(),
            source_map: Vec::new(),
            current_loc: Cell::new(SourceLoc::SYNTHETIC),
            vars: HashMap::new(),
            temp_counter: 0,
            label_counter: 0,
            register_names: HashSet::new(),
            defined_labels: HashSet::new(),
            referenced_labels: HashSet::new(),
        }
    }
}

impl Compiler {
    fn finish(mut self, module_name: &str) -> Result<IIRModule, CompileError> {
        for label in &self.referenced_labels {
            if !self.defined_labels.contains(label) {
                return Err(CompileError::Malformed(format!(
                    "goto references undefined label {label:?}"
                )));
            }
        }

        let (return_type, return_src) = match self.vars.get("result").copied() {
            Some(ty) => (ty.iir(), Operand::Var("result".to_string())),
            None => {
                self.emit(IIRInstr::new(
                    "const",
                    Some("_algol_exit".to_string()),
                    vec![Operand::Int(0)],
                    "i64",
                ));
                self.register_names.insert("_algol_exit".to_string());
                ("i64", Operand::Var("_algol_exit".to_string()))
            }
        };

        self.emit(IIRInstr::new("ret", None, vec![return_src], return_type));

        let body_len = self.instrs.len();
        let mut main = IIRFunction::new("main", vec![], return_type, self.instrs);
        main.type_status = FunctionTypeStatus::FullyTyped;
        main.register_count = self.register_names.len().saturating_add(8).max(8);
        while self.source_map.len() < body_len {
            self.source_map.push(SourceLoc::SYNTHETIC);
        }
        if self.source_map.len() > body_len {
            self.source_map.truncate(body_len);
        }
        main.source_map = self.source_map;

        let mut module = IIRModule::new(module_name, "algol60");
        module.functions.push(main);
        module.entry_point = Some("main".to_string());

        let validation = module.validate();
        if validation.is_empty() {
            Ok(module)
        } else {
            Err(CompileError::Validation(validation))
        }
    }

    fn emit_block(&mut self, node: &GrammarASTNode, is_root: bool) -> Result<(), CompileError> {
        self.set_loc(node);

        if !is_root
            && direct_nodes(node)
                .iter()
                .any(|n| n.rule_name == "declaration")
        {
            return Err(CompileError::Unsupported(
                "nested block declarations and lexical scope".into(),
            ));
        }

        for child in direct_nodes(node) {
            if child.rule_name == "declaration" {
                self.emit_declaration(child)?;
            }
        }
        for child in direct_nodes(node) {
            if child.rule_name == "statement" {
                self.emit_statement(child)?;
            }
        }
        Ok(())
    }

    fn emit_declaration(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        let Some(type_decl) = first_direct_node(node, "type_decl") else {
            let construct = direct_nodes(node)
                .first()
                .map(|n| n.rule_name.as_str())
                .unwrap_or("declaration");
            return Err(CompileError::Unsupported(format!(
                "{construct} declarations"
            )));
        };
        let type_node = first_direct_node(type_decl, "type")
            .ok_or_else(|| CompileError::Malformed("type_decl missing type".into()))?;
        let ty = self.scalar_type(type_node)?;
        let ident_list = first_direct_node(type_decl, "ident_list")
            .ok_or_else(|| CompileError::Malformed("type_decl missing ident_list".into()))?;

        for name in direct_tokens(ident_list)
            .into_iter()
            .filter(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
        {
            if self.vars.insert(name.clone(), ty).is_some() {
                return Err(CompileError::Type(format!(
                    "duplicate declaration for {name:?}"
                )));
            }
            self.register_names.insert(name.clone());
            self.emit(IIRInstr::new(
                "const",
                Some(name),
                vec![ty.default_operand()],
                ty.iir(),
            ));
        }
        Ok(())
    }

    fn scalar_type(&self, node: &GrammarASTNode) -> Result<ScalarType, CompileError> {
        let token = single_token_recursive(node)
            .ok_or_else(|| CompileError::Malformed("type node has no token".into()))?;
        match token.value.as_str() {
            "integer" => Ok(ScalarType::Integer),
            "boolean" => Ok(ScalarType::Boolean),
            "real" => Err(CompileError::Unsupported(
                "real scalars on the common VM/JIT/backend slice".into(),
            )),
            "string" => Err(CompileError::Unsupported("string scalars".into())),
            other => Err(CompileError::Malformed(format!(
                "unknown type token {other:?}"
            ))),
        }
    }

    fn emit_statement(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);

        let children = direct_nodes(node);
        if let Some(label) = children.iter().find(|n| n.rule_name == "label") {
            let name = self.label_name(label)?;
            self.defined_labels.insert(name.clone());
            self.emit(IIRInstr::new(
                "label",
                None,
                vec![Operand::Var(name)],
                "void",
            ));
        }

        if let Some(cond) = children.iter().find(|n| n.rule_name == "cond_stmt") {
            return self.emit_cond_stmt(cond);
        }
        if let Some(unlabeled) = children.iter().find(|n| n.rule_name == "unlabeled_stmt") {
            return self.emit_unlabeled_stmt(unlabeled);
        }
        Ok(())
    }

    fn emit_unlabeled_stmt(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        let child = direct_nodes(node)
            .first()
            .copied()
            .ok_or_else(|| CompileError::Malformed("unlabeled_stmt has no child".into()))?;
        match child.rule_name.as_str() {
            "assign_stmt" => self.emit_assignment(child),
            "goto_stmt" => self.emit_goto(child),
            "compound_stmt" => self.emit_compound(child),
            "for_stmt" => self.emit_for(child),
            "block" => self.emit_block(child, false),
            "proc_stmt" => Err(CompileError::Unsupported(
                "procedure call statements".into(),
            )),
            other => Err(CompileError::Unsupported(format!("{other} statements"))),
        }
    }

    fn emit_compound(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        for stmt in direct_nodes(node)
            .into_iter()
            .filter(|n| n.rule_name == "statement")
        {
            self.emit_statement(stmt)?;
        }
        Ok(())
    }

    fn emit_assignment(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        let left_parts: Vec<&GrammarASTNode> = direct_nodes(node)
            .into_iter()
            .filter(|n| n.rule_name == "left_part")
            .collect();
        if left_parts.is_empty() {
            return Err(CompileError::Malformed(
                "assign_stmt has no left_part".into(),
            ));
        }
        let expr = first_direct_node(node, "expression")
            .ok_or_else(|| CompileError::Malformed("assign_stmt has no expression".into()))?;
        let rhs = self.emit_expr(expr)?;

        for left in left_parts {
            let var_node = first_direct_node(left, "variable")
                .ok_or_else(|| CompileError::Malformed("left_part has no variable".into()))?;
            let name = self.simple_variable_name(var_node)?;
            let expected = self.require_var(&name)?;
            if expected != rhs.ty {
                return Err(CompileError::Type(format!(
                    "cannot assign {} expression to {} variable {name:?}",
                    rhs.ty.name(),
                    expected.name()
                )));
            }
            self.emit(IIRInstr::new(
                "mov",
                Some(name),
                vec![Operand::Var(rhs.slot.clone())],
                expected.iir(),
            ));
        }
        Ok(())
    }

    fn emit_goto(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        let desig = first_direct_node(node, "desig_expr")
            .ok_or_else(|| CompileError::Malformed("goto_stmt has no desig_expr".into()))?;
        let label = self.designational_label(desig)?;
        self.referenced_labels.insert(label.clone());
        self.emit(IIRInstr::new(
            "jmp",
            None,
            vec![Operand::Var(label)],
            "void",
        ));
        Ok(())
    }

    fn emit_cond_stmt(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        let children = direct_nodes(node);
        let cond_node = children
            .iter()
            .find(|n| n.rule_name == "bool_expr")
            .copied()
            .ok_or_else(|| CompileError::Malformed("cond_stmt missing bool_expr".into()))?;
        let cond = self.emit_expr(cond_node)?;
        if cond.ty != ScalarType::Boolean {
            return Err(CompileError::Type("if condition must be boolean".into()));
        }

        let branches: Vec<&GrammarASTNode> = children
            .into_iter()
            .filter(|n| n.rule_name == "unlabeled_stmt" || n.rule_name == "statement")
            .collect();
        let then_branch = branches
            .first()
            .copied()
            .ok_or_else(|| CompileError::Malformed("cond_stmt missing then branch".into()))?;
        let else_branch = branches.get(1).copied();

        let else_label = self.fresh_label("if_else");
        let end_label = self.fresh_label("if_end");

        self.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(cond.slot), Operand::Var(else_label.clone())],
            "void",
        ));
        self.emit_branch_node(then_branch)?;
        self.emit(IIRInstr::new(
            "jmp",
            None,
            vec![Operand::Var(end_label.clone())],
            "void",
        ));
        self.emit_label(&else_label);
        if let Some(branch) = else_branch {
            self.emit_branch_node(branch)?;
        }
        self.emit_label(&end_label);
        Ok(())
    }

    fn emit_branch_node(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        match node.rule_name.as_str() {
            "unlabeled_stmt" => self.emit_unlabeled_stmt(node),
            "statement" => self.emit_statement(node),
            other => Err(CompileError::Malformed(format!(
                "expected branch statement, got {other:?}"
            ))),
        }
    }

    fn emit_for(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        let var_name = direct_tokens(node)
            .into_iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CompileError::Malformed("for_stmt missing loop variable".into()))?;
        let var_ty = self.require_var(&var_name)?;
        if var_ty != ScalarType::Integer {
            return Err(CompileError::Type(format!(
                "for variable {var_name:?} must be integer"
            )));
        }

        let for_list = first_direct_node(node, "for_list")
            .ok_or_else(|| CompileError::Malformed("for_stmt missing for_list".into()))?;
        let elems: Vec<&GrammarASTNode> = direct_nodes(for_list)
            .into_iter()
            .filter(|n| n.rule_name == "for_elem")
            .collect();
        if elems.len() != 1 {
            return Err(CompileError::Unsupported(
                "multi-element ALGOL for lists".into(),
            ));
        }
        let elem = elems[0];
        if direct_tokens(elem).iter().any(|t| t.value == "while") {
            return Err(CompileError::Unsupported("for while elements".into()));
        }
        if !direct_tokens(elem).iter().any(|t| t.value == "step") {
            return Err(CompileError::Unsupported(
                "single-value for elements outside step/until form".into(),
            ));
        }
        let arith_nodes: Vec<&GrammarASTNode> = direct_nodes(elem)
            .into_iter()
            .filter(|n| n.rule_name == "arith_expr")
            .collect();
        if arith_nodes.len() != 3 {
            return Err(CompileError::Malformed(
                "step/until for element should have start, step, and limit".into(),
            ));
        }

        let start = self.emit_expr(arith_nodes[0])?;
        let step = self.emit_expr(arith_nodes[1])?;
        let limit = self.emit_expr(arith_nodes[2])?;
        if start.ty != ScalarType::Integer
            || step.ty != ScalarType::Integer
            || limit.ty != ScalarType::Integer
        {
            return Err(CompileError::Type(
                "for bounds and step must be integer".into(),
            ));
        }
        let step_const = const_i64_from_node(arith_nodes[1])
            .ok_or_else(|| CompileError::Unsupported("non-constant for step values".into()))?;
        let cmp_op = if step_const >= 0 { "cmp_le" } else { "cmp_ge" };

        let body = direct_nodes(node)
            .into_iter()
            .find(|n| n.rule_name == "statement")
            .ok_or_else(|| CompileError::Malformed("for_stmt missing body statement".into()))?;

        self.emit(IIRInstr::new(
            "mov",
            Some(var_name.clone()),
            vec![Operand::Var(start.slot)],
            "i64",
        ));

        let loop_label = self.fresh_label("for_loop");
        let end_label = self.fresh_label("for_end");
        self.emit_label(&loop_label);
        let cond = self.fresh_temp();
        self.emit(IIRInstr::new(
            cmp_op,
            Some(cond.clone()),
            vec![Operand::Var(var_name.clone()), Operand::Var(limit.slot)],
            "bool",
        ));
        self.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(cond), Operand::Var(end_label.clone())],
            "void",
        ));
        self.emit_statement(body)?;
        let next = self.fresh_temp();
        self.emit(IIRInstr::new(
            "add",
            Some(next.clone()),
            vec![Operand::Var(var_name.clone()), Operand::Var(step.slot)],
            "i64",
        ));
        self.emit(IIRInstr::new(
            "mov",
            Some(var_name),
            vec![Operand::Var(next)],
            "i64",
        ));
        self.emit(IIRInstr::new(
            "jmp",
            None,
            vec![Operand::Var(loop_label)],
            "void",
        ));
        self.emit_label(&end_label);
        Ok(())
    }

    fn emit_expr(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        self.set_loc(node);

        if direct_tokens(node).iter().any(|t| t.value == "if") {
            return Err(CompileError::Unsupported("conditional expressions".into()));
        }

        match node.rule_name.as_str() {
            "variable" => {
                let name = self.simple_variable_name(node)?;
                let ty = self.require_var(&name)?;
                Ok(ExprValue { slot: name, ty })
            }
            "proc_call" => Err(CompileError::Unsupported(
                "procedure calls in expressions".into(),
            )),
            "expression" | "arith_expr" | "bool_expr" => self.emit_single_child_expr(node),
            "expr_eqv" | "expr_impl" | "expr_or" | "expr_and" | "simple_bool" | "implication"
            | "bool_term" | "bool_factor" => self.emit_bool_wrapper(node),
            "expr_not" | "bool_secondary" => self.emit_not_or_child(node),
            "expr_cmp" | "relation" => self.emit_binary_or_child(node, BinaryFamily::Comparison),
            "expr_add" | "simple_arith" => self.emit_binary_or_child(node, BinaryFamily::Additive),
            "expr_mul" | "term" => self.emit_binary_or_child(node, BinaryFamily::Multiplicative),
            "expr_pow" | "factor" => {
                if pieces(node)
                    .iter()
                    .any(|p| matches!(p, Piece::Op(op) if op == "^" || op == "**"))
                {
                    return Err(CompileError::Unsupported("exponentiation".into()));
                }
                self.emit_single_child_expr(node)
            }
            "expr_atom" | "primary" | "bool_primary" => self.emit_atom(node),
            other => {
                if let Some(token) = single_token_recursive(node) {
                    self.emit_token_atom(token)
                } else {
                    Err(CompileError::Malformed(format!(
                        "cannot lower expression node {other:?}"
                    )))
                }
            }
        }
    }

    fn emit_bool_wrapper(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        let found_op = pieces(node).into_iter().find_map(|p| {
            if let Piece::Op(op) = p {
                matches!(op.as_str(), "and" | "or" | "impl" | "eqv").then_some(op)
            } else {
                None
            }
        });
        if let Some(op) = found_op {
            return Err(CompileError::Unsupported(format!(
                "boolean operator {op:?}"
            )));
        }
        self.emit_single_child_expr(node)
    }

    fn emit_not_or_child(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        if direct_tokens(node).iter().any(|t| t.value == "not") {
            let child = direct_nodes(node)
                .first()
                .copied()
                .ok_or_else(|| CompileError::Malformed("not expression missing operand".into()))?;
            let value = self.emit_expr(child)?;
            if value.ty != ScalarType::Boolean {
                return Err(CompileError::Type("not operand must be boolean".into()));
            }
            let false_slot = self.emit_const(ScalarType::Boolean, Operand::Bool(false));
            let dest = self.fresh_temp();
            self.emit(IIRInstr::new(
                "cmp_eq",
                Some(dest.clone()),
                vec![Operand::Var(value.slot), Operand::Var(false_slot)],
                "bool",
            ));
            Ok(ExprValue {
                slot: dest,
                ty: ScalarType::Boolean,
            })
        } else {
            self.emit_single_child_expr(node)
        }
    }

    fn emit_single_child_expr(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        let mut child_nodes = direct_nodes(node);
        child_nodes.retain(|n| is_expr_like(n));
        if child_nodes.len() == 1 {
            self.emit_expr(child_nodes[0])
        } else {
            Err(CompileError::Malformed(format!(
                "{:?} expected one expression child, got {}",
                node.rule_name,
                child_nodes.len()
            )))
        }
    }

    fn emit_atom(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        if let Some(token) = direct_tokens(node)
            .into_iter()
            .find(|t| is_literal_token(t))
        {
            return self.emit_token_atom(token);
        }
        let child_nodes = direct_nodes(node);
        if child_nodes.len() == 1 {
            self.emit_expr(child_nodes[0])
        } else {
            Err(CompileError::Malformed(format!(
                "atom node expected one child or literal, got {} children",
                child_nodes.len()
            )))
        }
    }

    fn emit_token_atom(&mut self, token: &Token) -> Result<ExprValue, CompileError> {
        match (token.effective_type_name(), token.value.as_str()) {
            ("INTEGER_LIT", _) => {
                let value = token.value.parse::<i64>().map_err(|_| {
                    CompileError::Type(format!("integer literal {:?} overflows i64", token.value))
                })?;
                let slot = self.emit_const(ScalarType::Integer, Operand::Int(value));
                Ok(ExprValue {
                    slot,
                    ty: ScalarType::Integer,
                })
            }
            ("REAL_LIT", _) => Err(CompileError::Unsupported(
                "real literals on the common VM/JIT/backend slice".into(),
            )),
            ("STRING_LIT", _) => Err(CompileError::Unsupported("string literals".into())),
            ("KEYWORD", "true") => {
                let slot = self.emit_const(ScalarType::Boolean, Operand::Bool(true));
                Ok(ExprValue {
                    slot,
                    ty: ScalarType::Boolean,
                })
            }
            ("KEYWORD", "false") => {
                let slot = self.emit_const(ScalarType::Boolean, Operand::Bool(false));
                Ok(ExprValue {
                    slot,
                    ty: ScalarType::Boolean,
                })
            }
            ("NAME", _) => {
                let name = token.value.clone();
                let ty = self.require_var(&name)?;
                Ok(ExprValue { slot: name, ty })
            }
            _ => Err(CompileError::Malformed(format!(
                "unexpected atom token {} {:?}",
                token.effective_type_name(),
                token.value
            ))),
        }
    }

    fn emit_binary_or_child(
        &mut self,
        node: &GrammarASTNode,
        family: BinaryFamily,
    ) -> Result<ExprValue, CompileError> {
        let seq = pieces(node);
        let op_count = seq.iter().filter(|p| matches!(p, Piece::Op(_))).count();
        if op_count == 0 {
            return self.emit_single_child_expr(node);
        }

        let mut idx = 0;
        let mut leading_minus = false;
        if matches!(family, BinaryFamily::Additive) {
            if let Some(Piece::Op(op)) = seq.get(idx) {
                if op == "+" || op == "-" {
                    leading_minus = op == "-";
                    idx += 1;
                }
            }
        }

        let first = match seq.get(idx) {
            Some(Piece::Node(n)) => *n,
            _ => {
                return Err(CompileError::Malformed(format!(
                    "{:?} expected expression after unary sign/operator",
                    node.rule_name
                )))
            }
        };
        idx += 1;
        let mut acc = self.emit_expr(first)?;
        if leading_minus {
            acc = self.emit_unary_minus(acc)?;
        }

        while idx < seq.len() {
            let op = match seq.get(idx) {
                Some(Piece::Op(op)) => op.clone(),
                Some(Piece::Node(_)) => {
                    return Err(CompileError::Malformed(format!(
                        "{:?} has adjacent expression children without an operator",
                        node.rule_name
                    )))
                }
                None => break,
            };
            idx += 1;
            let rhs_node = match seq.get(idx) {
                Some(Piece::Node(n)) => *n,
                _ => {
                    return Err(CompileError::Malformed(format!(
                        "{:?} operator {op:?} missing right operand",
                        node.rule_name
                    )))
                }
            };
            idx += 1;
            let rhs = self.emit_expr(rhs_node)?;
            acc = self.emit_binary(&op, acc, rhs)?;
        }
        Ok(acc)
    }

    fn emit_binary(
        &mut self,
        op: &str,
        lhs: ExprValue,
        rhs: ExprValue,
    ) -> Result<ExprValue, CompileError> {
        match op {
            "+" | "-" | "*" | "div" => {
                if lhs.ty != ScalarType::Integer || rhs.ty != ScalarType::Integer {
                    return Err(CompileError::Type(format!(
                        "operator {op:?} requires integer operands"
                    )));
                }
                let iir_op = match op {
                    "+" => "add",
                    "-" => "sub",
                    "*" => "mul",
                    "div" => "div",
                    _ => unreachable!(),
                };
                let dest = self.fresh_temp();
                self.emit(IIRInstr::new(
                    iir_op,
                    Some(dest.clone()),
                    vec![Operand::Var(lhs.slot), Operand::Var(rhs.slot)],
                    "i64",
                ));
                Ok(ExprValue {
                    slot: dest,
                    ty: ScalarType::Integer,
                })
            }
            "/" => Err(CompileError::Unsupported(
                "real division '/' in the integer-only backend slice; use div".into(),
            )),
            "mod" => Err(CompileError::Unsupported(
                "mod until the WASM backend accepts the shared IIR 'mod' opcode".into(),
            )),
            "=" | "!=" | "<>" | "<" | "<=" | ">" | ">=" => {
                if lhs.ty != rhs.ty {
                    return Err(CompileError::Type(format!(
                        "cannot compare {} and {}",
                        lhs.ty.name(),
                        rhs.ty.name()
                    )));
                }
                if lhs.ty == ScalarType::Boolean && !matches!(op, "=" | "!=" | "<>") {
                    return Err(CompileError::Type(
                        "boolean ordering comparisons are not supported".into(),
                    ));
                }
                let iir_op = match op {
                    "=" => "cmp_eq",
                    "!=" | "<>" => "cmp_ne",
                    "<" => "cmp_lt",
                    "<=" => "cmp_le",
                    ">" => "cmp_gt",
                    ">=" => "cmp_ge",
                    _ => unreachable!(),
                };
                let dest = self.fresh_temp();
                self.emit(IIRInstr::new(
                    iir_op,
                    Some(dest.clone()),
                    vec![Operand::Var(lhs.slot), Operand::Var(rhs.slot)],
                    "bool",
                ));
                Ok(ExprValue {
                    slot: dest,
                    ty: ScalarType::Boolean,
                })
            }
            "^" | "**" => Err(CompileError::Unsupported("exponentiation".into())),
            "and" | "or" | "impl" | "eqv" => Err(CompileError::Unsupported(format!(
                "boolean operator {op:?}"
            ))),
            other => Err(CompileError::Malformed(format!(
                "unknown operator {other:?}"
            ))),
        }
    }

    fn emit_unary_minus(&mut self, value: ExprValue) -> Result<ExprValue, CompileError> {
        if value.ty != ScalarType::Integer {
            return Err(CompileError::Type("unary minus requires an integer".into()));
        }
        let zero = self.emit_const(ScalarType::Integer, Operand::Int(0));
        let dest = self.fresh_temp();
        self.emit(IIRInstr::new(
            "sub",
            Some(dest.clone()),
            vec![Operand::Var(zero), Operand::Var(value.slot)],
            "i64",
        ));
        Ok(ExprValue {
            slot: dest,
            ty: ScalarType::Integer,
        })
    }

    fn emit_const(&mut self, ty: ScalarType, operand: Operand) -> String {
        let dest = self.fresh_temp();
        self.emit(IIRInstr::new(
            "const",
            Some(dest.clone()),
            vec![operand],
            ty.iir(),
        ));
        dest
    }

    fn emit_label(&mut self, label: &str) {
        self.defined_labels.insert(label.to_string());
        self.emit(IIRInstr::new(
            "label",
            None,
            vec![Operand::Var(label.to_string())],
            "void",
        ));
    }

    fn emit(&mut self, instr: IIRInstr) {
        self.instrs.push(instr);
        self.source_map.push(self.current_loc.get());
    }

    fn set_loc(&self, node: &GrammarASTNode) {
        self.current_loc.set(node_loc(node));
    }

    fn fresh_temp(&mut self) -> String {
        let name = format!("_t{}", self.temp_counter);
        self.temp_counter += 1;
        self.register_names.insert(name.clone());
        name
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let name = format!("__algol_{prefix}_{}", self.label_counter);
        self.label_counter += 1;
        name
    }

    fn require_var(&self, name: &str) -> Result<ScalarType, CompileError> {
        self.vars
            .get(name)
            .copied()
            .ok_or_else(|| CompileError::Type(format!("use of undeclared variable {name:?}")))
    }

    fn simple_variable_name(&self, node: &GrammarASTNode) -> Result<String, CompileError> {
        if direct_tokens(node)
            .iter()
            .any(|t| t.effective_type_name() == "LBRACKET")
        {
            return Err(CompileError::Unsupported(
                "array variables/subscripts".into(),
            ));
        }
        let names: Vec<&Token> = direct_tokens(node)
            .into_iter()
            .filter(|t| t.effective_type_name() == "NAME")
            .collect();
        if names.len() == 1 {
            Ok(names[0].value.clone())
        } else {
            Err(CompileError::Malformed(format!(
                "variable should contain exactly one NAME token, got {}",
                names.len()
            )))
        }
    }

    fn label_name(&self, node: &GrammarASTNode) -> Result<String, CompileError> {
        let tokens: Vec<&Token> = direct_tokens(node)
            .into_iter()
            .filter(|t| matches!(t.effective_type_name(), "NAME" | "INTEGER_LIT"))
            .collect();
        if tokens.len() == 1 {
            Ok(format!("L_{}", tokens[0].value))
        } else {
            Err(CompileError::Malformed(format!(
                "label should contain exactly one NAME or INTEGER_LIT token, got {}",
                tokens.len()
            )))
        }
    }

    fn designational_label(&self, node: &GrammarASTNode) -> Result<String, CompileError> {
        if recursive_tokens(node).iter().any(|t| t.value == "if") {
            return Err(CompileError::Unsupported(
                "conditional designational expressions".into(),
            ));
        }
        if recursive_tokens(node)
            .iter()
            .any(|t| matches!(t.effective_type_name(), "LBRACKET" | "RBRACKET"))
        {
            return Err(CompileError::Unsupported(
                "switch/subscript designators".into(),
            ));
        }
        let tokens: Vec<&Token> = recursive_tokens(node)
            .into_iter()
            .filter(|t| matches!(t.effective_type_name(), "NAME" | "INTEGER_LIT"))
            .collect();
        if tokens.len() == 1 {
            Ok(format!("L_{}", tokens[0].value))
        } else {
            Err(CompileError::Malformed(format!(
                "goto designator should resolve to one label, got {} tokens",
                tokens.len()
            )))
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BinaryFamily {
    Additive,
    Multiplicative,
    Comparison,
}

fn node_loc(node: &GrammarASTNode) -> SourceLoc {
    match (node.start_line, node.start_column) {
        (Some(line), Some(col)) => SourceLoc::new(line, col),
        _ => SourceLoc::SYNTHETIC,
    }
}

fn direct_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|child| match child {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

fn direct_tokens(node: &GrammarASTNode) -> Vec<&Token> {
    node.children
        .iter()
        .filter_map(|child| match child {
            ASTNodeOrToken::Token(t) => Some(t),
            ASTNodeOrToken::Node(_) => None,
        })
        .collect()
}

fn recursive_tokens(node: &GrammarASTNode) -> Vec<&Token> {
    let mut out = Vec::new();
    collect_tokens(node, &mut out);
    out
}

fn collect_tokens<'a>(node: &'a GrammarASTNode, out: &mut Vec<&'a Token>) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) => out.push(t),
            ASTNodeOrToken::Node(n) => collect_tokens(n, out),
        }
    }
}

fn first_direct_node<'a>(node: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Node(n) if n.rule_name == rule => Some(n),
        _ => None,
    })
}

fn single_token_recursive(node: &GrammarASTNode) -> Option<&Token> {
    let tokens = recursive_tokens(node);
    (tokens.len() == 1).then_some(tokens[0])
}

fn is_expr_like(node: &GrammarASTNode) -> bool {
    matches!(
        node.rule_name.as_str(),
        "expression"
            | "expr_eqv"
            | "expr_impl"
            | "expr_or"
            | "expr_and"
            | "expr_not"
            | "expr_cmp"
            | "expr_add"
            | "expr_mul"
            | "expr_pow"
            | "expr_atom"
            | "arith_expr"
            | "simple_arith"
            | "term"
            | "factor"
            | "primary"
            | "bool_expr"
            | "simple_bool"
            | "implication"
            | "bool_term"
            | "bool_factor"
            | "bool_secondary"
            | "bool_primary"
            | "relation"
            | "variable"
            | "proc_call"
    )
}

fn is_literal_token(token: &Token) -> bool {
    matches!(
        token.effective_type_name(),
        "INTEGER_LIT" | "REAL_LIT" | "STRING_LIT"
    ) || matches!(token.value.as_str(), "true" | "false")
}

fn pieces(node: &GrammarASTNode) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(n) if is_expr_like(n) => out.push(Piece::Node(n)),
            ASTNodeOrToken::Node(_) => {}
            ASTNodeOrToken::Token(t) => {
                if let Some(op) = operator_from_token(t) {
                    out.push(Piece::Op(op.to_string()));
                }
            }
        }
    }
    out
}

fn operator_from_token(token: &Token) -> Option<&'static str> {
    match token.effective_type_name() {
        "PLUS" => Some("+"),
        "MINUS" => Some("-"),
        "STAR" => Some("*"),
        "SLASH" => Some("/"),
        "EQ" => Some("="),
        "NEQ" => Some("!="),
        "LT" => Some("<"),
        "LEQ" => Some("<="),
        "GT" => Some(">"),
        "GEQ" => Some(">="),
        "CARET" => Some("^"),
        "POWER" => Some("**"),
        _ => match token.value.as_str() {
            "div" => Some("div"),
            "mod" => Some("mod"),
            "and" => Some("and"),
            "or" => Some("or"),
            "impl" => Some("impl"),
            "eqv" => Some("eqv"),
            _ => None,
        },
    }
}

fn const_i64_from_node(node: &GrammarASTNode) -> Option<i64> {
    let mut sign = 1i64;
    let mut literal: Option<i64> = None;
    for token in recursive_tokens(node) {
        match (token.effective_type_name(), token.value.as_str()) {
            ("PLUS", _) => {}
            ("MINUS", _) if literal.is_none() => sign = -sign,
            ("INTEGER_LIT", _) if literal.is_none() => {
                literal = token.value.parse::<i64>().ok();
            }
            ("LPAREN", _) | ("RPAREN", _) => {}
            _ => return None,
        }
    }
    literal.map(|n| sign * n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_i64(source: &str) -> i64 {
        let result = execute_source(source, "test")
            .expect("ALGOL source should compile and run")
            .expect("main should return a value");
        result.as_i64().expect("result should be an integer")
    }

    #[test]
    fn compiles_and_runs_integer_assignment() {
        let src = "begin integer result; result := 40 + 2 end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn compiles_and_runs_if_else() {
        let src = "begin integer x, result; x := 5; if x > 3 then result := 1 else result := 2 end";
        assert_eq!(run_i64(src), 1);
    }

    #[test]
    fn compiles_and_runs_for_step_until_sum() {
        let src = "begin integer i, result; result := 0; for i := 1 step 1 until 10 do result := result + i end";
        assert_eq!(run_i64(src), 55);
    }

    #[test]
    fn compiles_and_runs_goto_loop() {
        let src = "begin integer x, result; x := 0; loop: if x >= 5 then goto done; x := x + 1; goto loop; done: result := x end";
        assert_eq!(run_i64(src), 5);
    }

    #[test]
    fn boolean_not_assignment_runs() {
        let src = "begin boolean flag; integer result; flag := true; if not flag then result := 1 else result := 42 end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn rejects_real_declarations_cleanly() {
        let err = compile_source("begin real x; x := 1.5 end", "bad")
            .expect_err("real declarations are outside this slice");
        assert!(err.to_string().contains("real"));
    }

    #[test]
    fn source_map_tracks_every_instruction() {
        let module = compile_source("begin integer result; result := 42 end", "map")
            .expect("source should compile");
        let main = module.get_function("main").expect("main exists");
        assert_eq!(main.instructions.len(), main.source_map.len());
        assert_eq!(main.type_status, FunctionTypeStatus::FullyTyped);
    }
}
