//! ALGOL 60 scalar frontend for the LANG VM Rust chain.
//!
//! This crate lowers parsed ALGOL 60 into [`interpreter_ir::IIRModule`], the
//! shared IR consumed by `vm-core`, `jit-core`, `aot-core`, and the direct IIR
//! backends for WASM, JVM, CLR, BEAM, and LLVM.
//!
//! The first slice is intentionally conservative: it supports scalar
//! `integer` and `boolean` programs only. ALGOL features that need a richer
//! runtime model, such as arrays, procedures, strings, reals, switches, and
//! by-name calls, fail with explicit errors instead of
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
    /// ALGOL 60 `real` — an IEEE-754 double (`f64`) in the IIR (LANG-FULL AL1 /
    /// enabler E3).  Real arithmetic (`+`/`-`/`*`/`/`) and ordered comparisons
    /// lower to the IIR's `f64`-typed ops, which the WASM/LLVM/JVM backends and
    /// the VM/JIT execute as doubles.
    Real,
    Boolean,
}

impl ScalarType {
    fn iir(self) -> &'static str {
        match self {
            Self::Integer => "i64",
            Self::Real => "f64",
            Self::Boolean => "bool",
        }
    }

    fn default_operand(self) -> Operand {
        match self {
            Self::Integer => Operand::Int(0),
            Self::Real => Operand::Float(0.0),
            Self::Boolean => Operand::Bool(false),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Real => "real",
            Self::Boolean => "boolean",
        }
    }
}

#[derive(Debug, Clone)]
struct ExprValue {
    slot: String,
    ty: ScalarType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VarBinding {
    slot: String,
    ty: ScalarType,
}

/// A procedure heading read off the AST: `(name, value-params, return-type)`,
/// where each value parameter is `(name, type)` in declaration order.
type ProcedureParts = (String, Vec<(String, ScalarType)>, ScalarType);

/// The compile-time signature of a procedure: the ordered types of its
/// value parameters plus its return type.
///
/// ALGOL 60 lets a procedure be *called before it is textually declared*
/// (mutual recursion lives on this), so we register every procedure's
/// signature in a pre-pass over the block — `proc_sigs` below — before we
/// lower any procedure *body*.  When a call site is reached the lowerer
/// looks the name up here to know (a) how many arguments to evaluate,
/// (b) what type each argument must be, and (c) what type the call yields.
///
/// We deliberately only model **typed** procedures (ALGOL "function
/// procedures") with **value** parameters.  A proper (void) procedure has
/// no observable effect on the current executable slice — there is no
/// output statement and no by-reference / enclosing-scope mutation yet —
/// so admitting one would be lowering code no test could ever witness.
/// Those are rejected with a clear message and tracked as follow-up work.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcSig {
    /// Parameter types in declaration order (matches `IIRFunction::params`).
    params: Vec<ScalarType>,
    /// The procedure's return type (always `Some` on the supported slice).
    ret: ScalarType,
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
    scopes: Vec<HashMap<String, VarBinding>>,
    scope_counter: usize,
    temp_counter: usize,
    label_counter: usize,
    register_names: HashSet<String>,
    defined_labels: HashSet<String>,
    referenced_labels: HashSet<String>,
    /// Procedures lowered out of line, in declaration order.  These become
    /// extra `IIRFunction`s alongside `main` when the module is assembled.
    functions: Vec<IIRFunction>,
    /// Procedure name → signature, registered in a pre-pass so a call can be
    /// lowered before the callee's body is (forward references / recursion).
    proc_sigs: HashMap<String, ProcSig>,
    /// Switch name → its ordered list of target label slots.  A
    /// `switch s := first, second` becomes `s → ["L_first", "L_second"]`, and a
    /// `goto s[i]` (1-based) selects the i-th target.  Declared in the block's
    /// declaration part, before the statements that use it.
    switches: HashMap<String, Vec<String>>,
}

impl Default for Compiler {
    fn default() -> Self {
        Self {
            instrs: Vec::new(),
            source_map: Vec::new(),
            current_loc: Cell::new(SourceLoc::SYNTHETIC),
            scopes: vec![HashMap::new()],
            scope_counter: 0,
            temp_counter: 0,
            label_counter: 0,
            register_names: HashSet::new(),
            defined_labels: HashSet::new(),
            referenced_labels: HashSet::new(),
            functions: Vec::new(),
            proc_sigs: HashMap::new(),
            switches: HashMap::new(),
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

        let (return_type, return_src) = match self
            .scopes
            .first()
            .and_then(|scope| scope.get("result"))
            .cloned()
        {
            Some(binding) => (binding.ty.iir(), Operand::Var(binding.slot)),
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
        // Out-of-line procedures lowered during the block become sibling
        // functions of `main`.  Every backend iterates `module.functions`, so
        // a same-module `call` resolves the callee's signature by name.
        for proc in self.functions {
            module.functions.push(proc);
        }
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

        if !is_root {
            self.push_scope();
        }

        // Pass 0 — register every procedure's signature *before* lowering any
        // body.  ALGOL allows a call to appear ahead of the textual
        // declaration (and a procedure may call itself), so the call site must
        // be able to resolve the signature even though the body is not yet
        // compiled.
        for child in direct_nodes(node) {
            if child.rule_name == "declaration" {
                if let Some(proc_decl) = first_direct_node(child, "procedure_decl") {
                    self.register_proc_sig(proc_decl)?;
                }
            }
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
        if !is_root {
            self.pop_scope();
        }
        Ok(())
    }

    fn emit_declaration(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        // A procedure declaration is lowered out of line into its own
        // `IIRFunction` (its signature was already registered in pass 0).
        if let Some(proc_decl) = first_direct_node(node, "procedure_decl") {
            let func = self.compile_procedure(proc_decl)?;
            self.functions.push(func);
            return Ok(());
        }
        // A switch declaration records a named jump table; the labels it lists
        // are resolved (and validated) when a `goto s[i]` uses it.
        if let Some(switch_decl) = first_direct_node(node, "switch_decl") {
            return self.register_switch(switch_decl);
        }
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
            let slot = self.declare_var(&name, ty)?;
            self.emit(IIRInstr::new(
                "const",
                Some(slot),
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
            "real" => Ok(ScalarType::Real),
            "boolean" => Ok(ScalarType::Boolean),
            "string" => Err(CompileError::Unsupported("string scalars".into())),
            other => Err(CompileError::Malformed(format!(
                "unknown type token {other:?}"
            ))),
        }
    }

    /// Map a `specifier` keyword (the type a `spec_part` attaches to a formal
    /// parameter) to a scalar type.  `specifier` is a superset of `type`: it
    /// also admits `array`, `label`, `switch`, and `procedure`, none of which
    /// the current executable slice carries, so those produce a clear
    /// "unsupported" message rather than a confusing "unknown token".
    fn specifier_scalar_type(&self, node: &GrammarASTNode) -> Result<ScalarType, CompileError> {
        let token = single_token_recursive(node)
            .ok_or_else(|| CompileError::Malformed("specifier has no token".into()))?;
        match token.value.as_str() {
            "integer" => Ok(ScalarType::Integer),
            "real" => Ok(ScalarType::Real),
            "boolean" => Ok(ScalarType::Boolean),
            "string" => Err(CompileError::Unsupported("string parameters".into())),
            kind @ ("array" | "label" | "switch" | "procedure") => Err(
                CompileError::Unsupported(format!("{kind} parameters")),
            ),
            other => Err(CompileError::Malformed(format!(
                "unknown specifier token {other:?}"
            ))),
        }
    }

    /// Read a `procedure_decl` node into `(name, value-params, return-type)`.
    ///
    /// The grammar splits a procedure heading across three places:
    ///
    /// ```text
    /// integer procedure sq(x);   value x;   integer x;   sq := x*x
    ///   ^type   ^kw    ^name(^formal_params) ^value_part ^spec_part  ^proc_body
    /// ```
    ///
    /// * `formal_params` gives the parameter *names* in call order.
    /// * `value_part` lists which of them are passed **by value** (a copy).
    /// * each `spec_part` declares the *type* of one or more parameters.
    ///
    /// On the supported slice every parameter must be a `value` parameter
    /// (call-by-name / Jensen's device is not modelled), the procedure must
    /// have a return type (proper/void procedures are inert here), and every
    /// parameter must be specified exactly once.
    fn procedure_parts(
        &self,
        proc_decl: &GrammarASTNode,
    ) -> Result<ProcedureParts, CompileError> {
        let name = direct_tokens(proc_decl)
            .into_iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CompileError::Malformed("procedure_decl missing name".into()))?;

        let ret = match first_direct_node(proc_decl, "type") {
            Some(type_node) => self.scalar_type(type_node)?,
            None => {
                return Err(CompileError::Unsupported(format!(
                    "proper (void) procedure {name:?}: only typed procedures with a return \
                     value are observable on the current ALGOL slice"
                )))
            }
        };

        // Parameter names, in call order, from `formal_params`.
        let param_names: Vec<String> = match first_direct_node(proc_decl, "formal_params") {
            Some(fp) => match first_direct_node(fp, "ident_list") {
                Some(list) => ident_list_names(list),
                None => Vec::new(),
            },
            None => Vec::new(),
        };

        // Which parameters are passed by value.
        let value_names: HashSet<String> = match first_direct_node(proc_decl, "value_part") {
            Some(vp) => first_direct_node(vp, "ident_list")
                .map(ident_list_names)
                .unwrap_or_default()
                .into_iter()
                .collect(),
            None => HashSet::new(),
        };
        for p in &param_names {
            if !value_names.contains(p) {
                return Err(CompileError::Unsupported(format!(
                    "call-by-name parameter {p:?}: only `value` parameters are supported"
                )));
            }
        }

        // Each parameter's type, gathered from the `spec_part` declarations.
        let mut type_of: HashMap<String, ScalarType> = HashMap::new();
        for spec in direct_nodes(proc_decl)
            .into_iter()
            .filter(|n| n.rule_name == "spec_part")
        {
            let specifier = first_direct_node(spec, "specifier")
                .ok_or_else(|| CompileError::Malformed("spec_part missing specifier".into()))?;
            let ty = self.specifier_scalar_type(specifier)?;
            let list = first_direct_node(spec, "ident_list")
                .ok_or_else(|| CompileError::Malformed("spec_part missing ident_list".into()))?;
            for n in ident_list_names(list) {
                type_of.insert(n, ty);
            }
        }

        let mut params = Vec::with_capacity(param_names.len());
        for p in param_names {
            let ty = type_of.get(&p).copied().ok_or_else(|| {
                CompileError::Malformed(format!("parameter {p:?} has no specification"))
            })?;
            params.push((p, ty));
        }

        Ok((name, params, ret))
    }

    /// Pre-pass: record a procedure's signature so call sites can resolve it
    /// before the body is lowered (forward references and recursion).
    fn register_proc_sig(&mut self, proc_decl: &GrammarASTNode) -> Result<(), CompileError> {
        let (name, params, ret) = self.procedure_parts(proc_decl)?;
        if self.proc_sigs.contains_key(&name) {
            return Err(CompileError::Type(format!(
                "duplicate declaration for procedure {name:?}"
            )));
        }
        self.proc_sigs.insert(
            name,
            ProcSig {
                params: params.into_iter().map(|(_, ty)| ty).collect(),
                ret,
            },
        );
        Ok(())
    }

    /// Lower a procedure body into its own `IIRFunction`.
    ///
    /// A procedure is a *fresh* compilation context: its instructions,
    /// source map, register set, labels, and scopes are entirely its own.
    /// We therefore swap those fields out for empty ones, lower the body, snap
    /// them back, and hand the collected instructions to a new function.  The
    /// monotonic `temp_counter` / `label_counter` are intentionally **not**
    /// reset, so generated names stay globally unique across functions.
    ///
    /// Parameter binding mirrors ALGOL's "the procedure name behaves like a
    /// local variable holding the result": each value parameter is declared in
    /// the procedure's root scope (slot == bare name == `IIRFunction` param),
    /// and so is the procedure's own name, which the body assigns to and we
    /// `ret` at the end.
    fn compile_procedure(
        &mut self,
        proc_decl: &GrammarASTNode,
    ) -> Result<IIRFunction, CompileError> {
        self.set_loc(proc_decl);
        let (name, params, ret) = self.procedure_parts(proc_decl)?;

        // ── swap in a fresh emission context ─────────────────────────────
        let saved_instrs = std::mem::take(&mut self.instrs);
        let saved_source_map = std::mem::take(&mut self.source_map);
        let saved_registers = std::mem::take(&mut self.register_names);
        let saved_defined = std::mem::take(&mut self.defined_labels);
        let saved_referenced = std::mem::take(&mut self.referenced_labels);
        let saved_switches = std::mem::take(&mut self.switches);
        let saved_scopes = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);

        // Bind value parameters and the result variable (the procedure name).
        let mut param_pairs: Vec<(String, String)> = Vec::with_capacity(params.len());
        for (pname, pty) in &params {
            let slot = self.declare_var(pname, *pty)?;
            param_pairs.push((slot, pty.iir().to_string()));
        }
        // The procedure's name is an in-scope variable holding the return
        // value; seed it with a default so a path that never assigns it still
        // returns a defined value.
        let result_slot = self.declare_var(&name, ret)?;
        self.emit(IIRInstr::new(
            "const",
            Some(result_slot.clone()),
            vec![ret.default_operand()],
            ret.iir(),
        ));

        // ── lower the body ───────────────────────────────────────────────
        let body = first_direct_node(proc_decl, "proc_body")
            .ok_or_else(|| CompileError::Malformed("procedure_decl missing proc_body".into()))?;
        let inner = direct_nodes(body)
            .first()
            .copied()
            .ok_or_else(|| CompileError::Malformed("proc_body is empty".into()))?;
        match inner.rule_name.as_str() {
            "block" => self.emit_block(inner, false)?,
            "statement" => self.emit_statement(inner)?,
            other => {
                return Err(CompileError::Malformed(format!(
                    "unexpected proc_body child {other:?}"
                )))
            }
        }

        // Labels do not cross the procedure boundary, so every `goto` target
        // referenced inside the body must be defined inside the body.
        for label in &self.referenced_labels {
            if !self.defined_labels.contains(label) {
                return Err(CompileError::Malformed(format!(
                    "goto references undefined label {label:?} in procedure {name:?}"
                )));
            }
        }

        self.emit(IIRInstr::new(
            "ret",
            None,
            vec![Operand::Var(result_slot)],
            ret.iir(),
        ));

        // ── assemble the function and restore the caller's context ───────
        let body_instrs = std::mem::take(&mut self.instrs);
        let body_len = body_instrs.len();
        let mut func = IIRFunction::new(name, param_pairs, ret.iir(), body_instrs);
        func.type_status = FunctionTypeStatus::FullyTyped;
        func.register_count = self.register_names.len().saturating_add(8).max(8);
        let mut sm = std::mem::take(&mut self.source_map);
        while sm.len() < body_len {
            sm.push(SourceLoc::SYNTHETIC);
        }
        sm.truncate(body_len);
        func.source_map = sm;

        self.instrs = saved_instrs;
        self.source_map = saved_source_map;
        self.register_names = saved_registers;
        self.defined_labels = saved_defined;
        self.referenced_labels = saved_referenced;
        self.switches = saved_switches;
        self.scopes = saved_scopes;

        Ok(func)
    }

    /// Lower a procedure *call* in value position (`sq(7)`), returning the
    /// slot that holds the result.  Used from `emit_expr`.
    fn emit_proc_call(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        self.set_loc(node);
        let dest = self.emit_call_common(node)?;
        Ok(dest)
    }

    /// Lower a procedure *call* in statement position (`bump(3)`).  The
    /// returned value is computed but discarded.
    fn emit_proc_stmt(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        self.emit_call_common(node)?;
        Ok(())
    }

    /// Shared call-lowering for `proc_call` and `proc_stmt`: resolve the
    /// signature, evaluate and type-check the actuals, then emit a `call`
    /// whose `srcs[0]` names the callee and whose remaining `srcs` are the
    /// argument slots, matching the IIR calling convention every backend
    /// understands.
    fn emit_call_common(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        let name = direct_tokens(node)
            .into_iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CompileError::Malformed("call has no procedure name".into()))?;

        let sig = self
            .proc_sigs
            .get(&name)
            .cloned()
            .ok_or_else(|| CompileError::Type(format!("call to undeclared procedure {name:?}")))?;

        let actuals: Vec<&GrammarASTNode> = match first_direct_node(node, "actual_params") {
            Some(ap) => direct_nodes(ap)
                .into_iter()
                .filter(|n| n.rule_name == "expression")
                .collect(),
            None => Vec::new(),
        };
        if actuals.len() != sig.params.len() {
            return Err(CompileError::Type(format!(
                "procedure {name:?} expects {} argument(s), got {}",
                sig.params.len(),
                actuals.len()
            )));
        }

        let mut arg_slots = Vec::with_capacity(actuals.len());
        for (actual, expected) in actuals.iter().zip(sig.params.iter()) {
            let value = self.emit_expr(actual)?;
            if value.ty != *expected {
                return Err(CompileError::Type(format!(
                    "procedure {name:?}: argument is {} but parameter is {}",
                    value.ty.name(),
                    expected.name()
                )));
            }
            arg_slots.push(value.slot);
        }

        let dest = self.fresh_temp();
        let mut srcs = Vec::with_capacity(arg_slots.len() + 1);
        srcs.push(Operand::Var(name));
        srcs.extend(arg_slots.into_iter().map(Operand::Var));
        self.emit(IIRInstr::new(
            "call",
            Some(dest.clone()),
            srcs,
            sig.ret.iir(),
        ));
        Ok(ExprValue {
            slot: dest,
            ty: sig.ret,
        })
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
            "proc_stmt" => self.emit_proc_stmt(child),
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
            let binding = self.require_var(&name)?;
            if binding.ty != rhs.ty {
                return Err(CompileError::Type(format!(
                    "cannot assign {} expression to {} variable {name:?}",
                    rhs.ty.name(),
                    binding.ty.name()
                )));
            }
            self.emit(IIRInstr::new(
                "mov",
                Some(binding.slot),
                vec![Operand::Var(rhs.slot.clone())],
                binding.ty.iir(),
            ));
        }
        Ok(())
    }

    fn emit_goto(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        let desig = first_direct_node(node, "desig_expr")
            .ok_or_else(|| CompileError::Malformed("goto_stmt has no desig_expr".into()))?;
        self.emit_desig_jump(desig)
    }

    /// Emit the jump(s) that transfer control to a designational expression.
    ///
    /// A designational expression resolves to a label at run time and can be:
    ///
    /// * a plain `label` — a single `jmp`;
    /// * a switch subscript `s[i]` — a 1-based selection among the switch's
    ///   target labels (out-of-range falls through, per the ALGOL report's
    ///   "undefined" rule, which real implementations treat as a no-op);
    /// * a conditional `if b then d1 else d2` — branch on `b`, then jump to
    ///   whichever sub-designator is selected.
    ///
    /// All control flow uses only the portable `jmp` / `jmp_if_false` / `label`
    /// subset (the CLR textual `.il` path has no `jmp_if_true`), so a computed
    /// goto lowers to a chain every backend already runs.
    fn emit_desig_jump(&mut self, desig: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(desig);
        // desig_expr = "if" bool_expr "then" simple_desig "else" desig_expr
        //            | simple_desig
        if direct_tokens(desig).iter().any(|t| t.value == "if") {
            let cond_node = first_direct_node(desig, "bool_expr").ok_or_else(|| {
                CompileError::Malformed("conditional designator missing condition".into())
            })?;
            let then_node = first_direct_node(desig, "simple_desig").ok_or_else(|| {
                CompileError::Malformed("conditional designator missing then target".into())
            })?;
            let else_node = direct_nodes(desig)
                .into_iter()
                .find(|n| n.rule_name == "desig_expr")
                .ok_or_else(|| {
                    CompileError::Malformed("conditional designator missing else target".into())
                })?;
            let cond = self.emit_expr(cond_node)?;
            if cond.ty != ScalarType::Boolean {
                return Err(CompileError::Type(
                    "designator condition must be boolean".into(),
                ));
            }
            let else_label = self.fresh_label("desig_else");
            self.emit(IIRInstr::new(
                "jmp_if_false",
                None,
                vec![Operand::Var(cond.slot), Operand::Var(else_label.clone())],
                "void",
            ));
            self.emit_simple_desig_jump(then_node)?;
            self.emit_label(&else_label);
            self.emit_desig_jump(else_node)
        } else {
            let simple = first_direct_node(desig, "simple_desig").ok_or_else(|| {
                CompileError::Malformed("designator missing simple_desig".into())
            })?;
            self.emit_simple_desig_jump(simple)
        }
    }

    /// Emit the jump(s) for a `simple_desig`: a switch subscript, a
    /// parenthesised designator, or a plain label.
    fn emit_simple_desig_jump(
        &mut self,
        simple: &GrammarASTNode,
    ) -> Result<(), CompileError> {
        let tokens = direct_tokens(simple);
        // simple_desig = NAME LBRACKET arith_expr RBRACKET   (switch subscript)
        if tokens.iter().any(|t| t.effective_type_name() == "LBRACKET") {
            let name = tokens
                .iter()
                .find(|t| t.effective_type_name() == "NAME")
                .map(|t| t.value.clone())
                .ok_or_else(|| CompileError::Malformed("switch subscript missing name".into()))?;
            let index_node = first_direct_node(simple, "arith_expr").ok_or_else(|| {
                CompileError::Malformed("switch subscript missing index".into())
            })?;
            let labels = self.switches.get(&name).cloned().ok_or_else(|| {
                CompileError::Type(format!("goto uses undeclared switch {name:?}"))
            })?;
            let index = self.emit_expr(index_node)?;
            if index.ty != ScalarType::Integer {
                return Err(CompileError::Type(
                    "switch subscript index must be an integer".into(),
                ));
            }
            // 1-based: `goto s[k]` jumps to the k-th target.  Emit a linear
            // `index == k ? jmp Lk` chain; an out-of-range index matches no arm
            // and falls through.
            for (i, label) in labels.iter().enumerate() {
                let k = ExprValue {
                    slot: self.emit_const(ScalarType::Integer, Operand::Int((i as i64) + 1)),
                    ty: ScalarType::Integer,
                };
                let matched = self.emit_binary("=", index.clone(), k)?;
                let next_label = self.fresh_label("switch_next");
                self.emit(IIRInstr::new(
                    "jmp_if_false",
                    None,
                    vec![Operand::Var(matched.slot), Operand::Var(next_label.clone())],
                    "void",
                ));
                self.referenced_labels.insert(label.clone());
                self.emit(IIRInstr::new(
                    "jmp",
                    None,
                    vec![Operand::Var(label.clone())],
                    "void",
                ));
                self.emit_label(&next_label);
            }
            Ok(())
        } else if tokens.iter().any(|t| t.effective_type_name() == "LPAREN") {
            // simple_desig = LPAREN desig_expr RPAREN
            let inner = first_direct_node(simple, "desig_expr").ok_or_else(|| {
                CompileError::Malformed("parenthesised designator missing inner".into())
            })?;
            self.emit_desig_jump(inner)
        } else {
            // simple_desig = label
            let label_node = first_direct_node(simple, "label")
                .ok_or_else(|| CompileError::Malformed("designator missing label".into()))?;
            let label = self.label_name(label_node)?;
            self.referenced_labels.insert(label.clone());
            self.emit(IIRInstr::new("jmp", None, vec![Operand::Var(label)], "void"));
            Ok(())
        }
    }

    /// Record a switch declaration's ordered target labels.
    ///
    /// `switch_decl = "switch" NAME ASSIGN switch_list` and
    /// `switch_list = desig_expr { COMMA desig_expr }`.  On the executable
    /// slice each element must be a plain label (the overwhelmingly common
    /// form); conditional or nested-subscript switch elements are rejected.
    fn register_switch(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        let name = direct_tokens(node)
            .into_iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CompileError::Malformed("switch_decl missing name".into()))?;
        let switch_list = first_direct_node(node, "switch_list")
            .ok_or_else(|| CompileError::Malformed("switch_decl missing switch_list".into()))?;

        let mut labels = Vec::new();
        for elem in direct_nodes(switch_list)
            .into_iter()
            .filter(|n| n.rule_name == "desig_expr")
        {
            labels.push(self.switch_element_label(elem)?);
        }
        if labels.is_empty() {
            return Err(CompileError::Malformed("switch has no targets".into()));
        }
        if self.switches.contains_key(&name) {
            return Err(CompileError::Type(format!(
                "duplicate declaration for switch {name:?}"
            )));
        }
        self.switches.insert(name, labels);
        Ok(())
    }

    /// Resolve a switch-list element to a single target label slot.  Only plain
    /// labels are supported as switch elements on the current slice.
    fn switch_element_label(&self, desig: &GrammarASTNode) -> Result<String, CompileError> {
        let toks = recursive_tokens(desig);
        if toks.iter().any(|t| t.value == "if") {
            return Err(CompileError::Unsupported(
                "conditional switch-list elements".into(),
            ));
        }
        if toks
            .iter()
            .any(|t| matches!(t.effective_type_name(), "LBRACKET" | "RBRACKET"))
        {
            return Err(CompileError::Unsupported(
                "nested switch-list elements".into(),
            ));
        }
        let names: Vec<&Token> = toks
            .into_iter()
            .filter(|t| matches!(t.effective_type_name(), "NAME" | "INTEGER_LIT"))
            .collect();
        if names.len() == 1 {
            Ok(format!("L_{}", names[0].value))
        } else {
            Err(CompileError::Malformed(format!(
                "switch element should be one label, got {} tokens",
                names.len()
            )))
        }
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
        let var_binding = self.require_var(&var_name)?;
        if var_binding.ty != ScalarType::Integer {
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
        if elems.is_empty() {
            return Err(CompileError::Malformed("for_list has no elements".into()));
        }
        let body = direct_nodes(node)
            .into_iter()
            .find(|n| n.rule_name == "statement")
            .ok_or_else(|| CompileError::Malformed("for_stmt missing body statement".into()))?;

        for elem in elems {
            self.emit_for_element(&var_binding.slot, elem, body)?;
        }
        Ok(())
    }

    fn emit_for_element(
        &mut self,
        var_name: &str,
        elem: &GrammarASTNode,
        body: &GrammarASTNode,
    ) -> Result<(), CompileError> {
        if direct_tokens(elem).iter().any(|t| t.value == "while") {
            return self.emit_for_while(var_name, elem, body);
        }
        if direct_tokens(elem).iter().any(|t| t.value == "step") {
            return self.emit_for_step_until(var_name, elem, body);
        }
        self.emit_for_once(var_name, elem, body)
    }

    fn emit_for_once(
        &mut self,
        var_name: &str,
        elem: &GrammarASTNode,
        body: &GrammarASTNode,
    ) -> Result<(), CompileError> {
        let arith_nodes: Vec<&GrammarASTNode> = direct_nodes(elem)
            .into_iter()
            .filter(|n| n.rule_name == "arith_expr")
            .collect();
        if arith_nodes.len() != 1 {
            return Err(CompileError::Malformed(
                "single-value for element should have one value".into(),
            ));
        }

        let value = self.emit_expr(arith_nodes[0])?;
        if value.ty != ScalarType::Integer {
            return Err(CompileError::Type(
                "single-value for element must be integer".into(),
            ));
        }
        self.emit(IIRInstr::new(
            "mov",
            Some(var_name.to_string()),
            vec![Operand::Var(value.slot)],
            "i64",
        ));
        self.emit_statement(body)
    }

    fn emit_for_step_until(
        &mut self,
        var_name: &str,
        elem: &GrammarASTNode,
        body: &GrammarASTNode,
    ) -> Result<(), CompileError> {
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
        self.emit(IIRInstr::new(
            "mov",
            Some(var_name.to_string()),
            vec![Operand::Var(start.slot)],
            "i64",
        ));

        let zero = self.emit_const(ScalarType::Integer, Operand::Int(0));
        let loop_label = self.fresh_label("for_loop");
        let negative_check_label = self.fresh_label("for_negative_check");
        let body_label = self.fresh_label("for_body");
        let end_label = self.fresh_label("for_end");
        self.emit_label(&loop_label);

        let step_non_negative = self.fresh_temp();
        self.emit(IIRInstr::new(
            "cmp_ge",
            Some(step_non_negative.clone()),
            vec![Operand::Var(step.slot.clone()), Operand::Var(zero)],
            "bool",
        ));
        self.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![
                Operand::Var(step_non_negative),
                Operand::Var(negative_check_label.clone()),
            ],
            "void",
        ));

        let positive_cond = self.fresh_temp();
        self.emit(IIRInstr::new(
            "cmp_le",
            Some(positive_cond.clone()),
            vec![
                Operand::Var(var_name.to_string()),
                Operand::Var(limit.slot.clone()),
            ],
            "bool",
        ));
        self.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(positive_cond), Operand::Var(end_label.clone())],
            "void",
        ));
        self.emit(IIRInstr::new(
            "jmp",
            None,
            vec![Operand::Var(body_label.clone())],
            "void",
        ));

        self.emit_label(&negative_check_label);
        let negative_cond = self.fresh_temp();
        self.emit(IIRInstr::new(
            "cmp_ge",
            Some(negative_cond.clone()),
            vec![
                Operand::Var(var_name.to_string()),
                Operand::Var(limit.slot.clone()),
            ],
            "bool",
        ));
        self.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(negative_cond), Operand::Var(end_label.clone())],
            "void",
        ));

        self.emit_label(&body_label);
        self.emit_statement(body)?;
        let next = self.fresh_temp();
        self.emit(IIRInstr::new(
            "add",
            Some(next.clone()),
            vec![Operand::Var(var_name.to_string()), Operand::Var(step.slot)],
            "i64",
        ));
        self.emit(IIRInstr::new(
            "mov",
            Some(var_name.to_string()),
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

    fn emit_for_while(
        &mut self,
        var_name: &str,
        elem: &GrammarASTNode,
        body: &GrammarASTNode,
    ) -> Result<(), CompileError> {
        let arith_node = direct_nodes(elem)
            .into_iter()
            .find(|n| n.rule_name == "arith_expr")
            .ok_or_else(|| CompileError::Malformed("while for element missing value".into()))?;
        let cond_node = first_direct_node(elem, "bool_expr")
            .ok_or_else(|| CompileError::Malformed("while for element missing condition".into()))?;

        let loop_label = self.fresh_label("for_while_loop");
        let end_label = self.fresh_label("for_while_end");
        self.emit_label(&loop_label);

        let value = self.emit_expr(arith_node)?;
        if value.ty != ScalarType::Integer {
            return Err(CompileError::Type(
                "for while value expression must be integer".into(),
            ));
        }
        self.emit(IIRInstr::new(
            "mov",
            Some(var_name.to_string()),
            vec![Operand::Var(value.slot)],
            "i64",
        ));

        let cond = self.emit_expr(cond_node)?;
        if cond.ty != ScalarType::Boolean {
            return Err(CompileError::Type(
                "for while condition must be boolean".into(),
            ));
        }
        self.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(cond.slot), Operand::Var(end_label.clone())],
            "void",
        ));
        self.emit_statement(body)?;
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
            return self.emit_conditional_expr(node);
        }

        match node.rule_name.as_str() {
            "variable" => {
                let name = self.simple_variable_name(node)?;
                let binding = self.require_var(&name)?;
                Ok(ExprValue {
                    slot: binding.slot,
                    ty: binding.ty,
                })
            }
            "proc_call" => self.emit_proc_call(node),
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

    fn emit_conditional_expr(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        match node.rule_name.as_str() {
            "arith_expr" => {
                let cond_node = first_direct_node(node, "bool_expr").ok_or_else(|| {
                    CompileError::Malformed("arithmetic conditional missing condition".into())
                })?;
                let then_node = first_direct_node(node, "simple_arith").ok_or_else(|| {
                    CompileError::Malformed("arithmetic conditional missing then branch".into())
                })?;
                let else_node = direct_nodes(node)
                    .into_iter()
                    .find(|n| n.rule_name == "arith_expr")
                    .ok_or_else(|| {
                        CompileError::Malformed(
                            "arithmetic conditional missing else branch".into(),
                        )
                    })?;
                self.emit_conditional_branches(cond_node, then_node, else_node)
            }
            "bool_expr" => {
                let bool_nodes: Vec<&GrammarASTNode> = direct_nodes(node)
                    .into_iter()
                    .filter(|n| n.rule_name == "bool_expr")
                    .collect();
                if bool_nodes.len() != 2 {
                    return Err(CompileError::Malformed(
                        "boolean conditional should have condition and else bool_expr".into(),
                    ));
                }
                let then_node = first_direct_node(node, "simple_bool").ok_or_else(|| {
                    CompileError::Malformed("boolean conditional missing then branch".into())
                })?;
                self.emit_conditional_branches(bool_nodes[0], then_node, bool_nodes[1])
            }
            other => Err(CompileError::Unsupported(format!(
                "conditional expressions in {other}"
            ))),
        }
    }

    fn emit_conditional_branches(
        &mut self,
        cond_node: &GrammarASTNode,
        then_node: &GrammarASTNode,
        else_node: &GrammarASTNode,
    ) -> Result<ExprValue, CompileError> {
        let cond = self.emit_expr(cond_node)?;
        if cond.ty != ScalarType::Boolean {
            return Err(CompileError::Type(
                "conditional expression condition must be boolean".into(),
            ));
        }

        let else_label = self.fresh_label("expr_else");
        let end_label = self.fresh_label("expr_end");
        let dest = self.fresh_temp();

        self.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(cond.slot), Operand::Var(else_label.clone())],
            "void",
        ));
        let then_value = self.emit_expr(then_node)?;
        self.emit(IIRInstr::new(
            "mov",
            Some(dest.clone()),
            vec![Operand::Var(then_value.slot)],
            then_value.ty.iir(),
        ));
        self.emit(IIRInstr::new(
            "jmp",
            None,
            vec![Operand::Var(end_label.clone())],
            "void",
        ));
        self.emit_label(&else_label);
        let else_value = self.emit_expr(else_node)?;
        if then_value.ty != else_value.ty {
            return Err(CompileError::Type(format!(
                "conditional expression branches have types {} and {}",
                then_value.ty.name(),
                else_value.ty.name()
            )));
        }
        self.emit(IIRInstr::new(
            "mov",
            Some(dest.clone()),
            vec![Operand::Var(else_value.slot)],
            else_value.ty.iir(),
        ));
        self.emit_label(&end_label);

        Ok(ExprValue {
            slot: dest,
            ty: then_value.ty,
        })
    }

    fn emit_bool_wrapper(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        if pieces(node)
            .iter()
            .any(|p| matches!(p, Piece::Op(op) if matches!(op.as_str(), "and" | "or" | "impl" | "eqv")))
        {
            return self.emit_binary_or_child(node, BinaryFamily::Boolean);
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
            self.emit_not_value(value)
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
            ("REAL_LIT", _) => {
                // `REAL_LIT` is digits with a decimal point and/or exponent
                // (`3.14`, `1.0E-3`, `100E2`).  Rust's `f64::from_str` accepts
                // exactly that surface syntax, so parse it directly into an
                // `Operand::Float` (LANG-FULL AL1 / E3).
                let value = token.value.parse::<f64>().map_err(|_| {
                    CompileError::Type(format!("malformed real literal {:?}", token.value))
                })?;
                let slot = self.emit_const(ScalarType::Real, Operand::Float(value));
                Ok(ExprValue {
                    slot,
                    ty: ScalarType::Real,
                })
            }
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
                let binding = self.require_var(&name)?;
                Ok(ExprValue {
                    slot: binding.slot,
                    ty: binding.ty,
                })
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

    /// Require both operands of a numeric operator to be the **same** numeric
    /// type (`integer`+`integer` or `real`+`real`) and return it. Rejects a
    /// boolean operand and an integer/real mix — v1 has no implicit coercion.
    fn same_numeric_type(
        &self,
        op: &str,
        lhs: &ExprValue,
        rhs: &ExprValue,
    ) -> Result<ScalarType, CompileError> {
        let numeric = |t: ScalarType| matches!(t, ScalarType::Integer | ScalarType::Real);
        if !numeric(lhs.ty) || !numeric(rhs.ty) {
            return Err(CompileError::Type(format!(
                "operator {op:?} requires numeric operands, got {} and {}",
                lhs.ty.name(),
                rhs.ty.name()
            )));
        }
        if lhs.ty != rhs.ty {
            return Err(CompileError::Type(format!(
                "operator {op:?} cannot mix {} and {} (no implicit integer→real \
                 coercion in this slice)",
                lhs.ty.name(),
                rhs.ty.name()
            )));
        }
        Ok(lhs.ty)
    }

    fn emit_binary(
        &mut self,
        op: &str,
        lhs: ExprValue,
        rhs: ExprValue,
    ) -> Result<ExprValue, CompileError> {
        match op {
            "+" | "-" | "*" => {
                // `+`/`-`/`*` work on **either** `integer` (i64) or `real` (f64)
                // operands, but not a mix — ALGOL's implicit integer→real
                // coercion needs an IIR int→f64 convert op the code-gen slice
                // doesn't carry yet, so v1 requires both operands the same
                // numeric type (a clean error otherwise). The IIR `type_hint`
                // carries the operand width, so the backends pick `add`/`fadd`
                // etc. from it.
                let ty = self.same_numeric_type(op, &lhs, &rhs)?;
                let iir_op = match op {
                    "+" => "add",
                    "-" => "sub",
                    "*" => "mul",
                    _ => unreachable!(),
                };
                let dest = self.fresh_temp();
                self.emit(IIRInstr::new(
                    iir_op,
                    Some(dest.clone()),
                    vec![Operand::Var(lhs.slot), Operand::Var(rhs.slot)],
                    ty.iir(),
                ));
                Ok(ExprValue { slot: dest, ty })
            }
            "div" | "mod" => {
                // `div` (integer division) and `mod` are integer-only operators
                // in ALGOL 60 — reals use `/`.
                if lhs.ty != ScalarType::Integer || rhs.ty != ScalarType::Integer {
                    return Err(CompileError::Type(format!(
                        "operator {op:?} requires integer operands"
                    )));
                }
                let dest = self.fresh_temp();
                self.emit(IIRInstr::new(
                    op,
                    Some(dest.clone()),
                    vec![Operand::Var(lhs.slot), Operand::Var(rhs.slot)],
                    "i64",
                ));
                Ok(ExprValue {
                    slot: dest,
                    ty: ScalarType::Integer,
                })
            }
            "/" => {
                // Real division. ALGOL's `/` always yields a `real`; v1 requires
                // real operands (no integer→real coercion yet — see the `+`/`-`
                // note). Lowers to the IIR `div` with an `f64` hint, so the
                // backends emit `fdiv`/`f64.div`/`ddiv`. IEEE division by zero
                // is `±inf`, consistent across every backend (no trap).
                if lhs.ty != ScalarType::Real || rhs.ty != ScalarType::Real {
                    return Err(CompileError::Type(
                        "real division '/' requires real operands (integer→real \
                         coercion is not in this slice; use `div` for integers)"
                            .into(),
                    ));
                }
                let dest = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "div",
                    Some(dest.clone()),
                    vec![Operand::Var(lhs.slot), Operand::Var(rhs.slot)],
                    "f64",
                ));
                Ok(ExprValue {
                    slot: dest,
                    ty: ScalarType::Real,
                })
            }
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
                // The comparison's `type_hint` is the **operand** width, not the
                // `bool` result width.  Code-gen backends size the compare from
                // this hint: emitting `bool` made LLVM compare two `i64` operands
                // at 1-bit `i1` (`3 == 1` truncates both to `1` → wrongly equal),
                // and produced invalid IR clang rejects outright.  Comparing at
                // the operand width (`i64` for integers, `bool` for booleans) is
                // the same fix the BASIC BA0 work applied. The *result* is still
                // a boolean (`ExprValue.ty`).
                let dest = self.fresh_temp();
                self.emit(IIRInstr::new(
                    iir_op,
                    Some(dest.clone()),
                    vec![Operand::Var(lhs.slot), Operand::Var(rhs.slot)],
                    lhs.ty.iir(),
                ));
                Ok(ExprValue {
                    slot: dest,
                    ty: ScalarType::Boolean,
                })
            }
            "^" | "**" => Err(CompileError::Unsupported("exponentiation".into())),
            "and" | "or" => {
                self.ensure_boolean_operands(op, &lhs, &rhs)?;
                let dest = self.fresh_temp();
                self.emit(IIRInstr::new(
                    op,
                    Some(dest.clone()),
                    vec![Operand::Var(lhs.slot), Operand::Var(rhs.slot)],
                    "bool",
                ));
                Ok(ExprValue {
                    slot: dest,
                    ty: ScalarType::Boolean,
                })
            }
            "impl" => {
                self.ensure_boolean_operands(op, &lhs, &rhs)?;
                let not_lhs = self.emit_not_value(lhs)?;
                self.emit_binary("or", not_lhs, rhs)
            }
            "eqv" => {
                self.ensure_boolean_operands(op, &lhs, &rhs)?;
                let dest = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "cmp_eq",
                    Some(dest.clone()),
                    vec![Operand::Var(lhs.slot), Operand::Var(rhs.slot)],
                    "bool",
                ));
                Ok(ExprValue {
                    slot: dest,
                    ty: ScalarType::Boolean,
                })
            }
            other => Err(CompileError::Malformed(format!(
                "unknown operator {other:?}"
            ))),
        }
    }

    fn ensure_boolean_operands(
        &self,
        op: &str,
        lhs: &ExprValue,
        rhs: &ExprValue,
    ) -> Result<(), CompileError> {
        if lhs.ty != ScalarType::Boolean || rhs.ty != ScalarType::Boolean {
            return Err(CompileError::Type(format!(
                "operator {op:?} requires boolean operands"
            )));
        }
        Ok(())
    }

    fn emit_not_value(&mut self, value: ExprValue) -> Result<ExprValue, CompileError> {
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
    }

    fn emit_unary_minus(&mut self, value: ExprValue) -> Result<ExprValue, CompileError> {
        // Negation is `0 - x` at the operand's numeric width: `i64` for
        // `integer`, `f64` for `real` (the `0` const and the `sub` both carry
        // the type, so a real negation lowers to an `f64` subtract → `fsub`).
        let ty = match value.ty {
            ScalarType::Integer | ScalarType::Real => value.ty,
            ScalarType::Boolean => {
                return Err(CompileError::Type(
                    "unary minus requires a numeric operand".into(),
                ))
            }
        };
        let zero = self.emit_const(ty, ty.default_operand());
        let dest = self.fresh_temp();
        self.emit(IIRInstr::new(
            "sub",
            Some(dest.clone()),
            vec![Operand::Var(zero), Operand::Var(value.slot)],
            ty.iir(),
        ));
        Ok(ExprValue { slot: dest, ty })
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

    fn push_scope(&mut self) {
        self.scope_counter += 1;
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn declare_var(&mut self, name: &str, ty: ScalarType) -> Result<String, CompileError> {
        let slot = if self.scopes.len() == 1 {
            name.to_string()
        } else {
            format!("__algol_s{}_{}", self.scope_counter, name)
        };
        let current = self
            .scopes
            .last_mut()
            .expect("compiler always keeps a root scope");
        if current.contains_key(name) {
            return Err(CompileError::Type(format!(
                "duplicate declaration for {name:?}"
            )));
        }
        current.insert(
            name.to_string(),
            VarBinding {
                slot: slot.clone(),
                ty,
            },
        );
        self.register_names.insert(slot.clone());
        Ok(slot)
    }

    fn require_var(&self, name: &str) -> Result<VarBinding, CompileError> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .cloned()
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

}

#[derive(Debug, Clone, Copy)]
enum BinaryFamily {
    Additive,
    Multiplicative,
    Comparison,
    Boolean,
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

/// Collect the `NAME` tokens of an `ident_list` (`NAME { COMMA NAME }`) in
/// order — used to read a procedure's formal parameters, `value` list, and
/// `spec_part` identifier groups.
fn ident_list_names(node: &GrammarASTNode) -> Vec<String> {
    direct_tokens(node)
        .into_iter()
        .filter(|t| t.effective_type_name() == "NAME")
        .map(|t| t.value.clone())
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn run_i64(source: &str) -> i64 {
        let result = execute_source(source, "test")
            .expect("ALGOL source should compile and run")
            .expect("main should return a value");
        result.as_i64().expect("result should be an integer")
    }

    fn run_f64(source: &str) -> f64 {
        let result = execute_source(source, "test")
            .expect("ALGOL source should compile and run")
            .expect("main should return a value");
        result.as_f64().expect("result should be a real")
    }

    #[test]
    fn compiles_and_runs_integer_assignment() {
        let src = "begin integer result; result := 40 + 2 end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn compiles_and_runs_mod_expression() {
        let src = "begin integer result; result := 17 mod 5 end";
        assert_eq!(run_i64(src), 2);
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
    fn compiles_and_runs_dynamic_for_step_until() {
        let src = "begin integer i, stepvalue, result; result := 0; stepvalue := 2; for i := 1 step stepvalue until 5 do result := result + i; stepvalue := 0 - stepvalue; for i := 5 step stepvalue until 1 do result := result + i end";
        assert_eq!(run_i64(src), 18);
    }

    #[test]
    fn compiles_and_runs_for_while_sum() {
        let src = "begin integer x, result; x := 6; result := 0; for x := x - 1 while x > 0 do result := result + x end";
        assert_eq!(run_i64(src), 15);
    }

    #[test]
    fn compiles_and_runs_single_value_for_element() {
        let src = "begin integer i, result; for i := 2 do result := 40 + i end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn compiles_and_runs_multi_element_for_list() {
        let src = "begin integer i, result; i := 0; result := 0; for i := 1 step 1 until 3, 10, i + 1 while i < 13 do result := result + i end";
        assert_eq!(run_i64(src), 39);
    }

    #[test]
    fn compiles_and_runs_arithmetic_conditional_expression() {
        let src = "begin boolean flag; integer i, result; flag := true; result := 0; for i := if flag then 1 else 4 step 1 until if flag then 3 else 4 do result := result + i end";
        assert_eq!(run_i64(src), 6);
    }

    #[test]
    fn compiles_and_runs_boolean_conditional_expression() {
        let src = "begin boolean flag; integer result; flag := true; if if flag then true else false then result := 42 else result := 1 end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn compiles_and_runs_nested_block_shadowing() {
        let src = "begin integer x, result; boolean flag; x := 1; flag := true; result := 0; begin integer x; boolean flag; x := 10; flag := false; begin integer x; x := 31; if not flag then result := x else result := 1 end; result := result + x end; if flag then result := result + x else result := 0 end";
        assert_eq!(run_i64(src), 42);
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
    fn boolean_operators_run() {
        let src = "begin boolean a, b; integer result; a := true; b := false; if (a and not b) and ((b impl a) eqv (a or b)) then result := 42 else result := 1 end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn real_declarations_compile_to_f64() {
        // (Was `rejects_real_declarations_cleanly` — reals are now supported,
        // LANG-FULL AL1 / E3.) A `real x` declares an `f64` slot seeded to 0.0.
        let module = compile_source("begin real x; x := 1.5 end", "ok")
            .expect("real declarations now compile");
        let main = module.get_function("main").expect("main exists");
        assert!(main.instructions.iter().any(|i|
            i.op == "const"
                && i.dest.as_deref() == Some("x")
                && i.type_hint == "f64"),
            "real `x` should get an f64 const slot");
    }

    #[test]
    fn source_map_tracks_every_instruction() {
        let module = compile_source("begin integer result; result := 42 end", "map")
            .expect("source should compile");
        let main = module.get_function("main").expect("main exists");
        assert_eq!(main.instructions.len(), main.source_map.len());
        assert_eq!(main.type_status, FunctionTypeStatus::FullyTyped);
    }

    // ---- AL3: typed procedures with value parameters ----

    #[test]
    fn compiles_and_runs_value_procedure() {
        let src = "begin integer result; integer procedure sq(x); value x; integer x; \
                   sq := x * x; result := sq(7) end";
        assert_eq!(run_i64(src), 49);
    }

    #[test]
    fn procedure_takes_multiple_value_parameters() {
        let src = "begin integer result; integer procedure add(a, b); value a, b; integer a, b; \
                   add := a + b; result := add(20, 22) end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn recursive_procedure_runs() {
        // Factorial uses an if-*statement* body so the recursion's base case is
        // a `cond_stmt`, and recurses through a `proc_call` in the else branch.
        let src = "begin integer result; integer procedure fact(n); value n; integer n; \
                   if n < 2 then fact := 1 else fact := n * fact(n - 1); result := fact(5) end";
        assert_eq!(run_i64(src), 120);
    }

    #[test]
    fn boolean_value_procedure_runs() {
        let src = "begin boolean b; integer result; boolean procedure neg(p); value p; boolean p; \
                   neg := not p; b := neg(false); if b then result := 42 else result := 1 end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn procedure_call_as_statement_runs() {
        // A typed procedure invoked in statement position: the result is
        // discarded, but the call still executes (and the assignment of `m`
        // observes the surrounding program reached the statement).
        let src = "begin integer result; integer procedure id(x); value x; integer x; id := x; \
                   id(99); result := 42 end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn procedure_emitted_as_sibling_function() {
        let module = compile_source(
            "begin integer result; integer procedure sq(x); value x; integer x; \
             sq := x * x; result := sq(3) end",
            "proc",
        )
        .expect("procedure should compile");
        let sq = module.get_function("sq").expect("sq is a sibling function");
        assert_eq!(sq.params, vec![("x".to_string(), "i64".to_string())]);
        assert_eq!(sq.return_type, "i64");
        assert_eq!(sq.type_status, FunctionTypeStatus::FullyTyped);
        assert_eq!(sq.instructions.len(), sq.source_map.len());
        // The call site in `main` names `sq` as srcs[0].
        let main = module.get_function("main").expect("main exists");
        let call = main
            .instructions
            .iter()
            .find(|i| i.op == "call")
            .expect("main calls sq");
        assert!(matches!(call.srcs.first(), Some(Operand::Var(s)) if s == "sq"));
    }

    #[test]
    fn rejects_void_procedure_cleanly() {
        let err = compile_source(
            "begin integer result; procedure noop; result := 1; result := 42 end",
            "bad",
        )
        .expect_err("proper (void) procedures are outside this slice");
        assert!(err.to_string().contains("void"));
    }

    #[test]
    fn rejects_call_by_name_parameter() {
        let err = compile_source(
            "begin integer result; integer procedure f(x); integer x; f := x; result := f(1) end",
            "bad",
        )
        .expect_err("call-by-name parameters are unsupported");
        assert!(err.to_string().contains("value"));
    }

    #[test]
    fn rejects_argument_count_mismatch() {
        let err = compile_source(
            "begin integer result; integer procedure sq(x); value x; integer x; sq := x * x; \
             result := sq(1, 2) end",
            "bad",
        )
        .expect_err("arity mismatch");
        assert!(err.to_string().contains("argument"));
    }

    #[test]
    fn rejects_argument_type_mismatch() {
        let err = compile_source(
            "begin integer result; integer procedure sq(x); value x; integer x; sq := x * x; \
             result := sq(true) end",
            "bad",
        )
        .expect_err("argument type mismatch");
        assert!(err.to_string().to_lowercase().contains("boolean"));
    }

    // ---- AL5: switches + conditional designational expressions ----

    fn switch_prog(index: i64) -> String {
        format!(
            "begin integer result; switch s := a1, a2, a3; integer i; i := {index}; \
             goto s[i]; a1: result := 1; goto done; a2: result := 2; goto done; \
             a3: result := 3; done: end"
        )
    }

    #[test]
    fn switch_selects_first_target() {
        assert_eq!(run_i64(&switch_prog(1)), 1);
    }

    #[test]
    fn switch_selects_middle_target() {
        assert_eq!(run_i64(&switch_prog(2)), 2);
    }

    #[test]
    fn switch_selects_last_target() {
        // s[3] is the case that an i1-truncated compare would mis-select, so it
        // also guards the cmp operand-width fix.
        assert_eq!(run_i64(&switch_prog(3)), 3);
    }

    #[test]
    fn switch_out_of_range_falls_through() {
        // An out-of-range subscript matches no arm and continues to the next
        // statement (ALGOL leaves this undefined; we treat it as a no-op).
        let src = "begin integer result; switch s := a1, a2; integer i; result := 7; i := 9; \
                   goto s[i]; result := 42; a1: ; a2: end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn conditional_designator_picks_then() {
        let src = "begin integer result; boolean b; b := true; goto if b then yes else no; \
                   yes: result := 42; goto fin; no: result := 1; fin: end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn conditional_designator_picks_else() {
        let src = "begin integer result; boolean b; b := false; goto if b then yes else no; \
                   yes: result := 1; goto fin; no: result := 42; fin: end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn rejects_goto_undeclared_switch() {
        let err = compile_source(
            "begin integer result; integer i; i := 1; goto s[i] end",
            "bad",
        )
        .expect_err("switch s is undeclared");
        assert!(err.to_string().contains("switch"));
    }

    #[test]
    fn rejects_non_integer_switch_index() {
        let err = compile_source(
            "begin integer result; boolean b; switch s := a1; b := true; goto s[b]; a1: end",
            "bad",
        )
        .expect_err("switch index must be integer");
        assert!(err.to_string().contains("integer"));
    }

    #[test]
    fn comparison_uses_operand_width_not_bool() {
        // Regression for the cmp width: an integer comparison must lower to a
        // `cmp_*` whose type_hint is the i64 operand width, not the bool result
        // (emitting `bool` made LLVM truncate to a 1-bit compare).
        let module = compile_source(
            "begin integer x, result; x := 3; if x = 3 then result := 1 else result := 2 end",
            "cmp",
        )
        .expect("compiles");
        let main = module.get_function("main").expect("main exists");
        let cmp = main
            .instructions
            .iter()
            .find(|i| i.op == "cmp_eq")
            .expect("emits cmp_eq");
        assert_eq!(cmp.type_hint, "i64", "cmp must carry the i64 operand width");
    }

    // ── real (f64) arithmetic — LANG-FULL AL1 / enabler E3 ───────────

    #[test]
    fn compiles_and_runs_real_multiplication() {
        // `2.5 * 4.0` computes in f64 → 10.0 (an integer `*` would never produce
        // a fraction; this proves the real track is taken).
        assert_eq!(run_f64("begin real result; result := 2.5 * 4.0 end"), 10.0);
    }

    #[test]
    fn compiles_and_runs_real_division() {
        // `/` is true real division: 7.0 / 2.0 = 3.5 (integer `div` would give 3).
        assert_eq!(run_f64("begin real result; result := 7.0 / 2.0 end"), 3.5);
    }

    #[test]
    fn compiles_and_runs_real_add_sub() {
        assert_eq!(run_f64("begin real result; result := 1.5 + 0.25 - 0.75 end"), 1.0);
    }

    #[test]
    fn compiles_and_runs_real_unary_minus() {
        assert_eq!(run_f64("begin real result; result := - 2.5 + 4.0 end"), 1.5);
    }

    #[test]
    fn compiles_and_runs_real_comparison_fold() {
        // The cross-backend matrix-proof shape: do real arithmetic, then fold a
        // real comparison to an integer exit code (no float printing needed).
        let src = "begin real r; integer result; r := 2.5 * 2.0; \
                   if r = 5.0 then result := 42 else result := 0 end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn compiles_and_runs_real_ordered_comparison() {
        let src = "begin real r; integer result; r := 7.0 / 2.0; \
                   if r < 4.0 then result := 1 else result := 0 end";
        assert_eq!(run_i64(src), 1);
    }

    #[test]
    fn real_lowers_to_f64_typed_ops() {
        let module = compile_source(
            "begin real result; result := 2.5 * 4.0 end", "test")
            .expect("compiles");
        let main = &module.functions[0];
        let mul = main.instructions.iter()
            .find(|i| i.op == "mul")
            .expect("emits mul");
        assert_eq!(mul.type_hint, "f64", "real multiply must carry the f64 hint");
        assert!(main.instructions.iter().any(|i|
            i.op == "const" && matches!(i.srcs.first(), Some(Operand::Float(_)))),
            "a real literal lowers to an Operand::Float const");
    }

    #[test]
    fn rejects_mixed_integer_and_real() {
        // No implicit integer→real coercion in this slice.
        let err = compile_source(
            "begin real result; result := 1 + 2.5 end", "test").unwrap_err();
        assert!(matches!(err, CompileError::Type(_)),
            "mixing integer and real should be a Type error, got {err:?}");
    }

    #[test]
    fn rejects_real_division_on_integers() {
        let err = compile_source(
            "begin integer result; result := 7 / 2 end", "test").unwrap_err();
        assert!(matches!(err, CompileError::Type(_)),
            "`/` on integers should be a Type error (use div), got {err:?}");
    }
}
