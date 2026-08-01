//! ALGOL 60 scalar frontend for the LANG VM Rust chain.
//!
//! This crate lowers parsed ALGOL 60 into [`interpreter_ir::IIRModule`], the
//! shared IR consumed by `vm-core`, `jit-core`, `aot-core`, and the direct IIR
//! backends for WASM, JVM, CLR, BEAM, and LLVM.
//!
//! The first slice was intentionally conservative; the supported surface has
//! grown to scalar `integer`/`real`/`boolean` programs, arrays, procedures,
//! switches, `own` variables, standard numeric functions, and literal string
//! output. Features that still need a richer runtime model fail with explicit
//! errors instead of silently producing partial IR.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use coding_adventures_algol_parser::parse_algol;
use interpreter_ir::opcodes::make_array_type;
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
    /// LANG-FULL E4 scalar string. Local slots, captured block scalars, and
    /// `own` statics all hold runtime string handles.
    String,
}

impl ScalarType {
    fn iir(self) -> &'static str {
        match self {
            Self::Integer => "i64",
            Self::Real => "f64",
            Self::Boolean => "bool",
            Self::String => "str",
        }
    }

    fn default_operand(self) -> Operand {
        match self {
            Self::Integer => Operand::Int(0),
            Self::Real => Operand::Float(0.0),
            Self::Boolean => Operand::Bool(false),
            Self::String => Operand::Str(String::new()),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Real => "real",
            Self::Boolean => "boolean",
            Self::String => "string",
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
    /// `Some` when this name is an **array** (LANG-FULL enabler E5).  The
    /// binding's `slot` then holds the array *handle* (the value `alloc_array`
    /// produced), `ty` is the **element** type, and `array` carries the extra
    /// state a subscript access needs.  `None` for an ordinary scalar.
    array: Option<ArrayInfo>,
    /// `true` when this block scalar is accessed from inside a **procedure**
    /// body (LANG-FULL enabler **E6**).  Such a variable outlives any single
    /// call frame and is shared across functions, so it is materialised as a
    /// module **global** rather than a register: every read lowers to
    /// `global_load "<slot>"` and every write to `global_store "<slot>"` (the
    /// `slot` doubles as the global's name).  A plain scalar stays `false`.
    is_global: bool,
}

/// One dimension of a (possibly multidimensional) ALGOL array.
///
/// The flat 0-based contribution of dimension `d` to the linear index is
/// `(subscript[d] - lower[d]) * stride[d]`.  For the **last** dimension the
/// stride is always 1, so we skip the multiply and represent it as
/// `stride_slot: None`.  For earlier dimensions the stride equals the product
/// of all later dimension sizes, and its run-time value lives in `stride_slot`.
///
/// Row-major layout: dimension 0 is the outermost (slowest-varying).
/// For a 2-D array `A[lo1:hi1, lo2:hi2]`:
///   stride[0] = hi2 − lo2 + 1,  stride[1] = 1 (omitted)
///   flat_idx = (i − lo1) * stride[0] + (j − lo2)
#[derive(Debug, Clone, PartialEq, Eq)]
struct ArrayDim {
    /// Run-time slot holding the evaluated lower bound of this dimension.
    lower_slot: String,
    /// Run-time slot holding the stride, or `None` for the last/only dimension
    /// where the stride is 1 and the multiply is elided.
    stride_slot: Option<String>,
}

/// Per-array state recorded at its declaration, so a later `A[i]` (or
/// `A[i, j]`, etc.) access can be lowered.  The IIR array ops are **0-based**,
/// so subscripts are translated to a flat 0-based linear index.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ArrayInfo {
    /// One entry per declared dimension, in source order (outermost first).
    /// Always contains at least one entry.
    dims: Vec<ArrayDim>,
    /// The array's element type — `array_get` yields it, `array_set` checks it.
    elem_ty: ScalarType,
}

/// A captured array needs more than its handle in module-global storage: every
/// procedure that indexes it must also recover the declaration-time lower
/// bounds and row-major strides. These opaque names are never emitted as
/// backend identifiers, only as string keys for `global_load`/`global_store`.
fn array_dim_global_name(array_slot: &str, dim_index: usize, field: &str) -> String {
    format!("{array_slot}.__algol_array_dim_{dim_index}_{field}")
}

/// Hidden IIR parameter carrying an array formal's first lower bound. It keeps
/// the old 1-D spelling so existing IIR consumers and snapshots stay stable.
/// Compiler-generated slots cannot collide with source names because ALGOL
/// identifiers do not admit the double-underscore spelling.
fn array_param_lower_slot(name: &str) -> String {
    format!("__algol_array_param_{name}_lower")
}

/// Hidden IIR parameter carrying an array formal's lower bound for dimensions
/// after the first. Dimension zero deliberately uses [`array_param_lower_slot`]
/// for backward-compatible names.
fn array_param_dim_lower_slot(name: &str, dim_index: usize) -> String {
    if dim_index == 0 {
        array_param_lower_slot(name)
    } else {
        format!("__algol_array_param_{name}_lower_{dim_index}")
    }
}

/// Hidden IIR parameter carrying an array formal's row-major stride for one
/// non-final dimension. The final dimension has an implicit stride of one.
fn array_param_stride_slot(name: &str, dim_index: usize) -> String {
    format!("__algol_array_param_{name}_stride_{dim_index}")
}

/// Module-global backing name for an array formal captured by a nested
/// procedure. It is distinct from the incoming IIR parameter slot so the
/// outer procedure can copy the complete descriptor before the nested sibling
/// function runs.
fn array_param_capture_slot(procedure_name: &str, param_name: &str) -> String {
    format!("__algol_capture_{procedure_name}_{param_name}")
}

/// Module-global backing name for a scalar value parameter captured by a nested
/// procedure. The incoming IIR parameter remains the procedure ABI slot; the
/// outer procedure publishes its value here before the nested sibling runs.
fn scalar_param_capture_slot(procedure_name: &str, param_name: &str) -> String {
    format!("__algol_scalar_capture_{procedure_name}_{param_name}")
}

/// A procedure formal that the supported call-by-value slice can carry.
///
/// An array formal receives the caller's storage handle plus its complete
/// rank-specific descriptor: each dimension's lower bound and every non-final
/// row-major stride. The handle is shared storage, while this descriptor is
/// copied into the callee's frame. The rank is inferred from the formal's
/// subscripted uses in its body, so the fixed IIR function signature remains
/// statically known before either caller or callee is lowered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcedureParamType {
    Scalar(ScalarType),
    Array {
        elem_ty: ScalarType,
        dimensions: usize,
    },
}

/// A procedure heading read off the AST: `(name, value-params, return-type)`,
/// where each value parameter is `(name, type)` in declaration order. A missing
/// return type is an ALGOL proper procedure.
type ProcedureParts = (String, Vec<(String, ProcedureParamType)>, Option<ScalarType>);

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
/// We model typed procedures (ALGOL "function procedures") as value-producing
/// calls and proper procedures as void functions usable only in statement
/// position. Parameters are still restricted to `value` parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcSig {
    /// Source-level parameter types in declaration order. Array formals lower
    /// to a handle plus rank-specific descriptor values, but still consume one
    /// ALGOL actual at a call site.
    params: Vec<ProcedureParamType>,
    /// The procedure's return type, or `None` for a proper procedure.
    ret: Option<ScalarType>,
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
    /// Switch name → its ordered designational expressions. A `goto s[i]`
    /// evaluates the selected expression at run time, which permits both
    /// conditional and nested switch-list elements.
    switches: HashMap<String, Vec<GrammarASTNode>>,
    /// Switches being expanded into the current linear dispatch chain. The
    /// source grammar permits a switch to name another switch, but a cycle
    /// cannot be finitely inlined into portable IIR control flow.
    resolving_switches: HashSet<String>,
    /// Number of switch-designator arms expanded in the current function.
    /// This bounds exponential fan-out through an otherwise acyclic switch
    /// graph before it can exhaust compiler resources.
    switch_expansion_steps: usize,
    /// Names referenced inside a **procedure** body in the block currently being
    /// compiled (LANG-FULL **E6**).  Computed once per block before any scalar
    /// is declared; a block scalar whose name is in this set is materialised as
    /// a module global (`is_global`) so the procedure and the enclosing block
    /// share it.  Saved/restored around nested blocks.
    block_captured: HashSet<String>,
    /// String slots that hold a defined value in source order. Unlike
    /// a literal-only model, this also covers runtime results such as a string
    /// procedure call copied into a scalar local.
    initialized_string_slots: HashSet<String>,
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
            resolving_switches: HashSet::new(),
            switch_expansion_steps: 0,
            block_captured: HashSet::new(),
            initialized_string_slots: HashSet::new(),
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
            Some(binding) if binding.ty == ScalarType::String => {
                return Err(CompileError::Unsupported(
                    "string result variables as main return values".into(),
                ));
            }
            Some(binding) if binding.is_global => {
                let dest = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "global_load",
                    Some(dest.clone()),
                    vec![Operand::Str(binding.slot)],
                    binding.ty.iir(),
                ));
                (binding.ty.iir(), Operand::Var(dest))
            }
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

        // Pass 0.5 — E6 capture analysis.  Compute the set of names referenced
        // inside *any* procedure body of this block, so a block scalar with such
        // a name is declared as a module **global** (shared across functions)
        // rather than a register.  Done before any scalar is declared so the
        // `declare_var` below sees the right disposition, and before any
        // procedure body is lowered so the global binding exists for injection.
        let saved_captured = std::mem::take(&mut self.block_captured);
        self.block_captured = self.collect_block_captures(node);

        // Pass 1 — non-procedure declarations (scalars / arrays / switches).
        // Scalars marked captured become globals here.
        for child in direct_nodes(node) {
            if child.rule_name == "declaration"
                && first_direct_node(child, "procedure_decl").is_none()
            {
                self.emit_declaration(child)?;
            }
        }
        // Pass 2 — procedure declarations, lowered out of line.  Every global
        // they capture is now declared, so `compile_procedure` can inject it.
        for child in direct_nodes(node) {
            if child.rule_name == "declaration"
                && first_direct_node(child, "procedure_decl").is_some()
            {
                self.emit_declaration(child)?;
            }
        }
        // Pass 3 — statements.
        for child in direct_nodes(node) {
            if child.rule_name == "statement" {
                self.emit_statement(child)?;
            }
        }

        self.block_captured = saved_captured;
        if !is_root {
            self.pop_scope();
        }
        Ok(())
    }

    /// E6 capture analysis: collect names used from a procedure body that are
    /// not declared by that procedure or any intervening nested procedure. A
    /// block scalar whose name lands in this set is materialised as a global.
    fn collect_block_captures(&self, block: &GrammarASTNode) -> HashSet<String> {
        let mut captured = HashSet::new();
        for child in direct_nodes(block) {
            if child.rule_name != "declaration" {
                continue;
            }
            let Some(proc_decl) = first_direct_node(child, "procedure_decl") else {
                continue;
            };
            let hidden = procedure_local_names(proc_decl);
            let Some(body) = first_direct_node(proc_decl, "proc_body") else {
                continue;
            };

            let mut references = HashSet::new();
            collect_name_tokens_excluding_nested_procedures(body, &mut references);
            references.retain(|name| !hidden.contains(name));
            captured.extend(references);
            collect_nested_block_captures(body, &hidden, &mut captured);
        }
        captured
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
        // An array declaration (LANG-FULL E5): `integer array A[1:10]`.  Each
        // segment is lowered to an `alloc_array` whose length is the run-time
        // span `upper - lower + 1`; the binding records the lower bound so a
        // later `A[i]` access can translate to the 0-based IIR index `i - lower`.
        if let Some(array_decl) = first_direct_node(node, "own_array_decl") {
            return self.emit_array_decl(array_decl, true);
        }
        if let Some(array_decl) = first_direct_node(node, "array_decl") {
            return self.emit_array_decl(array_decl, false);
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

        // LANG-FULL AL6: a leading `own` token gives these variables static
        // lifetime — they become module globals that persist across calls.
        let is_own = direct_tokens(type_decl).iter().any(|t| t.value == "own");

        for name in direct_tokens(ident_list)
            .into_iter()
            .filter(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
        {
            let slot = self.declare_var(&name, ty, is_own)?;
            // A global (an `own` variable, or an E6-captured block scalar) is
            // zero-initialised once at module load — exactly the `own`
            // lifetime semantics — so it must NOT get a per-declaration `const`
            // init. Emitting one inside a procedure body would re-zero the
            // global on every call (destroying persistence); even at block
            // level it would be a dead register write shadowing the global.
            // A plain (register) scalar keeps its zero-init `const`.
            let is_global = is_own || self.block_captured.contains(&name);
            if !is_global && ty != ScalarType::String {
                self.emit(IIRInstr::new(
                    "const",
                    Some(slot.clone()),
                    vec![ty.default_operand()],
                    ty.iir(),
                ));
            }
            if is_own && ty == ScalarType::String {
                // A string handle cannot use the all-zero scalar default: the
                // string backends dereference it for comparison and output. An
                // `own` declaration runs in a procedure body, so initialise its
                // empty-string value behind a persistent flag exactly once.
                let flag = format!("{slot}.__algol_own_string_initialized");
                let initialized = self.fresh_temp();
                let initialize_label = self.fresh_label("own_string_initialize");
                let ready_label = self.fresh_label("own_string_ready");
                self.emit(IIRInstr::new(
                    "global_load",
                    Some(initialized.clone()),
                    vec![Operand::Str(flag.clone())],
                    "i64",
                ));
                self.emit(IIRInstr::new(
                    "jmp_if_false",
                    None,
                    vec![Operand::Var(initialized), Operand::Var(initialize_label.clone())],
                    "void",
                ));
                self.emit(IIRInstr::new(
                    "jmp",
                    None,
                    vec![Operand::Var(ready_label.clone())],
                    "void",
                ));
                self.emit_label(&initialize_label);
                let empty = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "str_const",
                    Some(empty.clone()),
                    vec![Operand::Str(String::new())],
                    "str",
                ));
                self.emit(IIRInstr::new(
                    "global_store",
                    None,
                    vec![Operand::Str(slot.clone()), Operand::Var(empty)],
                    "void",
                ));
                let one = self.emit_const(ScalarType::Integer, Operand::Int(1));
                self.emit(IIRInstr::new(
                    "global_store",
                    None,
                    vec![Operand::Str(flag), Operand::Var(one)],
                    "void",
                ));
                self.emit_label(&ready_label);
                self.initialized_string_slots.insert(slot);
            }
        }
        Ok(())
    }

    /// Lower an `array_decl` (LANG-FULL E5 / AL-multidim).
    ///
    /// ```text
    /// integer array A, B[1:10]         -- 1-D
    /// integer array M[1:3, 1:4]        -- 2-D (row-major, 12 elements)
    ///   ^type        ^names ^bound_pairs (one per dimension)
    /// ```
    ///
    /// The element type defaults to `real` when omitted (ALGOL 60 rule).  Each
    /// `array_segment` declares one or more names that share the same bound list.
    /// For each name we evaluate all dimension bounds at run time, compute the
    /// flat total length (product of all dimension sizes), and emit one
    /// `alloc_array`.  Per-dimension lower bounds and row-major strides are
    /// recorded in `ArrayInfo.dims` so `A[i, j]` can compute the flat index.
    fn emit_array_decl(
        &mut self,
        node: &GrammarASTNode,
        is_own: bool,
    ) -> Result<(), CompileError> {
        let elem_ty = match first_direct_node(node, "type") {
            Some(type_node) => self.scalar_type(type_node)?,
            None => ScalarType::Real, // bare `array A[..]` is `real` in ALGOL 60
        };
        if !matches!(
            elem_ty,
            ScalarType::Integer | ScalarType::Real | ScalarType::Boolean | ScalarType::String
        ) {
            return Err(CompileError::Unsupported(format!(
                "{} arrays (only integer/real/boolean/string element types so far)",
                elem_ty.name()
            )));
        }

        for segment in direct_nodes(node)
            .into_iter()
            .filter(|n| n.rule_name == "array_segment")
        {
            let names: Vec<String> = first_direct_node(segment, "ident_list")
                .map(ident_list_names)
                .unwrap_or_default();
            if names.is_empty() {
                return Err(CompileError::Malformed(
                    "array_segment has no names".into(),
                ));
            }

            // An `own` array declared inside a procedure is allocated on the
            // first invocation only. The flag is a separate scalar global so
            // every standard backend can use its existing i64 global support
            // to guard the typed array-handle global and its dimension metadata.
            let own_init = is_own.then(|| {
                let array_slot = self.scoped_slot_name(&names[0]);
                let flag = format!("{array_slot}.__algol_own_array_initialized");
                let initialized = self.fresh_temp();
                let allocate_label = self.fresh_label("own_array_allocate");
                let ready_label = self.fresh_label("own_array_ready");
                self.emit(IIRInstr::new(
                    "global_load",
                    Some(initialized.clone()),
                    vec![Operand::Str(flag.clone())],
                    "i64",
                ));
                self.emit(IIRInstr::new(
                    "jmp_if_false",
                    None,
                    vec![Operand::Var(initialized), Operand::Var(allocate_label.clone())],
                    "void",
                ));
                self.emit(IIRInstr::new(
                    "jmp",
                    None,
                    vec![Operand::Var(ready_label.clone())],
                    "void",
                ));
                self.emit_label(&allocate_label);
                (flag, ready_label)
            });

            let bound_pairs: Vec<&GrammarASTNode> = direct_nodes(segment)
                .into_iter()
                .filter(|n| n.rule_name == "bound_pair")
                .collect();
            if bound_pairs.is_empty() {
                return Err(CompileError::Malformed(
                    "array segment has no bounds".into(),
                ));
            }

            // Evaluate (lower_slot, size_slot) for each dimension.
            // size = upper − lower + 1  (all are run-time i64 values).
            let mut lower_slots: Vec<String> = Vec::with_capacity(bound_pairs.len());
            let mut size_slots: Vec<String> = Vec::with_capacity(bound_pairs.len());

            for bp in &bound_pairs {
                // bound_pair = arith_expr COLON arith_expr → [lower, upper]
                let bounds: Vec<&GrammarASTNode> = direct_nodes(bp)
                    .into_iter()
                    .filter(|n| n.rule_name == "arith_expr")
                    .collect();
                if bounds.len() != 2 {
                    return Err(CompileError::Malformed(
                        "bound_pair must have exactly two bounds".into(),
                    ));
                }
                let lower = self.emit_expr(bounds[0])?;
                if lower.ty != ScalarType::Integer {
                    return Err(CompileError::Type(
                        "array lower bound must be an integer".into(),
                    ));
                }
                let upper = self.emit_expr(bounds[1])?;
                if upper.ty != ScalarType::Integer {
                    return Err(CompileError::Type(
                        "array upper bound must be an integer".into(),
                    ));
                }

                // size = upper − lower + 1
                let span = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "sub",
                    Some(span.clone()),
                    vec![
                        Operand::Var(upper.slot.clone()),
                        Operand::Var(lower.slot.clone()),
                    ],
                    "i64",
                ));
                let one = self.emit_const(ScalarType::Integer, Operand::Int(1));
                let size = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "add",
                    Some(size.clone()),
                    vec![Operand::Var(span), Operand::Var(one)],
                    "i64",
                ));
                lower_slots.push(lower.slot);
                size_slots.push(size);
            }

            // Compute strides right-to-left (row-major).
            //   stride[last] = 1  → represented as None (multiply elided)
            //   stride[d]    = size[d+1] * stride[d+1]
            let n = bound_pairs.len();
            let mut stride_slots: Vec<Option<String>> = vec![None; n]; // last dim = 1
            // running product: None means "1" (no slot needed yet)
            let mut running: Option<String> = None;

            for d in (0..n.saturating_sub(1)).rev() {
                // stride[d] = size[d+1] * running  (running starts as 1 = None)
                let s_next = &size_slots[d + 1];
                let stride_d = if let Some(prev) = running {
                    let prod = self.fresh_temp();
                    self.emit(IIRInstr::new(
                        "mul",
                        Some(prod.clone()),
                        vec![Operand::Var(s_next.clone()), Operand::Var(prev)],
                        "i64",
                    ));
                    prod
                } else {
                    // stride = size[d+1] * 1 = size[d+1]
                    s_next.clone()
                };
                stride_slots[d] = Some(stride_d.clone());
                running = stride_slots[d].clone();
            }

            // Total allocation length = size[0] * stride[0]  (for N ≥ 2)
            //                         = size[0]              (for N = 1)
            let total_len = if n == 1 {
                size_slots[0].clone()
            } else {
                let stride_0 = stride_slots[0]
                    .clone()
                    .expect("non-last dimension always has a stride slot");
                let total = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "mul",
                    Some(total.clone()),
                    vec![Operand::Var(size_slots[0].clone()), Operand::Var(stride_0)],
                    "i64",
                ));
                total
            };

            // Build the per-dimension descriptor.
            let dims: Vec<ArrayDim> = lower_slots
                .into_iter()
                .zip(stride_slots)
                .map(|(lower_slot, stride_slot)| ArrayDim {
                    lower_slot,
                    stride_slot,
                })
                .collect();

            let array_ty = make_array_type(elem_ty.iir());
            for name in names {
                let is_global = is_own || self.block_captured.contains(&name);
                let handle = self.declare_array(&name, elem_ty, dims.clone(), is_global)?;
                let alloc_dest = if is_global {
                    self.fresh_temp()
                } else {
                    handle.clone()
                };
                self.emit(IIRInstr::new(
                    "alloc_array",
                    Some(alloc_dest.clone()),
                    vec![Operand::Var(total_len.clone())],
                    &array_ty,
                ));
                if is_global {
                    self.emit(IIRInstr::new(
                        "global_store",
                        None,
                        vec![Operand::Str(handle.clone()), Operand::Var(alloc_dest)],
                        "void",
                    ));
                    for (dim_index, dim) in dims.iter().enumerate() {
                        self.emit(IIRInstr::new(
                            "global_store",
                            None,
                            vec![
                                Operand::Str(array_dim_global_name(&handle, dim_index, "lower")),
                                Operand::Var(dim.lower_slot.clone()),
                            ],
                            "void",
                        ));
                        if let Some(stride) = &dim.stride_slot {
                            self.emit(IIRInstr::new(
                                "global_store",
                                None,
                                vec![
                                    Operand::Str(array_dim_global_name(
                                        &handle,
                                        dim_index,
                                        "stride",
                                    )),
                                    Operand::Var(stride.clone()),
                                ],
                                "void",
                            ));
                        }
                    }
                }
            }
            if let Some((flag, ready_label)) = own_init {
                let one = self.emit_const(ScalarType::Integer, Operand::Int(1));
                self.emit(IIRInstr::new(
                    "global_store",
                    None,
                    vec![Operand::Str(flag), Operand::Var(one)],
                    "void",
                ));
                self.emit_label(&ready_label);
            }
        }
        Ok(())
    }

    /// Resolve a subscripted `variable` node `A[i]` (or `A[i, j]`, etc.) to
    /// the array handle slot plus a slot holding the flat **0-based** linear
    /// index.  Shared by the `array_get` (read) and `array_set` (write) paths.
    ///
    /// For a 1-D array: flat_idx = i − lower  (same as before).
    /// For an N-D array (row-major):
    ///   flat_idx = Σ_d  (sub[d] − lower[d]) * stride[d]
    /// where stride[last] = 1 (multiply elided) and
    ///   stride[d] = size[d+1] * stride[d+1]  for d < last.
    fn resolve_array_index(
        &mut self,
        var_node: &GrammarASTNode,
    ) -> Result<(VarBinding, String), CompileError> {
        let name = direct_tokens(var_node)
            .into_iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CompileError::Malformed("subscripted variable missing name".into()))?;
        let mut binding = self.require_var(&name)?;
        let Some(info) = binding.array.clone() else {
            return Err(CompileError::Type(format!(
                "{name:?} is not an array — cannot subscript it"
            )));
        };
        let subs = array_subscripts(var_node).ok_or_else(|| {
            CompileError::Malformed("subscripted variable missing subscripts".into())
        })?;
        if subs.len() != info.dims.len() {
            return Err(CompileError::Type(format!(
                "{name:?} is {}-dimensional but {} subscript(s) given",
                info.dims.len(),
                subs.len()
            )));
        }

        // Compute flat 0-based index: Σ_d (sub[d] − lower[d]) * stride[d].
        // Accumulate into `flat`; start with None meaning "haven't written yet".
        let mut flat: Option<String> = None;

        for (dim_index, (dim, sub_node)) in info.dims.iter().zip(subs).enumerate() {
            let idx = self.emit_expr(sub_node)?;
            if idx.ty != ScalarType::Integer {
                return Err(CompileError::Type(format!(
                    "array subscript for {name:?} must be an integer"
                )));
            }

            // diff = sub − lower. A captured array owns its bound metadata in
            // module globals, because the procedure body has a fresh register
            // frame and cannot directly see the declaring block's temporaries.
            let lower_slot = if binding.is_global {
                let slot = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "global_load",
                    Some(slot.clone()),
                    vec![Operand::Str(array_dim_global_name(
                        &binding.slot,
                        dim_index,
                        "lower",
                    ))],
                    "i64",
                ));
                slot
            } else {
                dim.lower_slot.clone()
            };
            let diff = self.fresh_temp();
            self.emit(IIRInstr::new(
                "sub",
                Some(diff.clone()),
                vec![Operand::Var(idx.slot), Operand::Var(lower_slot)],
                "i64",
            ));

            // contrib = diff * stride  (or just diff when stride = 1, last dim)
            let contrib = if let Some(stride) = &dim.stride_slot {
                let stride_slot = if binding.is_global {
                    let slot = self.fresh_temp();
                    self.emit(IIRInstr::new(
                        "global_load",
                        Some(slot.clone()),
                        vec![Operand::Str(array_dim_global_name(
                            &binding.slot,
                            dim_index,
                            "stride",
                        ))],
                        "i64",
                    ));
                    slot
                } else {
                    stride.clone()
                };
                let prod = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "mul",
                    Some(prod.clone()),
                    vec![Operand::Var(diff), Operand::Var(stride_slot)],
                    "i64",
                ));
                prod
            } else {
                diff // last dimension: stride = 1, contrib = diff
            };

            // flat += contrib
            flat = Some(if let Some(acc) = flat {
                let sum = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "add",
                    Some(sum.clone()),
                    vec![Operand::Var(acc), Operand::Var(contrib)],
                    "i64",
                ));
                sum
            } else {
                contrib // first (or only) dimension: flat = contrib
            });
        }

        let flat = flat.expect("dims is always non-empty");
        if binding.is_global {
            let handle = self.fresh_temp();
            self.emit(IIRInstr::new(
                "global_load",
                Some(handle.clone()),
                vec![Operand::Str(binding.slot.clone())],
                make_array_type(binding.ty.iir()),
            ));
            binding.slot = handle;
            binding.is_global = false;
        }
        Ok((binding, flat))
    }

    /// Lower `A[i]` in an expression to a bounds-checked `array_get` (E5).
    fn emit_array_read(&mut self, var_node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        let (binding, zero) = self.resolve_array_index(var_node)?;
        let dest = self.fresh_temp();
        self.emit(IIRInstr::new(
            "array_get",
            Some(dest.clone()),
            vec![Operand::Var(binding.slot), Operand::Var(zero)],
            binding.ty.iir(),
        ));
        Ok(ExprValue {
            slot: dest,
            ty: binding.ty,
        })
    }

    fn scalar_type(&self, node: &GrammarASTNode) -> Result<ScalarType, CompileError> {
        let token = single_token_recursive(node)
            .ok_or_else(|| CompileError::Malformed("type node has no token".into()))?;
        match token.value.as_str() {
            "integer" => Ok(ScalarType::Integer),
            "real" => Ok(ScalarType::Real),
            "boolean" => Ok(ScalarType::Boolean),
            "string" => Ok(ScalarType::String),
            other => Err(CompileError::Malformed(format!(
                "unknown type token {other:?}"
            ))),
        }
    }

    /// Map a procedure `specifier` to the supported formal shape.
    ///
    /// The compiled parser accepts `integer array a` as a two-token specifier;
    /// its legacy `array a` spelling remains a real-array formal, matching the
    /// default element type of an untyped ALGOL array declaration. Array
    /// formals infer their dimension count from their subscripted uses in the
    /// procedure body; the actual's rank is checked when the call is lowered.
    fn procedure_param_type(
        &self,
        node: &GrammarASTNode,
    ) -> Result<ProcedureParamType, CompileError> {
        let tokens = recursive_tokens(node);
        let words: Vec<&str> = tokens.iter().map(|token| token.value.as_str()).collect();
        let scalar = |word: &str| match word {
            "integer" => Some(ScalarType::Integer),
            "real" => Some(ScalarType::Real),
            "boolean" => Some(ScalarType::Boolean),
            "string" => Some(ScalarType::String),
            _ => None,
        };

        match words.as_slice() {
            ["array"] => Ok(ProcedureParamType::Array {
                elem_ty: ScalarType::Real,
                dimensions: 1,
            }),
            [ty, "array"] => scalar(ty)
                .map(|elem_ty| ProcedureParamType::Array {
                    elem_ty,
                    dimensions: 1,
                })
                .ok_or_else(|| CompileError::Malformed(format!(
                    "unknown array parameter element type {ty:?}"
                ))),
            [ty] => scalar(ty)
                .map(ProcedureParamType::Scalar)
                .ok_or_else(|| CompileError::Unsupported(format!("{ty} parameters"))),
            [_, kind] if matches!(*kind, "label" | "switch" | "procedure") => Err(
                CompileError::Unsupported(format!("{kind} parameters")),
            ),
            _ => Err(CompileError::Malformed(format!(
                "unknown procedure parameter specifier {words:?}"
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
    /// (call-by-name / Jensen's device is not modelled), and every parameter
    /// must be specified exactly once. Scalar formals and integer/real/boolean/string
    /// array descriptors are supported; an array formal's rank is inferred from
    /// subscripted uses in its body. A missing heading type is a proper procedure
    /// and lowers to an IIR `void` function.
    fn procedure_parts(
        &self,
        proc_decl: &GrammarASTNode,
    ) -> Result<ProcedureParts, CompileError> {
        let name = direct_tokens(proc_decl)
            .into_iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CompileError::Malformed("procedure_decl missing name".into()))?;

        let ret = first_direct_node(proc_decl, "type")
            .map(|type_node| self.scalar_type(type_node))
            .transpose()?;
        // E4-dyn payoff (E4d-AL): `string procedure`s are now supported. The
        // result variable (the procedure name) holds a runtime string handle,
        // which every backend can carry and `print` since the E4-dyn foothold
        // landed on all seven columns. A body that assigns the result in more
        // than one branch (`if … then p := "HI" else p := "LO"`) makes the
        // result a genuinely runtime string; the backends' E4-dyn promotion
        // reads its length from the buffer header at run time.

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
        let mut type_of: HashMap<String, ProcedureParamType> = HashMap::new();
        for spec in direct_nodes(proc_decl)
            .into_iter()
            .filter(|n| n.rule_name == "spec_part")
        {
            let specifier = first_direct_node(spec, "specifier")
                .ok_or_else(|| CompileError::Malformed("spec_part missing specifier".into()))?;
            let ty = self.procedure_param_type(specifier)?;
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
            let ty = match ty {
                ProcedureParamType::Array { elem_ty, .. } => ProcedureParamType::Array {
                    elem_ty,
                    dimensions: array_formal_dimension_count(proc_decl, &p)?,
                },
                ProcedureParamType::Scalar(_) => ty,
            };
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
        let captured_array_formals = array_formals_captured_by_nested_procedures(proc_decl, &params);
        let captured_scalar_formals = scalar_formals_captured_by_nested_procedures(proc_decl, &params);

        // ── swap in a fresh emission context ─────────────────────────────
        let saved_instrs = std::mem::take(&mut self.instrs);
        let saved_source_map = std::mem::take(&mut self.source_map);
        let saved_registers = std::mem::take(&mut self.register_names);
        let saved_defined = std::mem::take(&mut self.defined_labels);
        let saved_referenced = std::mem::take(&mut self.referenced_labels);
        let saved_switches = std::mem::take(&mut self.switches);
        let saved_switch_expansion_steps = std::mem::replace(&mut self.switch_expansion_steps, 0);
        let saved_initialized_string_slots =
            std::mem::take(&mut self.initialized_string_slots);
        let saved_scopes = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);

        // E6: a procedure body addresses an enclosing block scalar that was
        // materialised as a global (`is_global`).  The fresh scope above hides
        // the enclosing scopes, so re-inject every visible global binding so the
        // body's `require_var` resolves it (and lowers to `global_load`/
        // `global_store`).  A value parameter with the same name shadows it
        // below — `declare_var` overwrites the entry — which is correct ALGOL
        // scoping.
        for scope in &saved_scopes {
            for (gname, binding) in scope {
                if binding.is_global {
                    self.scopes[0].insert(gname.clone(), binding.clone());
                }
            }
        }
        // A formal parameter or result with the same spelling shadows an
        // injected enclosing global. Keep this set separate so a duplicate
        // formal still reaches the normal duplicate-declaration error.
        let mut injected_global_names: HashSet<String> =
            self.scopes[0].keys().cloned().collect();

        // Bind value parameters and, for typed procedures, the result variable
        // (the procedure name). An array descriptor carries its typed handle,
        // every lower bound, and every non-final row-major stride. The caller
        // keeps ownership of the backing storage, so element writes in the
        // callee are visible to the actual array.
        let mut param_pairs: Vec<(String, String)> = Vec::with_capacity(params.len() * 4);
        for (pname, pty) in &params {
            if injected_global_names.remove(pname) {
                self.scopes[0].remove(pname);
            }
            match pty {
                ProcedureParamType::Scalar(pty) => {
                    // Parameters and the result slot are real registers, never `own`.
                    let slot = self.declare_var(pname, *pty, false)?;
                    // A string parameter is initialized by the caller, but can carry
                    // a runtime handle. It is deliberately not literal-backed: only
                    // direct `str_const` producers support ordering comparisons.
                    if *pty == ScalarType::String {
                        self.initialized_string_slots.insert(slot.clone());
                    }
                    if captured_scalar_formals.contains(pname) {
                        self.promote_scalar_parameter_capture(&name, pname, *pty, &slot)?;
                    }
                    param_pairs.push((slot, pty.iir().to_string()));
                }
                ProcedureParamType::Array {
                    elem_ty,
                    dimensions,
                } => {
                    if !matches!(
                        *elem_ty,
                        ScalarType::Integer
                            | ScalarType::Real
                            | ScalarType::Boolean
                            | ScalarType::String
                    ) {
                        return Err(CompileError::Unsupported(format!(
                            "{} array parameters (only integer/real/boolean/string element types so far)",
                            elem_ty.name()
                        )));
                    }
                    let dims = (0..*dimensions)
                        .map(|dim_index| ArrayDim {
                            lower_slot: array_param_dim_lower_slot(pname, dim_index),
                            stride_slot: (dim_index + 1 < *dimensions)
                                .then(|| array_param_stride_slot(pname, dim_index)),
                        })
                        .collect();
                    let handle = self.declare_array(
                        pname,
                        *elem_ty,
                        dims,
                        false,
                    )?;
                    param_pairs.push((handle.clone(), make_array_type(elem_ty.iir())));
                    for dim_index in 0..*dimensions {
                        let lower_slot = array_param_dim_lower_slot(pname, dim_index);
                        self.register_names.insert(lower_slot.clone());
                        param_pairs.push((lower_slot, "i64".to_string()));
                        if dim_index + 1 < *dimensions {
                            let stride_slot = array_param_stride_slot(pname, dim_index);
                            self.register_names.insert(stride_slot.clone());
                            param_pairs.push((stride_slot, "i64".to_string()));
                        }
                    }
                    if captured_array_formals.contains(pname) {
                        self.promote_array_parameter_capture(
                            &name,
                            pname,
                            *elem_ty,
                            *dimensions,
                            &handle,
                        )?;
                    }
                }
            }
        }
        let result_slot = if let Some(ret) = ret {
            // The procedure's name is an in-scope variable holding the return
            // value; seed it with a default so a path that never assigns it
            // still returns a defined value.
            // Capture analysis is deliberately conservative and walks the
            // whole declaration, including `name := ...` in this procedure's
            // body. The result variable belongs to this fresh procedure frame,
            // never to the enclosing block, even if that scan recorded its
            // spelling as a candidate capture.
            let result_was_captured = self.block_captured.remove(&name);
            if injected_global_names.remove(&name) {
                self.scopes[0].remove(&name);
            }
            let declared_result = self.declare_var(&name, ret, false);
            if result_was_captured {
                self.block_captured.insert(name.clone());
            }
            let result_slot = declared_result?;
            if ret == ScalarType::String {
                // A string result is a runtime handle, so its default must be a real
                // (empty) string buffer, not a `const 0`. Seed it with `str_const ""`
                // — the same shape a literal assignment produces — and mark it
                // literal-backed so an unassigned path still yields a printable value.
                self.emit(IIRInstr::new(
                    "str_const",
                    Some(result_slot.clone()),
                    vec![Operand::Str(String::new())],
                    "str",
                ));
                self.initialized_string_slots.insert(result_slot.clone());
            } else {
                self.emit(IIRInstr::new(
                    "const",
                    Some(result_slot.clone()),
                    vec![ret.default_operand()],
                    ret.iir(),
                ));
            }
            Some(result_slot)
        } else {
            None
        };

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

        let return_type = if let (Some(ret), Some(result_slot)) = (ret, result_slot) {
            self.emit(IIRInstr::new(
                "ret",
                None,
                vec![Operand::Var(result_slot)],
                ret.iir(),
            ));
            ret.iir()
        } else {
            self.emit(IIRInstr::new("ret_void", None, vec![], "void"));
            "void"
        };

        // ── assemble the function and restore the caller's context ───────
        let body_instrs = std::mem::take(&mut self.instrs);
        let body_len = body_instrs.len();
        let mut func = IIRFunction::new(name, param_pairs, return_type, body_instrs);
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
        self.switch_expansion_steps = saved_switch_expansion_steps;
        self.initialized_string_slots = saved_initialized_string_slots;
        self.scopes = saved_scopes;

        Ok(func)
    }

    /// Publish a scalar value formal into module-global storage when a nested
    /// procedure needs to read or write it from a separate IIR function frame.
    fn promote_scalar_parameter_capture(
        &mut self,
        procedure_name: &str,
        param_name: &str,
        ty: ScalarType,
        incoming_slot: &str,
    ) -> Result<(), CompileError> {
        let capture_slot = scalar_param_capture_slot(procedure_name, param_name);
        let binding = self
            .scopes
            .last_mut()
            .and_then(|scope| scope.get_mut(param_name))
            .ok_or_else(|| CompileError::Malformed(format!(
                "scalar parameter {param_name:?} missing while preparing nested capture"
            )))?;
        if binding.ty != ty || binding.array.is_some() {
            return Err(CompileError::Malformed(format!(
                "scalar parameter {param_name:?} has an inconsistent binding"
            )));
        }
        binding.slot = capture_slot.clone();
        binding.is_global = true;

        self.emit(IIRInstr::new(
            "global_store",
            None,
            vec![
                Operand::Str(capture_slot),
                Operand::Var(incoming_slot.to_string()),
            ],
            "void",
        ));
        Ok(())
    }

    /// Copy an incoming array descriptor into module globals before a nested
    /// procedure can run. Nested procedures compile as sibling IIR functions,
    /// so their fresh frames cannot read the outer procedure's parameter slots
    /// directly; rebinding the outer formal to the global descriptor gives both
    /// functions the same storage handle, lower bounds, and strides.
    fn promote_array_parameter_capture(
        &mut self,
        procedure_name: &str,
        param_name: &str,
        elem_ty: ScalarType,
        dimensions: usize,
        incoming_handle: &str,
    ) -> Result<(), CompileError> {
        let capture_slot = array_param_capture_slot(procedure_name, param_name);
        let binding = self
            .scopes
            .last_mut()
            .and_then(|scope| scope.get_mut(param_name))
            .ok_or_else(|| CompileError::Malformed(format!(
                "array parameter {param_name:?} missing while preparing nested capture"
            )))?;
        if binding.ty != elem_ty || binding.array.is_none() {
            return Err(CompileError::Malformed(format!(
                "array parameter {param_name:?} has an inconsistent descriptor"
            )));
        }
        binding.slot = capture_slot.clone();
        binding.is_global = true;

        self.emit(IIRInstr::new(
            "global_store",
            None,
            vec![
                Operand::Str(capture_slot.clone()),
                Operand::Var(incoming_handle.to_string()),
            ],
            "void",
        ));
        for dim_index in 0..dimensions {
            self.emit(IIRInstr::new(
                "global_store",
                None,
                vec![
                    Operand::Str(array_dim_global_name(&capture_slot, dim_index, "lower")),
                    Operand::Var(array_param_dim_lower_slot(param_name, dim_index)),
                ],
                "void",
            ));
            if dim_index + 1 < dimensions {
                self.emit(IIRInstr::new(
                    "global_store",
                    None,
                    vec![
                        Operand::Str(array_dim_global_name(&capture_slot, dim_index, "stride")),
                        Operand::Var(array_param_stride_slot(param_name, dim_index)),
                    ],
                    "void",
                ));
            }
        }
        Ok(())
    }

    /// Lower a procedure *call* in value position (`sq(7)`), returning the
    /// slot that holds the result.  Used from `emit_expr`.
    fn emit_proc_call(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        self.set_loc(node);
        self.emit_call_common(node, true)?.ok_or_else(|| {
            CompileError::Type("proper procedure call has no return value".into())
        })
    }

    /// Lower a procedure *call* in statement position (`bump(3)`).  A typed
    /// procedure's returned value is computed but discarded; a proper procedure
    /// emits a void call with no destination.
    fn emit_proc_stmt(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        let name = direct_tokens(node)
            .into_iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CompileError::Malformed("procedure statement has no name".into()))?;
        if self.try_emit_standard_output_stmt(&name, node)? {
            return Ok(());
        }
        self.emit_call_common(node, false)?;
        Ok(())
    }

    /// ALGOL 60's report leaves input/output in implementation-defined
    /// procedures; this LANG-FULL AL4 foothold recognises undeclared statement
    /// calls named `print` or `output` and lowers literal string arguments to
    /// the shared E4 stdout primitive. A user-declared procedure of the same
    /// name still wins, matching the standard-function override policy.
    fn try_emit_standard_output_stmt(
        &mut self,
        name: &str,
        node: &GrammarASTNode,
    ) -> Result<bool, CompileError> {
        if !matches!(name, "print" | "output") || self.proc_sigs.contains_key(name) {
            return Ok(false);
        }

        let actuals = self.standard_fn_actuals(node);
        if actuals.is_empty() {
            return Err(CompileError::Type(format!(
                "standard output procedure {name:?} expects at least 1 argument"
            )));
        }

        for actual in actuals {
            if let Some(literal) = expr_string_literal(actual) {
                let slot = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "str_const",
                    Some(slot.clone()),
                    vec![Operand::Str(literal)],
                    "str",
                ));
                self.emit(IIRInstr::new(
                    "print_str",
                    None,
                    vec![Operand::Var(slot)],
                    "void",
                ));
                continue;
            }

            if let Some(var_name) = expr_variable_name(actual) {
                let binding = self.require_var(&var_name)?;
                if binding.ty != ScalarType::String {
                    return Err(CompileError::Type(format!(
                        "standard output procedure {name:?} cannot print {} variable {var_name:?}",
                        binding.ty.name()
                    )));
                }
                if !binding.is_global && !self.initialized_string_slots.contains(&binding.slot) {
                    return Err(CompileError::Unsupported(format!(
                        "standard output procedure {name:?} requires initialized string variable {var_name:?}"
                    )));
                }
                let value = self.read_scalar(binding);
                self.emit(IIRInstr::new(
                    "print_str",
                    None,
                    vec![Operand::Var(value.slot)],
                    "void",
                ));
                continue;
            }

            // E4-dyn payoff (E4d-AL): a general string-valued expression — most
            // importantly a `string procedure` call, e.g. `print(pick(1))`.
            // Evaluate it to a runtime string handle and print it. This is now
            // sound on every backend because the E4-dyn foothold proved
            // `print_str` of a runtime string on all seven columns, so — unlike
            // the literal/variable fast paths above — no literal-backing is
            // required. A non-string result is a type error.
            let value = self.emit_expr(actual)?;
            if value.ty != ScalarType::String {
                return Err(CompileError::Type(format!(
                    "standard output procedure {name:?} cannot print a {} value",
                    value.ty.name()
                )));
            }
            self.emit(IIRInstr::new(
                "print_str",
                None,
                vec![Operand::Var(value.slot)],
                "void",
            ));
        }

        Ok(true)
    }

    /// Shared call-lowering for `proc_call` and `proc_stmt`: resolve the
    /// signature, evaluate and type-check the actuals, then emit a `call`
    /// whose `srcs[0]` names the callee and whose remaining `srcs` are the
    /// argument slots, matching the IIR calling convention every backend
    /// understands.
    fn emit_call_common(
        &mut self,
        node: &GrammarASTNode,
        require_value: bool,
    ) -> Result<Option<ExprValue>, CompileError> {
        let name = direct_tokens(node)
            .into_iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CompileError::Malformed("call has no procedure name".into()))?;

        // ALGOL 60 §3.2.4 *standard functions* (`abs`, `sign`, `entier`, …) are
        // built into the language, not user-declared procedures, so they have no
        // `proc_sigs` entry.  A program may still legally *redeclare* one as its
        // own procedure — so we only fall back to the built-in when the name is
        // not a user-declared procedure.  This keeps the override semantics the
        // Report grants while making `abs(x)` work out of the box.
        let sig = match self.proc_sigs.get(&name).cloned() {
            Some(sig) => sig,
            None => {
                if let Some(result) = self.try_emit_standard_function(&name, node)? {
                    return Ok(Some(result));
                }
                return Err(CompileError::Type(format!(
                    "call to undeclared procedure {name:?}"
                )));
            }
        };

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

        let mut arg_slots = Vec::with_capacity(actuals.len() * 4);
        for (actual, expected) in actuals.iter().zip(sig.params.iter()) {
            match expected {
                ProcedureParamType::Scalar(expected) => {
                    let value = self.emit_expr(actual)?;
                    let value = self.coerce_value(
                        value,
                        *expected,
                        &format!("procedure {name:?}: argument"),
                    )?;
                    arg_slots.push(value.slot);
                }
                ProcedureParamType::Array {
                    elem_ty: expected_elem_ty,
                    dimensions: expected_dimensions,
                } => {
                    let actual_name = expr_variable_name(actual).ok_or_else(|| {
                        CompileError::Type(format!(
                            "procedure {name:?}: array parameter requires a bare array variable"
                        ))
                    })?;
                    let binding = self.require_var(&actual_name)?;
                    let info = binding.array.clone().ok_or_else(|| {
                        CompileError::Type(format!(
                            "procedure {name:?}: argument {actual_name:?} is not an array"
                        ))
                    })?;
                    if info.elem_ty != *expected_elem_ty {
                        return Err(CompileError::Type(format!(
                            "procedure {name:?}: array argument {actual_name:?} has {} elements but parameter expects {}",
                            info.elem_ty.name(),
                            expected_elem_ty.name()
                        )));
                    }
                    if info.dims.len() != *expected_dimensions {
                        return Err(CompileError::Type(format!(
                            "procedure {name:?}: array argument {actual_name:?} is {}-dimensional but the formal is {}-dimensional",
                            info.dims.len(),
                            expected_dimensions
                        )));
                    }

                    // Global arrays (captured or `own`) store their descriptor
                    // metadata outside the current frame. Reload the complete
                    // rank-specific descriptor so the callee gets the same
                    // handle/lower-bound/stride values as a local-array actual.
                    if binding.is_global {
                        let handle = self.fresh_temp();
                        self.emit(IIRInstr::new(
                            "global_load",
                            Some(handle.clone()),
                            vec![Operand::Str(binding.slot.clone())],
                            make_array_type(binding.ty.iir()),
                        ));
                        arg_slots.push(handle);
                        for (dim_index, dim) in info.dims.iter().enumerate() {
                            let lower = self.fresh_temp();
                            self.emit(IIRInstr::new(
                                "global_load",
                                Some(lower.clone()),
                                vec![Operand::Str(array_dim_global_name(
                                    &binding.slot,
                                    dim_index,
                                    "lower",
                                ))],
                                "i64",
                            ));
                            arg_slots.push(lower);
                            if dim.stride_slot.is_some() {
                                let stride = self.fresh_temp();
                                self.emit(IIRInstr::new(
                                    "global_load",
                                    Some(stride.clone()),
                                    vec![Operand::Str(array_dim_global_name(
                                        &binding.slot,
                                        dim_index,
                                        "stride",
                                    ))],
                                    "i64",
                                ));
                                arg_slots.push(stride);
                            }
                        }
                    } else {
                        arg_slots.push(binding.slot);
                        for dim in &info.dims {
                            arg_slots.push(dim.lower_slot.clone());
                            if let Some(stride_slot) = &dim.stride_slot {
                                arg_slots.push(stride_slot.clone());
                            }
                        }
                    }
                }
            }
        }

        if require_value && sig.ret.is_none() {
            return Err(CompileError::Type(format!(
                "proper procedure {name:?} has no return value"
            )));
        }

        let (dest, type_hint) = match sig.ret {
            Some(ret) => (Some(self.fresh_temp()), ret.iir()),
            None => (None, "void"),
        };
        let mut srcs = Vec::with_capacity(arg_slots.len() + 1);
        srcs.push(Operand::Var(name));
        srcs.extend(arg_slots.into_iter().map(Operand::Var));
        self.emit(IIRInstr::new("call", dest.clone(), srcs, type_hint));
        Ok(match (dest, sig.ret) {
            (Some(slot), Some(ty)) => Some(ExprValue { slot, ty }),
            _ => None,
        })
    }

    /// Resolve a *standard function* call (ALGOL 60 §3.2.4) by name.  Returns
    /// `Ok(Some(value))` if `name` is a built-in we lower inline, `Ok(None)` if
    /// it is not a standard function (so the caller raises the usual
    /// "undeclared procedure" error).
    ///
    /// Implemented so far: `abs` (PR-1) and `sign` (PR-2) — both pure IIR
    /// (compare + branch + move/const).  `entier` (floor of a real → integer)
    /// uses the E8 `real_to_int_floor` op.  `sqrt` (PR-4) emits the new
    /// `f64_sqrt` IIR op — every backend maps it to its native hardware sqrt
    /// (aarch64 `fsqrt`, SSE2 `sqrtsd`, WASM `f64.sqrt`, LLVM intrinsic,
    /// JVM `Math.sqrt`, CLR `Math.Sqrt`).  `sin`/`cos`/`ln`/`exp` need the
    /// same cross-backend runtime math library and land in later slices.
    fn try_emit_standard_function(
        &mut self,
        name: &str,
        node: &GrammarASTNode,
    ) -> Result<Option<ExprValue>, CompileError> {
        match name {
            "abs"    => Ok(Some(self.emit_abs(node)?)),
            "sign"   => Ok(Some(self.emit_sign(node)?)),
            "entier" => Ok(Some(self.emit_entier(node)?)),
            "sqrt"   => Ok(Some(self.emit_sqrt(node)?)),
            "sin"    => Ok(Some(self.emit_f64_unary("sin",     "f64_sin",  node)?)),
            "cos"    => Ok(Some(self.emit_f64_unary("cos",     "f64_cos",  node)?)),
            "ln"     => Ok(Some(self.emit_f64_unary("ln",      "f64_ln",   node)?)),
            "exp"    => Ok(Some(self.emit_f64_unary("exp",     "f64_exp",  node)?)),
            "arctan" => Ok(Some(self.emit_f64_unary("arctan",  "f64_atan", node)?)),
            _ => Ok(None),
        }
    }

    /// Extract the actual-parameter expressions of a standard-function call —
    /// the same `actual_params → expression*` shape `emit_call_common` reads.
    fn standard_fn_actuals<'n>(&self, node: &'n GrammarASTNode) -> Vec<&'n GrammarASTNode> {
        match first_direct_node(node, "actual_params") {
            Some(ap) => direct_nodes(ap)
                .into_iter()
                .filter(|n| n.rule_name == "expression")
                .collect(),
            None => Vec::new(),
        }
    }

    /// `abs(E)` — absolute value, preserving `E`'s numeric type
    /// (`integer`→`integer`, `real`→`real`).  We lower it to the value of the
    /// conditional expression `if E < 0 then -E else E`, using the *exact*
    /// `jmp_if_false` / `mov`-into-`dest` shape `emit_conditional_branches`
    /// uses.  Writing `dest` once per branch (store-per-branch, never an SSA
    /// phi) is what lets the result merge identically on all seven backends —
    /// the VM/JIT and the stack machines treat `dest` as a slot, and the LLVM
    /// backend promotes a twice-assigned temp to an `alloca` (the same
    /// reassigned-value path E-LLVM-1 hardened).  `E` is evaluated **once**
    /// (its slot is read by the test, the negation, and the else branch), so
    /// `abs` has no double-evaluation surprise even if `E` has side effects.
    ///
    /// ```text
    ///        t := E                 ; evaluate the operand once
    ///        cond := t < 0          ; cmp_lt at the operand width
    ///        jmp_if_false cond, else
    ///        neg := 0 - t           ; -t  (i64 sub / f64 fsub)
    ///        dest := neg
    ///        jmp end
    ///   else: dest := t
    ///   end:  (dest holds |E|)
    /// ```
    fn emit_abs(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        let actuals = self.standard_fn_actuals(node);
        if actuals.len() != 1 {
            return Err(CompileError::Type(format!(
                "standard function abs expects 1 argument, got {}",
                actuals.len()
            )));
        }
        let value = self.emit_expr(actuals[0])?;
        let ty = match value.ty {
            ScalarType::Integer | ScalarType::Real => value.ty,
            ScalarType::Boolean | ScalarType::String => {
                return Err(CompileError::Type(
                    "standard function abs requires a numeric argument".into(),
                ))
            }
        };

        // cond := (value < 0), compared at the operand width (see the relational
        // lowering in `emit_binary`: the hint is the operand type, not `bool`).
        let zero = self.emit_const(ty, ty.default_operand());
        let cond = self.fresh_temp();
        self.emit(IIRInstr::new(
            "cmp_lt",
            Some(cond.clone()),
            vec![Operand::Var(value.slot.clone()), Operand::Var(zero)],
            ty.iir(),
        ));

        let else_label = self.fresh_label("abs_else");
        let end_label = self.fresh_label("abs_end");
        let dest = self.fresh_temp();

        self.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(cond), Operand::Var(else_label.clone())],
            "void",
        ));
        // then: dest := -value
        let neg = self.emit_unary_minus(ExprValue {
            slot: value.slot.clone(),
            ty,
        })?;
        self.emit(IIRInstr::new(
            "mov",
            Some(dest.clone()),
            vec![Operand::Var(neg.slot)],
            ty.iir(),
        ));
        self.emit(IIRInstr::new(
            "jmp",
            None,
            vec![Operand::Var(end_label.clone())],
            "void",
        ));
        // else: dest := value
        self.emit_label(&else_label);
        self.emit(IIRInstr::new(
            "mov",
            Some(dest.clone()),
            vec![Operand::Var(value.slot)],
            ty.iir(),
        ));
        self.emit_label(&end_label);

        Ok(ExprValue { slot: dest, ty })
    }

    /// `sign(E)` — the *signum*: `+1` if `E > 0`, `-1` if `E < 0`, `0` if
    /// `E = 0` (ALGOL 60 §3.2.4).  Unlike `abs`, the **result is always
    /// `integer`** regardless of the operand's type — `sign(-2.5)` is the
    /// integer `-1`.  The operand may be `integer` or `real`; the comparisons
    /// run at the operand width (the `0` we compare against is typed to match),
    /// but every value `dest` receives is an `i64` constant.
    ///
    /// It lowers to the nested conditional `if E > 0 then 1 else if E < 0 then
    /// -1 else 0`, written with the same store-per-branch `dest` discipline as
    /// `abs` (one `mov`/`const` into `dest` per path, no SSA phi), so it runs
    /// identically on all seven backends.  `E` is evaluated once.
    ///
    /// ```text
    ///        t := E                  ; evaluate the operand once
    ///        gt := t > 0             ; cmp_gt at the operand width
    ///        jmp_if_false gt, neg?   ; not positive → test the sign
    ///        dest := 1
    ///        jmp end
    ///   neg?: lt := t < 0
    ///        jmp_if_false lt, zero   ; not negative (and not positive) → 0
    ///        dest := -1
    ///        jmp end
    ///   zero: dest := 0
    ///   end:  (dest holds sign E, an integer)
    /// ```
    fn emit_sign(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        let actuals = self.standard_fn_actuals(node);
        if actuals.len() != 1 {
            return Err(CompileError::Type(format!(
                "standard function sign expects 1 argument, got {}",
                actuals.len()
            )));
        }
        let value = self.emit_expr(actuals[0])?;
        let operand_ty = match value.ty {
            ScalarType::Integer | ScalarType::Real => value.ty,
            ScalarType::Boolean | ScalarType::String => {
                return Err(CompileError::Type(
                    "standard function sign requires a numeric argument".into(),
                ))
            }
        };

        // The result is always an integer; the three outcomes are i64 consts.
        let dest = self.fresh_temp();
        let neg_label = self.fresh_label("sign_neg");
        let zero_label = self.fresh_label("sign_zero");
        let end_label = self.fresh_label("sign_end");

        // gt := (value > 0), compared at the operand width.
        let zero_operand = self.emit_const(operand_ty, operand_ty.default_operand());
        let gt = self.fresh_temp();
        self.emit(IIRInstr::new(
            "cmp_gt",
            Some(gt.clone()),
            vec![Operand::Var(value.slot.clone()), Operand::Var(zero_operand)],
            operand_ty.iir(),
        ));
        self.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(gt), Operand::Var(neg_label.clone())],
            "void",
        ));
        // positive ⇒ dest := 1
        let one = self.emit_const(ScalarType::Integer, Operand::Int(1));
        self.emit(IIRInstr::new(
            "mov",
            Some(dest.clone()),
            vec![Operand::Var(one)],
            "i64",
        ));
        self.emit(IIRInstr::new(
            "jmp",
            None,
            vec![Operand::Var(end_label.clone())],
            "void",
        ));

        // neg?: lt := (value < 0)
        self.emit_label(&neg_label);
        let zero_operand2 = self.emit_const(operand_ty, operand_ty.default_operand());
        let lt = self.fresh_temp();
        self.emit(IIRInstr::new(
            "cmp_lt",
            Some(lt.clone()),
            vec![Operand::Var(value.slot), Operand::Var(zero_operand2)],
            operand_ty.iir(),
        ));
        self.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(lt), Operand::Var(zero_label.clone())],
            "void",
        ));
        // negative ⇒ dest := -1
        let minus_one = self.emit_const(ScalarType::Integer, Operand::Int(-1));
        self.emit(IIRInstr::new(
            "mov",
            Some(dest.clone()),
            vec![Operand::Var(minus_one)],
            "i64",
        ));
        self.emit(IIRInstr::new(
            "jmp",
            None,
            vec![Operand::Var(end_label.clone())],
            "void",
        ));

        // zero ⇒ dest := 0
        self.emit_label(&zero_label);
        let zero_result = self.emit_const(ScalarType::Integer, Operand::Int(0));
        self.emit(IIRInstr::new(
            "mov",
            Some(dest.clone()),
            vec![Operand::Var(zero_result)],
            "i64",
        ));
        self.emit_label(&end_label);

        Ok(ExprValue {
            slot: dest,
            ty: ScalarType::Integer,
        })
    }

    /// `entier(E)` — ALGOL 60 §3.2.5: the largest **integer** not greater than
    /// the **real** `E` (i.e. floor, rounding toward −∞).  `entier(2.7) = 2`,
    /// `entier(-2.7) = -3` (NOT `-2` — a plain truncation toward zero would be
    /// wrong here, which is exactly why E8 provides a distinct
    /// `real_to_int_floor` op alongside `real_to_int_trunc`).
    ///
    /// This is the canonical use of the E8 conversion family: a single
    /// `real_to_int_floor` whose result type is `integer`.  Unlike `abs`/`sign`
    /// (which synthesise a conditional), `entier` is one IIR op — the floor and
    /// the real→integer narrowing are fused into the primitive, so every backend
    /// emits its native floor-then-convert (`llvm.floor`+`fptosi`, `f64.floor`+
    /// `i64.trunc_sat`, `Math.floor`+`d2l`, `Math::Floor`+`conv.ovf.i4`,
    /// `frintm`+`fcvtzs`, `roundsd`+`cvttsd2si`).
    ///
    /// Integer operands widen to real before flooring, which preserves
    /// ALGOL's arithmetic coercion rule (`entier(7)` is 7). A user `integer
    /// procedure entier` still wins, because `proc_sigs` is consulted before
    /// this fallback in `emit_call_common`.
    fn emit_entier(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        let actuals = self.standard_fn_actuals(node);
        if actuals.len() != 1 {
            return Err(CompileError::Type(format!(
                "standard function entier expects 1 argument, got {}",
                actuals.len()
            )));
        }
        let value = self.emit_expr(actuals[0])?;
        let value = self.coerce_value(value, ScalarType::Real, "standard function entier")?;

        // result := floor(E), narrowed to an integer.  `real_to_int_floor`'s
        // `type_hint` is the *result* type (`integer`/`i64`), matching the E8
        // convention that a backend sizes the op from the hint.
        let dest = self.fresh_temp();
        self.emit(IIRInstr::new(
            "real_to_int_floor",
            Some(dest.clone()),
            vec![Operand::Var(value.slot)],
            ScalarType::Integer.iir(),
        ));
        Ok(ExprValue {
            slot: dest,
            ty: ScalarType::Integer,
        })
    }

    /// `sqrt(E)` — ALGOL 60 §3.2.4 square root. Integer operands widen to
    /// `real` before the portable `f64_sqrt` IIR op, which every backend maps
    /// to its native hardware square-root instruction (aarch64 `fsqrt`, SSE2
    /// `sqrtsd`, WASM `f64.sqrt`, LLVM `@llvm.sqrt.f64`, JVM `Math.sqrt`, CLR
    /// `Math.Sqrt`).
    ///
    /// ```text
    ///   t := E          ; evaluate the operand once (real)
    ///   dest := f64_sqrt t
    /// ```
    fn emit_sqrt(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        let actuals = self.standard_fn_actuals(node);
        if actuals.len() != 1 {
            return Err(CompileError::Type(format!(
                "standard function sqrt expects 1 argument, got {}",
                actuals.len()
            )));
        }
        let value = self.emit_expr(actuals[0])?;
        let value = self.coerce_value(value, ScalarType::Real, "standard function sqrt")?;

        let dest = self.fresh_temp();
        self.emit(IIRInstr::new(
            "f64_sqrt",
            Some(dest.clone()),
            vec![Operand::Var(value.slot)],
            ScalarType::Real.iir(),
        ));
        Ok(ExprValue {
            slot: dest,
            ty: ScalarType::Real,
        })
    }

    /// Generic `real → real` standard function backed by a single IIR op.
    ///
    /// Used for `sin`/`cos`/`ln`/`exp` — each takes one `real` argument and
    /// returns a `real` result.  The frontend name (`fn_name`) is the ALGOL
    /// identifier; `op` is the IIR opcode (e.g. `"f64_sin"`).
    ///
    /// ```text
    ///   t    := E         ; evaluate the argument once (integer inputs widen)
    ///   dest := <op> t
    /// ```
    fn emit_f64_unary(
        &mut self,
        fn_name: &str,
        op: &str,
        node: &GrammarASTNode,
    ) -> Result<ExprValue, CompileError> {
        let actuals = self.standard_fn_actuals(node);
        if actuals.len() != 1 {
            return Err(CompileError::Type(format!(
                "standard function {fn_name} expects 1 argument, got {}",
                actuals.len()
            )));
        }
        let value = self.emit_expr(actuals[0])?;
        let value = self.coerce_value(
            value,
            ScalarType::Real,
            &format!("standard function {fn_name}"),
        )?;
        let dest = self.fresh_temp();
        self.emit(IIRInstr::new(
            op,
            Some(dest.clone()),
            vec![Operand::Var(value.slot)],
            ScalarType::Real.iir(),
        ));
        Ok(ExprValue {
            slot: dest,
            ty: ScalarType::Real,
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

        if let Some(literal) = expr_string_literal(expr) {
            let mut saw_string_target = false;
            for left in &left_parts {
                let var_node = first_direct_node(left, "variable")
                    .ok_or_else(|| CompileError::Malformed("left_part has no variable".into()))?;
                if array_subscripts(var_node).is_some() {
                    let (binding, zero) = self.resolve_array_index(var_node)?;
                    if binding.ty != ScalarType::String {
                        return Err(CompileError::Type(format!(
                            "cannot assign string expression to {} array element",
                            binding.ty.name()
                        )));
                    }
                    let value = self.fresh_temp();
                    self.emit(IIRInstr::new(
                        "str_const",
                        Some(value.clone()),
                        vec![Operand::Str(literal.clone())],
                        "str",
                    ));
                    self.emit(IIRInstr::new(
                        "array_set",
                        None,
                        vec![
                            Operand::Var(binding.slot),
                            Operand::Var(zero),
                            Operand::Var(value),
                        ],
                        "str",
                    ));
                    saw_string_target = true;
                    continue;
                }
                let name = self.simple_variable_name(var_node)?;
                let binding = self.require_var(&name)?;
                if binding.ty != ScalarType::String {
                    return Err(CompileError::Type(format!(
                        "cannot assign string expression to {} variable {name:?}",
                        binding.ty.name()
                    )));
                }
                saw_string_target = true;
                let dest = if binding.is_global {
                    self.fresh_temp()
                } else {
                    binding.slot.clone()
                };
                self.emit(IIRInstr::new(
                    "str_const",
                    Some(dest.clone()),
                    vec![Operand::Str(literal.clone())],
                    "str",
                ));
                if binding.is_global {
                    self.emit(IIRInstr::new(
                        "global_store",
                        None,
                        vec![Operand::Str(binding.slot.clone()), Operand::Var(dest)],
                        "void",
                    ));
                }
                self.initialized_string_slots.insert(binding.slot.clone());
            }
            if saw_string_target {
                return Ok(());
            }
        } else if let Some(src_name) = expr_variable_name(expr) {
            let src_binding = self.require_var(&src_name)?;
            if src_binding.ty == ScalarType::String {
                if !src_binding.is_global
                    && !self.initialized_string_slots.contains(&src_binding.slot)
                {
                    return Err(CompileError::Unsupported(format!(
                        "string assignment requires initialized string variable {src_name:?}"
                    )));
                }
                let src_slot = self.read_scalar(src_binding).slot;
                let mut saw_string_target = false;
                for left in &left_parts {
                    let var_node = first_direct_node(left, "variable").ok_or_else(|| {
                        CompileError::Malformed("left_part has no variable".into())
                    })?;
                    if array_subscripts(var_node).is_some() {
                        let (binding, zero) = self.resolve_array_index(var_node)?;
                        if binding.ty != ScalarType::String {
                            return Err(CompileError::Type(format!(
                                "cannot assign string expression to {} array element",
                                binding.ty.name()
                            )));
                        }
                        self.emit(IIRInstr::new(
                            "array_set",
                            None,
                            vec![
                                Operand::Var(binding.slot),
                                Operand::Var(zero),
                                Operand::Var(src_slot.clone()),
                            ],
                            "str",
                        ));
                        saw_string_target = true;
                        continue;
                    }
                    let name = self.simple_variable_name(var_node)?;
                    let binding = self.require_var(&name)?;
                    let target_ty = binding.ty;
                    let target_slot = binding.slot.clone();
                    let target_is_global = binding.is_global;
                    if target_ty != ScalarType::String {
                        return Err(CompileError::Type(format!(
                            "cannot assign string expression to {} variable {name:?}",
                            target_ty.name()
                        )));
                    }
                    saw_string_target = true;
                    if target_is_global || target_slot != src_slot {
                        let empty = self.fresh_temp();
                        let copy = if target_is_global {
                            self.fresh_temp()
                        } else {
                            target_slot.clone()
                        };
                        self.emit(IIRInstr::new(
                            "str_const",
                            Some(empty.clone()),
                            vec![Operand::Str(String::new())],
                            "str",
                        ));
                        self.emit(IIRInstr::new(
                            "str_concat",
                            Some(copy.clone()),
                            vec![Operand::Var(src_slot.clone()), Operand::Var(empty)],
                            "str",
                        ));
                        if target_is_global {
                            self.emit(IIRInstr::new(
                                "global_store",
                                None,
                                vec![Operand::Str(target_slot.clone()), Operand::Var(copy)],
                                "void",
                            ));
                        }
                    }
                    self.initialized_string_slots.insert(target_slot.clone());
                }
                if saw_string_target {
                    return Ok(());
                }
            }
        }

        let rhs = self.emit_expr(expr)?;

        for left in left_parts {
            let var_node = first_direct_node(left, "variable")
                .ok_or_else(|| CompileError::Malformed("left_part has no variable".into()))?;

            // `A[i] := e` stores into an array element (E5); `x := e` is a mov.
            if array_subscripts(var_node).is_some() {
                let (binding, zero) = self.resolve_array_index(var_node)?;
                let value =
                    self.coerce_value(rhs.clone(), binding.ty, "array element assignment")?;
                self.emit(IIRInstr::new(
                    "array_set",
                    None,
                    vec![
                        Operand::Var(binding.slot),
                        Operand::Var(zero),
                        Operand::Var(value.slot),
                    ],
                    binding.ty.iir(),
                ));
                continue;
            }

            let name = self.simple_variable_name(var_node)?;
            let binding = self.require_var(&name)?;
            let value =
                self.coerce_value(rhs.clone(), binding.ty, &format!("assignment to {name:?}"))?;
            if binding.ty == ScalarType::String {
                if binding.is_global || binding.slot != value.slot {
                    let empty = self.fresh_temp();
                    let copy = if binding.is_global {
                        self.fresh_temp()
                    } else {
                        binding.slot.clone()
                    };
                    self.emit(IIRInstr::new(
                        "str_const",
                        Some(empty.clone()),
                        vec![Operand::Str(String::new())],
                        "str",
                    ));
                    self.emit(IIRInstr::new(
                        "str_concat",
                        Some(copy.clone()),
                        vec![Operand::Var(value.slot), Operand::Var(empty)],
                        "str",
                    ));
                    if binding.is_global {
                        self.emit(IIRInstr::new(
                            "global_store",
                            None,
                            vec![Operand::Str(binding.slot.clone()), Operand::Var(copy)],
                            "void",
                        ));
                    }
                }
                self.initialized_string_slots.insert(binding.slot.clone());
                continue;
            }
            if binding.is_global {
                // E6: a captured block scalar is a module global.
                self.emit(IIRInstr::new(
                    "global_store",
                    None,
                    vec![Operand::Str(binding.slot), Operand::Var(value.slot)],
                    "void",
                ));
            } else {
                self.emit(IIRInstr::new(
                    "mov",
                    Some(binding.slot),
                    vec![Operand::Var(value.slot)],
                    binding.ty.iir(),
                ));
            }
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
            let end_label = self.fresh_label("desig_end");
            self.emit(IIRInstr::new(
                "jmp_if_false",
                None,
                vec![Operand::Var(cond.slot), Operand::Var(else_label.clone())],
                "void",
            ));
            self.emit_simple_desig_jump(then_node)?;
            self.emit(IIRInstr::new(
                "jmp",
                None,
                vec![Operand::Var(end_label.clone())],
                "void",
            ));
            self.emit_label(&else_label);
            self.emit_desig_jump(else_node)?;
            self.emit_label(&end_label);
            Ok(())
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
            let targets = self.switches.get(&name).cloned().ok_or_else(|| {
                CompileError::Type(format!("goto uses undeclared switch {name:?}"))
            })?;
            let index = self.emit_expr(index_node)?;
            if index.ty != ScalarType::Integer {
                return Err(CompileError::Type(
                    "switch subscript index must be an integer".into(),
                ));
            }
            self.switch_expansion_steps = self
                .switch_expansion_steps
                .checked_add(targets.len())
                .ok_or_else(|| {
                    CompileError::Unsupported("switch designator expansion is too large".into())
                })?;
            if self.switch_expansion_steps > MAX_SWITCH_DESIGNATOR_EXPANSIONS {
                return Err(CompileError::Unsupported(format!(
                    "switch designator expansion exceeds {MAX_SWITCH_DESIGNATOR_EXPANSIONS} arms"
                )));
            }
            if !self.resolving_switches.insert(name.clone()) {
                return Err(CompileError::Type(format!(
                    "cyclic switch-list element involving {name:?}"
                )));
            }
            let result = (|| {
                // 1-based: `goto s[k]` selects the k-th designator. An
                // out-of-range index matches no arm and falls through. Each
                // matched arm jumps to `switch_done` after its designator so
                // a nested switch with an out-of-range subscript cannot fall
                // through and test this outer switch's next arm.
                let done_label = self.fresh_label("switch_done");
                for (i, target) in targets.iter().enumerate() {
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
                    self.emit_desig_jump(target)?;
                    self.emit(IIRInstr::new(
                        "jmp",
                        None,
                        vec![Operand::Var(done_label.clone())],
                        "void",
                    ));
                    self.emit_label(&next_label);
                }
                self.emit_label(&done_label);
                Ok(())
            })();
            self.resolving_switches.remove(&name);
            result
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

    /// Record a switch declaration's ordered designational expressions.
    ///
    /// `switch_decl = "switch" NAME ASSIGN switch_list` and
    /// `switch_list = desig_expr { COMMA desig_expr }`. The expressions are
    /// retained until a `goto switch[index]` selects one, preserving ALGOL's
    /// run-time conditional and nested-switch semantics.
    fn register_switch(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        self.set_loc(node);
        let name = direct_tokens(node)
            .into_iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CompileError::Malformed("switch_decl missing name".into()))?;
        let switch_list = first_direct_node(node, "switch_list")
            .ok_or_else(|| CompileError::Malformed("switch_decl missing switch_list".into()))?;

        let targets: Vec<GrammarASTNode> = direct_nodes(switch_list)
            .into_iter()
            .filter(|n| n.rule_name == "desig_expr")
            .cloned()
            .collect();
        if targets.is_empty() {
            return Err(CompileError::Malformed("switch has no targets".into()));
        }
        if self.switches.contains_key(&name) {
            return Err(CompileError::Type(format!(
                "duplicate declaration for switch {name:?}"
            )));
        }
        self.switches.insert(name, targets);
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
            // The guard compares **integer** operands (step vs 0), so the
            // comparison's `type_hint` is the *operand* width `i64` — not the
            // boolean *result* type. A code-gen backend (LLVM `lower_cmp`) reads
            // this hint as the `icmp` operand type, so `"bool"` would emit the
            // invalid `icmp i1 <i64>, <i64>`. This mirrors the regular relational
            // path, which already tags the cmp with `lhs.ty.iir()`.
            "i64",
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
            "i64", // operand width (loop var vs limit, both integer) — see above
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
            "i64", // operand width (loop var vs limit, both integer) — see above
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

    /// Read a scalar binding into an `ExprValue`.  A captured **global** (E6) is
    /// fetched with `global_load` into a fresh temp; a plain scalar's register
    /// slot is returned directly.
    fn read_scalar(&mut self, binding: VarBinding) -> ExprValue {
        if binding.is_global {
            let dest = self.fresh_temp();
            self.emit(IIRInstr::new(
                "global_load",
                Some(dest.clone()),
                vec![Operand::Str(binding.slot.clone())],
                binding.ty.iir(),
            ));
            ExprValue {
                slot: dest,
                ty: binding.ty,
            }
        } else {
            ExprValue {
                slot: binding.slot,
                ty: binding.ty,
            }
        }
    }

    fn emit_expr(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        self.set_loc(node);

        if direct_tokens(node).iter().any(|t| t.value == "if") {
            return self.emit_conditional_expr(node);
        }

        match node.rule_name.as_str() {
            "variable" => {
                // `A[i]` reads an array element (E5); a bare `x` reads a scalar.
                if array_subscripts(node).is_some() {
                    return self.emit_array_read(node);
                }
                let name = self.simple_variable_name(node)?;
                let binding = self.require_var(&name)?;
                if binding.ty == ScalarType::String
                    && !binding.is_global
                    && !self.initialized_string_slots.contains(&binding.slot)
                {
                    return Err(CompileError::Unsupported(format!(
                        "string variable {name:?} is read before it is initialized"
                    )));
                }
                Ok(self.read_scalar(binding))
            }
            "proc_call" => self.emit_proc_call(node),
            "expression" | "arith_expr" | "bool_expr" => self.emit_single_child_expr(node),
            "expr_eqv" | "expr_impl" | "expr_or" | "expr_and" | "simple_bool" | "implication"
            | "bool_term" | "bool_factor" => self.emit_bool_wrapper(node),
            "expr_not" | "bool_secondary" => self.emit_not_or_child(node),
            "expr_cmp" | "relation" => self.emit_binary_or_child(node, BinaryFamily::Comparison),
            "expr_add" | "simple_arith" => self.emit_binary_or_child(node, BinaryFamily::Additive),
            "expr_mul" | "term" => self.emit_binary_or_child(node, BinaryFamily::Multiplicative),
            "expr_pow" | "factor" => self.emit_pow(node),
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
            ("STRING_LIT", _) => {
                let slot = self.fresh_temp();
                self.emit(IIRInstr::new(
                    "str_const",
                    Some(slot.clone()),
                    vec![Operand::Str(unquote_algol_string(&token.value))],
                    "str",
                ));
                Ok(ExprValue {
                    slot,
                    ty: ScalarType::String,
                })
            }
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

    /// Lower ALGOL 60's exponentiation operator `↑` (§3.3.4; spelled `^` or `**`
    /// in our grammar) — LANG-FULL **AL-pow**.
    ///
    /// The `factor` / `expr_pow` node is `base [ ^ exp [ ^ exp … ] ]`.  With no
    /// `^` operator it is a plain pass-through to the single child.  Otherwise we
    /// fold left-to-right, raising the accumulator to each successive exponent.
    ///
    /// Two exponent shapes are in this slice, both reusing IIR the code-gen
    /// backends already run (no new op):
    ///
    /// | exponent            | lowering                          | result type |
    /// |---------------------|-----------------------------------|-------------|
    /// | nonneg integer literal `k` | `k−1` repeated `mul`s (`x*x*…`); `x↑0 = 1` | **base's type** — `integer↑k` stays `integer`, `real↑k` stays `real` |
    /// | numeric pair containing a real | `f64_pow` after integer→real widening (libm `pow`) | `real` |
    ///
    /// The integer-literal path keeps ALGOL's typing (`2 ↑ 10` = the *integer*
    /// 1024), unlike BASIC which always widens to `real`. A non-literal integer
    /// exponent on an integer base, or a negative literal, remains a clean
    /// `Unsupported`; any pair containing a real takes the `f64_pow` path.
    fn emit_pow(&mut self, node: &GrammarASTNode) -> Result<ExprValue, CompileError> {
        let seq = pieces(node);
        let has_pow = seq
            .iter()
            .any(|p| matches!(p, Piece::Op(op) if op == "^" || op == "**"));
        if !has_pow {
            return self.emit_single_child_expr(node);
        }

        let mut idx = 0;
        let first = match seq.first() {
            Some(Piece::Node(n)) => *n,
            _ => {
                return Err(CompileError::Malformed(
                    "exponentiation missing a base expression".into(),
                ))
            }
        };
        idx += 1;
        let mut acc = self.emit_expr(first)?;

        while idx < seq.len() {
            match seq.get(idx) {
                Some(Piece::Op(op)) if op == "^" || op == "**" => {}
                _ => {
                    return Err(CompileError::Malformed(
                        "exponentiation expected `^` between operands".into(),
                    ))
                }
            }
            idx += 1;
            let exp_node = match seq.get(idx) {
                Some(Piece::Node(n)) => *n,
                _ => {
                    return Err(CompileError::Malformed(
                        "exponentiation missing an exponent expression".into(),
                    ))
                }
            };
            idx += 1;
            acc = self.emit_power_step(acc, exp_node)?;
        }
        Ok(acc)
    }

    /// Raise `base` to a single exponent expression (see [`emit_pow`]).
    fn emit_power_step(
        &mut self,
        base: ExprValue,
        exp_node: &GrammarASTNode,
    ) -> Result<ExprValue, CompileError> {
        // Fast path: a bare nonnegative integer literal exponent unrolls to
        // repeated multiplication, preserving the base's numeric type.
        if let Some(k) = literal_nonneg_integer_exponent(exp_node) {
            return Ok(self.emit_pow_unroll(base, k));
        }

        // General path: any numeric pair containing a real uses `f64_pow`.
        let exp = self.emit_expr(exp_node)?;
        if matches!(base.ty, ScalarType::Integer | ScalarType::Real)
            && matches!(exp.ty, ScalarType::Integer | ScalarType::Real)
            && (base.ty == ScalarType::Real || exp.ty == ScalarType::Real)
        {
            let base = self.coerce_value(base, ScalarType::Real, "exponentiation base")?;
            let exp = self.coerce_value(exp, ScalarType::Real, "exponentiation exponent")?;
            let dest = self.fresh_temp();
            self.emit(IIRInstr::new(
                "f64_pow",
                Some(dest.clone()),
                vec![Operand::Var(base.slot), Operand::Var(exp.slot)],
                "f64",
            ));
            return Ok(ExprValue {
                slot: dest,
                ty: ScalarType::Real,
            });
        }

        Err(CompileError::Unsupported(format!(
            "exponentiation with a {} base and a {} exponent — this slice supports a \
             nonnegative integer-literal exponent (any base) or a numeric pair containing a real",
            base.ty.name(),
            exp.ty.name()
        )))
    }

    /// Unroll `base ↑ k` (compile-time `k ≥ 0`) into `k − 1` multiplies,
    /// preserving the base's type.  `base ↑ 0` is the type-appropriate `1`.
    fn emit_pow_unroll(&mut self, base: ExprValue, k: u32) -> ExprValue {
        let ty = base.ty;
        if k == 0 {
            let one = match ty {
                ScalarType::Real => self.emit_const(ScalarType::Real, Operand::Float(1.0)),
                _ => self.emit_const(ScalarType::Integer, Operand::Int(1)),
            };
            return ExprValue { slot: one, ty };
        }
        let base_slot = base.slot.clone();
        let mut acc = base.slot;
        for _ in 1..k {
            let dest = self.fresh_temp();
            self.emit(IIRInstr::new(
                "mul",
                Some(dest.clone()),
                vec![Operand::Var(acc), Operand::Var(base_slot.clone())],
                ty.iir(),
            ));
            acc = dest;
        }
        ExprValue { slot: acc, ty }
    }

    /// Widen an integer value to `real` with the shared IIR conversion.
    fn widen_integer_to_real(&mut self, value: ExprValue) -> ExprValue {
        debug_assert_eq!(value.ty, ScalarType::Integer);
        let dest = self.fresh_temp();
        self.emit(IIRInstr::new(
            "int_to_real",
            Some(dest.clone()),
            vec![Operand::Var(value.slot)],
            ScalarType::Real.iir(),
        ));
        ExprValue {
            slot: dest,
            ty: ScalarType::Real,
        }
    }

    /// Convert an expression only along ALGOL's numeric widening edge:
    /// `integer` to `real`. All other source/target pairs stay errors.
    fn coerce_value(
        &mut self,
        value: ExprValue,
        target: ScalarType,
        context: &str,
    ) -> Result<ExprValue, CompileError> {
        if value.ty == target {
            return Ok(value);
        }
        if value.ty == ScalarType::Integer && target == ScalarType::Real {
            return Ok(self.widen_integer_to_real(value));
        }
        Err(CompileError::Type(format!(
            "{context} cannot use {} where {} is required",
            value.ty.name(),
            target.name()
        )))
    }

    /// Promote a numeric pair to a common type. ALGOL's arithmetic widening is
    /// one-way: any `real` operand widens an `integer` peer; two integers stay
    /// integer. Boolean and string operands remain invalid for numeric ops.
    fn promote_numeric_pair(
        &mut self,
        op: &str,
        lhs: ExprValue,
        rhs: ExprValue,
    ) -> Result<(ExprValue, ExprValue, ScalarType), CompileError> {
        let numeric = |t: ScalarType| matches!(t, ScalarType::Integer | ScalarType::Real);
        if !numeric(lhs.ty) || !numeric(rhs.ty) {
            return Err(CompileError::Type(format!(
                "operator {op:?} requires numeric operands, got {} and {}",
                lhs.ty.name(),
                rhs.ty.name()
            )));
        }
        if lhs.ty == ScalarType::Real || rhs.ty == ScalarType::Real {
            return Ok((
                self.coerce_value(lhs, ScalarType::Real, "numeric promotion")?,
                self.coerce_value(rhs, ScalarType::Real, "numeric promotion")?,
                ScalarType::Real,
            ));
        }
        Ok((lhs, rhs, ScalarType::Integer))
    }

    fn emit_binary(
        &mut self,
        op: &str,
        lhs: ExprValue,
        rhs: ExprValue,
    ) -> Result<ExprValue, CompileError> {
        match op {
            "+" | "-" | "*" => {
                let (lhs, rhs, ty) = self.promote_numeric_pair(op, lhs, rhs)?;
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
                // Real division always yields a `real`, widening integer inputs
                // first. `div` and `mod` remain the integer-only operators.
                let (lhs, rhs, _) = self.promote_numeric_pair(op, lhs, rhs)?;
                let lhs = self.coerce_value(lhs, ScalarType::Real, "operator '/'")?;
                let rhs = self.coerce_value(rhs, ScalarType::Real, "operator '/'")?;
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
                let (lhs, rhs) = if lhs.ty != rhs.ty
                    && matches!(lhs.ty, ScalarType::Integer | ScalarType::Real)
                    && matches!(rhs.ty, ScalarType::Integer | ScalarType::Real)
                {
                    let (lhs, rhs, _) = self.promote_numeric_pair(op, lhs, rhs)?;
                    (lhs, rhs)
                } else if lhs.ty != rhs.ty {
                    return Err(CompileError::Type(format!(
                        "cannot compare {} and {}",
                        lhs.ty.name(),
                        rhs.ty.name()
                    )));
                } else {
                    (lhs, rhs)
                };
                if lhs.ty == ScalarType::String {
                    let dest = self.fresh_temp();
                    match op {
                        "=" | "!=" | "<>" => {
                            let equal = self.fresh_temp();
                            self.emit(IIRInstr::new(
                                "str_eq",
                                Some(equal.clone()),
                                vec![Operand::Var(lhs.slot), Operand::Var(rhs.slot)],
                                "i64",
                            ));
                            let zero = self.fresh_temp();
                            self.emit(IIRInstr::new(
                                "const",
                                Some(zero.clone()),
                                vec![Operand::Int(0)],
                                "i64",
                            ));
                            let cmp_op = if op == "=" { "cmp_ne" } else { "cmp_eq" };
                            self.emit(IIRInstr::new(
                                cmp_op,
                                Some(dest.clone()),
                                vec![Operand::Var(equal), Operand::Var(zero)],
                                "i64",
                            ));
                        }
                        "<" | "<=" | ">" | ">=" => {
                            let ordering = self.fresh_temp();
                            self.emit(IIRInstr::new(
                                "str_cmp",
                                Some(ordering.clone()),
                                vec![Operand::Var(lhs.slot), Operand::Var(rhs.slot)],
                                "i64",
                            ));
                            let zero = self.fresh_temp();
                            self.emit(IIRInstr::new(
                                "const",
                                Some(zero.clone()),
                                vec![Operand::Int(0)],
                                "i64",
                            ));
                            let cmp_op = match op {
                                "<" => "cmp_lt",
                                "<=" => "cmp_le",
                                ">" => "cmp_gt",
                                ">=" => "cmp_ge",
                                _ => unreachable!(),
                            };
                            self.emit(IIRInstr::new(
                                cmp_op,
                                Some(dest.clone()),
                                vec![Operand::Var(ordering), Operand::Var(zero)],
                                "i64",
                            ));
                        }
                        _ => unreachable!(),
                    }
                    return Ok(ExprValue {
                        slot: dest,
                        ty: ScalarType::Boolean,
                    });
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
        // Emit `cmp_eq bool_value, 0` with type_hint "i64" so LLVM generates
        // `icmp eq i64 <i64_load>, 0` — correct when `b` is promoted to an i64
        // alloca (written 2+ times).  The false constant uses `ScalarType::Boolean`
        // so it gets a "bool" type_hint: in WASM this makes the false-slot an i32
        // local, matching a non-promoted boolean variable's i32 register width.
        // In LLVM both Bool(false) and Int(0) render as the literal "0", so the
        // choice of ScalarType here has no effect on LLVM output.
        let false_slot = self.emit_const(ScalarType::Boolean, Operand::Bool(false));
        let dest = self.fresh_temp();
        self.emit(IIRInstr::new(
            "cmp_eq",
            Some(dest.clone()),
            vec![Operand::Var(value.slot), Operand::Var(false_slot)],
            "i64",
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
            ScalarType::Boolean | ScalarType::String => {
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

    fn declare_var(
        &mut self,
        name: &str,
        ty: ScalarType,
        is_own: bool,
    ) -> Result<String, CompileError> {
        let slot = self.scoped_slot_name(name);
        let current = self
            .scopes
            .last_mut()
            .expect("compiler always keeps a root scope");
        if current.contains_key(name) {
            return Err(CompileError::Type(format!(
                "duplicate declaration for {name:?}"
            )));
        }
        // A scalar becomes a module **global** (shared, persistent storage —
        // `global_load`/`global_store`, no register) in two cases:
        //   * E6 — it is referenced from inside a procedure body (captured),
        //     so the procedure and the enclosing block must see one cell; or
        //   * AL6 — it was declared `own`, giving it static lifetime so its
        //     value survives across every call of the enclosing block.
        // In both the slot doubles as the global's name. The slot is already
        // unique per scope (`__algol_s<N>_<name>` inside a procedure, where the
        // per-procedure `scope_counter` differs), so two procedures' `own n`
        // map to distinct globals.
        let is_global = is_own || self.block_captured.contains(name);
        current.insert(
            name.to_string(),
            VarBinding {
                slot: slot.clone(),
                ty,
                array: None,
                is_global,
            },
        );
        if !is_global {
            self.register_names.insert(slot.clone());
        }
        Ok(slot)
    }

    /// Declare an **array** variable (LANG-FULL E5).  Like `declare_var` it
    /// reserves a slot, but that slot holds the array *handle*, the binding's
    /// `ty` is the element type, and the lower-bound slot is recorded for
    /// subscript translation.  Returns the handle slot so the caller can emit
    /// the `alloc_array` that fills it.
    fn declare_array(
        &mut self,
        name: &str,
        elem_ty: ScalarType,
        dims: Vec<ArrayDim>,
        is_global: bool,
    ) -> Result<String, CompileError> {
        let slot = self.scoped_slot_name(name);
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
                ty: elem_ty,
                array: Some(ArrayInfo { dims, elem_ty }),
                is_global,
            },
        );
        if !is_global {
            self.register_names.insert(slot.clone());
        }
        Ok(slot)
    }

    fn scoped_slot_name(&self, name: &str) -> String {
        if self.scopes.len() == 1 {
            name.to_string()
        } else {
            format!("__algol_s{}_{}", self.scope_counter, name)
        }
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

/// The `arith_expr` subscripts of a `variable` node, or `None` when the
/// variable is an unsubscripted scalar.  `variable = NAME [ LBRACKET subscripts
/// RBRACKET ]`, and `subscripts = arith_expr { COMMA arith_expr }`, so a present
/// `subscripts` child means an array access (one expr per dimension).
fn array_subscripts(var_node: &GrammarASTNode) -> Option<Vec<&GrammarASTNode>> {
    let subs = first_direct_node(var_node, "subscripts")?;
    Some(
        direct_nodes(subs)
            .into_iter()
            .filter(|n| n.rule_name == "arith_expr")
            .collect(),
    )
}

/// Infer an array formal's rank from its lexical subscripted uses. ALGOL's
/// array parameter specifier does not carry bounds or a rank, but a compiled
/// IIR function needs a fixed descriptor parameter list. A formal that is only
/// forwarded or never indexed retains the established 1-D ABI; an indexed
/// formal records the exact number of source subscripts, including use by a
/// nested procedure unless that procedure shadows the formal.
fn array_formal_dimension_count(
    proc_decl: &GrammarASTNode,
    formal_name: &str,
) -> Result<usize, CompileError> {
    let mut dimensions = None;
    if let Some(body) = first_direct_node(proc_decl, "proc_body") {
        collect_array_formal_dimensions(body, formal_name, &mut dimensions)?;
    }
    Ok(dimensions.unwrap_or(1))
}

fn collect_array_formal_dimensions(
    node: &GrammarASTNode,
    formal_name: &str,
    dimensions: &mut Option<usize>,
) -> Result<(), CompileError> {
    for child in &node.children {
        let ASTNodeOrToken::Node(node) = child else {
            continue;
        };
        if node.rule_name == "procedure_decl" && procedure_local_names(node).contains(formal_name) {
            continue;
        }
        if node.rule_name == "variable" {
            let is_formal = direct_tokens(node)
                .iter()
                .filter(|token| token.effective_type_name() == "NAME")
                .map(|token| token.value.as_str())
                .eq(std::iter::once(formal_name));
            if is_formal {
                if let Some(subscripts) = array_subscripts(node) {
                    let observed = subscripts.len();
                    match dimensions {
                        Some(expected) if *expected != observed => {
                            return Err(CompileError::Type(format!(
                                "array parameter {formal_name:?} is used with both {expected} and {observed} subscripts"
                            )));
                        }
                        None => *dimensions = Some(observed),
                        _ => {}
                    }
                }
            }
        }
        collect_array_formal_dimensions(node, formal_name, dimensions)?;
    }
    Ok(())
}

/// Find array formals that a nested procedure must read from the outer
/// procedure's frame. The ordinary capture substrate is a typed module global;
/// array formals need the same treatment for their handle and full descriptor.
/// Formal and block-local declarations on the nested procedure shadow the
/// enclosing formal before its references are collected.
fn array_formals_captured_by_nested_procedures(
    proc_decl: &GrammarASTNode,
    params: &[(String, ProcedureParamType)],
) -> HashSet<String> {
    let visible: HashSet<String> = params
        .iter()
        .filter_map(|(name, ty)| matches!(ty, ProcedureParamType::Array { .. }).then_some(name.clone()))
        .collect();
    formals_captured_by_nested_procedures(proc_decl, &visible)
}

/// Scalar value formals use the same sibling-function capture model as array
/// descriptors. They are copied into a typed global before the nested procedure
/// runs, so a nested read or assignment sees the outer invocation's value.
fn scalar_formals_captured_by_nested_procedures(
    proc_decl: &GrammarASTNode,
    params: &[(String, ProcedureParamType)],
) -> HashSet<String> {
    let visible: HashSet<String> = params
        .iter()
        .filter_map(|(name, ty)| matches!(ty, ProcedureParamType::Scalar(_)).then_some(name.clone()))
        .collect();
    formals_captured_by_nested_procedures(proc_decl, &visible)
}

fn formals_captured_by_nested_procedures(
    proc_decl: &GrammarASTNode,
    visible: &HashSet<String>,
) -> HashSet<String> {
    let mut captured = HashSet::new();
    if let Some(body) = first_direct_node(proc_decl, "proc_body") {
        collect_nested_formal_captures(body, visible, &mut captured);
    }
    captured
}

fn collect_nested_formal_captures(
    node: &GrammarASTNode,
    visible: &HashSet<String>,
    captured: &mut HashSet<String>,
) {
    for child in direct_nodes(node) {
        if child.rule_name != "procedure_decl" {
            collect_nested_formal_captures(child, visible, captured);
            continue;
        }

        let mut nested_visible = visible.clone();
        for local in procedure_local_names(child) {
            nested_visible.remove(&local);
        }
        let Some(body) = first_direct_node(child, "proc_body") else {
            continue;
        };

        let mut references = HashSet::new();
        collect_name_tokens_excluding_nested_procedures(body, &mut references);
        captured.extend(nested_visible.intersection(&references).cloned());
        collect_nested_formal_captures(body, &nested_visible, captured);
    }
}

fn procedure_local_names(proc_decl: &GrammarASTNode) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Some(name) = direct_tokens(proc_decl)
        .into_iter()
        .find(|token| token.effective_type_name() == "NAME")
        .map(|token| token.value.clone())
    {
        names.insert(name);
    }
    if let Some(formals) = first_direct_node(proc_decl, "formal_params") {
        if let Some(list) = first_direct_node(formals, "ident_list") {
            names.extend(ident_list_names(list));
        }
    }
    if let Some(body) = first_direct_node(proc_decl, "proc_body") {
        if let Some(block) = first_direct_node(body, "block") {
            for declaration in direct_nodes(block)
                .into_iter()
                .filter(|node| node.rule_name == "declaration")
            {
                collect_declared_ident_list_names(declaration, &mut names);
            }
        }
    }
    names
}

fn collect_declared_ident_list_names(node: &GrammarASTNode, names: &mut HashSet<String>) {
    for child in direct_nodes(node) {
        if child.rule_name == "procedure_decl" {
            continue;
        }
        if child.rule_name == "ident_list" {
            names.extend(ident_list_names(child));
            continue;
        }
        collect_declared_ident_list_names(child, names);
    }
}

fn collect_name_tokens_excluding_nested_procedures(
    node: &GrammarASTNode,
    names: &mut HashSet<String>,
) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" => {
                names.insert(token.value.clone());
            }
            ASTNodeOrToken::Node(node) if node.rule_name != "procedure_decl" => {
                collect_name_tokens_excluding_nested_procedures(node, names);
            }
            _ => {}
        }
    }
}

/// Collect names that a nested procedure resolves outside every enclosing
/// procedure-local scope. Nested formals, result variables, and direct block
/// declarations shadow names from the block currently being analysed.
fn collect_nested_block_captures(
    node: &GrammarASTNode,
    hidden: &HashSet<String>,
    captured: &mut HashSet<String>,
) {
    for child in direct_nodes(node) {
        if child.rule_name != "procedure_decl" {
            collect_nested_block_captures(child, hidden, captured);
            continue;
        }

        let mut nested_hidden = hidden.clone();
        nested_hidden.extend(procedure_local_names(child));
        let Some(body) = first_direct_node(child, "proc_body") else {
            continue;
        };

        let mut references = HashSet::new();
        collect_name_tokens_excluding_nested_procedures(body, &mut references);
        references.retain(|name| !nested_hidden.contains(name));
        captured.extend(references);
        collect_nested_block_captures(body, &nested_hidden, captured);
    }
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

/// Largest exponent AL-pow will unroll into repeated multiplication.  Beyond
/// this the fixed-instruction expansion would bloat the IIR; a bigger exponent
/// falls through to the `real ↑ real` `f64_pow` path (or a clean `Unsupported`
/// for an integer base).  64 mirrors BASIC's BA-pow cap.
const MAX_POW_UNROLL_EXPONENT: u32 = 64;

/// Hard cap for the number of switch-list arms emitted while lowering one IIR
/// function. Nested switch designators form a graph; without this cap an
/// acyclic graph with repeated fan-out can grow exponentially during inlining.
const MAX_SWITCH_DESIGNATOR_EXPANSIONS: usize = 16_384;

/// If `node` is a **bare nonnegative integer literal** (an `INTEGER_LIT` token,
/// possibly wrapped in single-child expression nodes) no larger than
/// [`MAX_POW_UNROLL_EXPONENT`], return its value — the exponents AL-pow unrolls.
/// A `real`, negative, oversized, or non-literal exponent returns `None`.
fn literal_nonneg_integer_exponent(node: &GrammarASTNode) -> Option<u32> {
    let token = single_token_recursive(node)?;
    if token.effective_type_name() != "INTEGER_LIT" {
        return None;
    }
    let value = token.value.parse::<i64>().ok()?;
    if (0..=MAX_POW_UNROLL_EXPONENT as i64).contains(&value) {
        Some(value as u32)
    } else {
        None
    }
}

fn expr_string_literal(node: &GrammarASTNode) -> Option<String> {
    let tokens = direct_tokens(node);
    if tokens.len() == 1 && tokens[0].effective_type_name() == "STRING_LIT" {
        return Some(unquote_algol_string(&tokens[0].value));
    }

    let child_nodes = direct_nodes(node);
    if child_nodes.len() == 1 {
        return expr_string_literal(child_nodes[0]);
    }

    None
}

fn expr_variable_name(node: &GrammarASTNode) -> Option<String> {
    let tokens = direct_tokens(node);
    if tokens.len() == 1 && tokens[0].effective_type_name() == "NAME" {
        return Some(tokens[0].value.clone());
    }

    let child_nodes = direct_nodes(node);
    if child_nodes.len() == 1 {
        return expr_variable_name(child_nodes[0]);
    }

    None
}

fn unquote_algol_string(raw: &str) -> String {
    let s = raw.trim();
    if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
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

    // ── E6 (layer 1) — procedure reads/writes an enclosing-block global ──────

    /// The canonical proof: an outer-block `counter` is read+written by the
    /// procedure `add` *and* by the enclosing block, so it is materialised as a
    /// module global shared across the two functions. `add(x)` adds `x` to the
    /// shared counter and returns it; the block seeds `counter := 40`, then
    /// `result := add(2)` ⇒ 42. (`add` takes a parameter — a *parameterless*
    /// procedure call would parse as a bare variable, an orthogonal limitation.)
    const E6_PROG: &str = "begin integer counter, result; \
         integer procedure add(x); value x; integer x; \
            add := counter := counter + x; \
         counter := 40; \
         result := add(2) end";

    #[test]
    fn e6_enclosing_scalar_becomes_global() {
        let module = compile_source(E6_PROG, "test").expect("compiles");
        // `counter` is referenced inside `bump`, so it must lower to the typed
        // global ops — in BOTH `bump` and `main`, not a register mov.
        let mut loads = 0usize;
        let mut stores = 0usize;
        for f in &module.functions {
            for i in &f.instructions {
                match i.op.as_str() {
                    "global_load" => {
                        assert_eq!(i.srcs.first().and_then(|o| o.as_str_lit()), Some("counter"));
                        loads += 1;
                    }
                    "global_store" => {
                        assert_eq!(i.srcs.first().and_then(|o| o.as_str_lit()), Some("counter"));
                        stores += 1;
                    }
                    _ => {}
                }
            }
        }
        assert!(loads >= 1 && stores >= 1, "expected global_load+store, got {loads}+{stores}");
    }

    #[test]
    fn e6_procedure_shares_global_with_block_runs_on_vm() {
        // RUN it on the VM (which executes the typed global ops): ⇒ 42.
        assert_eq!(run_i64(E6_PROG), 42);
    }

    /// LANG-FULL AL6: an `own` variable inside a procedure keeps its value
    /// across calls. `bump(d)` adds `d` to its `own integer n`, so three calls
    /// accumulate: n = 1, 2, 3 and the sum is 6. A *non-`own*` local would
    /// reset to 0 each call → 1 + 1 + 1 = 3, so 6 is positive proof of static
    /// lifetime.
    const AL6_OWN_PROG: &str = "begin integer result; \
         integer procedure bump(d); value d; integer d; \
         begin own integer n; n := n + d; bump := n end; \
         result := bump(1) + bump(1) + bump(1) end";

    #[test]
    fn al6_own_variable_lowers_to_global() {
        // The `own integer n` must lower to the typed module-global ops, never
        // a per-call register — and it must NOT get a per-declaration `const`
        // re-init (that would re-zero it every call).
        let module = compile_source(AL6_OWN_PROG, "test").expect("compiles");
        let bump = module.functions.iter().find(|f| f.name == "bump").expect("bump fn");
        let own_slot = "__algol_s1_n";
        assert!(bump.instructions.iter().any(|i| i.op == "global_load"
            && i.srcs.first().and_then(|o| o.as_str_lit()) == Some(own_slot)),
            "own `n` read must be global_load {own_slot}; got: {:?}",
            bump.instructions.iter().map(|i| &i.op).collect::<Vec<_>>());
        assert!(bump.instructions.iter().any(|i| i.op == "global_store"
            && i.srcs.first().and_then(|o| o.as_str_lit()) == Some(own_slot)),
            "own `n` write must be global_store {own_slot}");
        // No `const` writes the own global (re-init each call would break it).
        assert!(!bump.instructions.iter().any(|i| i.op == "const"
            && i.dest.as_deref() == Some(own_slot)),
            "own global must not be re-zeroed by a per-call const");
    }

    #[test]
    fn al6_own_variable_persists_across_calls_runs_on_vm() {
        // RUN it: 1 + 2 + 3 = 6 (own persists); a plain local would give 3.
        assert_eq!(run_i64(AL6_OWN_PROG), 6);
    }

    /// `own` arrays share the scalar `own` lifetime rule, but need an explicit
    /// first-call allocation guard because their handle and index metadata are
    /// module globals rather than zero-initialized scalar values.
    const AL6_OWN_ARRAY_PROG: &str = "begin integer result; \
         integer procedure bump(d); value d; integer d; \
         begin own integer array memo[4:5]; memo[4] := memo[4] + d; bump := memo[4] end; \
         result := bump(1) + bump(1) + bump(1) end";

    #[test]
    fn al6_own_array_lowers_to_guarded_typed_globals() {
        let module = compile_source(AL6_OWN_ARRAY_PROG, "test").expect("compiles");
        let bump = module.functions.iter().find(|f| f.name == "bump").expect("bump fn");
        let array_slot = "__algol_s1_memo";
        let flag = "__algol_s1_memo.__algol_own_array_initialized";
        assert!(bump.instructions.iter().any(|i| i.op == "global_load"
            && i.srcs.first().and_then(|o| o.as_str_lit()) == Some(flag)),
            "own array must read its initialization flag");
        assert!(bump.instructions.iter().any(|i| i.op == "global_store"
            && i.srcs.first().and_then(|o| o.as_str_lit()) == Some(array_slot)),
            "own array allocation must store a typed handle global");
        assert!(bump.instructions.iter().any(|i| i.op == "global_store"
            && i.srcs.first().and_then(|o| o.as_str_lit()) == Some(flag)),
            "own array allocation must mark the initialization flag");
        assert!(bump.instructions.iter().any(|i| i.op == "jmp_if_false"),
            "own array initialization must be guarded");
    }

    #[test]
    fn al6_own_array_persists_across_calls_runs_on_vm() {
        // RUN it: memo[4] advances 0 → 1 → 2 → 3, so 1 + 2 + 3 = 6.
        assert_eq!(run_i64(AL6_OWN_ARRAY_PROG), 6);
    }

    /// Captured string assignments must cross a procedure boundary, and an
    /// `own string` must retain the first call's value. The latter uses the
    /// second invocation to prove it did not receive a fresh empty handle.
    const AL7_GLOBAL_STRING_PROG: &str = "begin integer result; string shared; \
         procedure setshared; shared := 'C'; \
         integer procedure remember(n); value n; integer n; \
            begin own string memo; if n = 1 then memo := 'A'; \
              if memo = 'A' then remember := 1 else remember := 0 end; \
         setshared; result := 0; \
         if shared = 'C' then result := result + 1; \
         result := result + remember(1) + remember(2) end";

    #[test]
    fn al7_captured_and_own_strings_lower_to_typed_globals() {
        let module = compile_source(AL7_GLOBAL_STRING_PROG, "test")
            .expect("captured and own strings compile");
        let setshared = module.functions.iter().find(|f| f.name == "setshared")
            .expect("setshared function");
        assert!(setshared.instructions.iter().any(|i| {
            i.op == "global_store"
                && i.srcs.first().and_then(|operand| operand.as_str_lit()) == Some("shared")
        }), "captured string assignment must be a global_store");

        let remember = module.functions.iter().find(|f| f.name == "remember")
            .expect("remember function");
        let memo = "__algol_s1_memo";
        let flag = "__algol_s1_memo.__algol_own_string_initialized";
        assert!(remember.instructions.iter().any(|i| {
            i.op == "global_load"
                && i.srcs.first().and_then(|operand| operand.as_str_lit()) == Some(flag)
        }), "own string initialization must read its persistent flag");
        assert!(remember.instructions.iter().any(|i| {
            i.op == "global_store"
                && i.srcs.first().and_then(|operand| operand.as_str_lit()) == Some(memo)
        }), "own string assignment must use a typed global_store");
    }

    #[test]
    fn al7_captured_and_own_strings_run_on_vm() {
        // captured `shared` supplies the first point; the two `remember` calls
        // prove the `own string memo` survives past its declaring procedure.
        assert_eq!(run_i64(AL7_GLOBAL_STRING_PROG), 3);
    }

    #[test]
    fn al6_two_procedures_have_independent_own() {
        // Each procedure's `own n` is a DISTINCT global (`__algol_s1_n` vs
        // `__algol_s2_n`), so they don't alias. (`z` is an unused value param
        // only because a parameterless procedure call parses as a bare variable
        // — an orthogonal frontend limitation.)
        //   a(0): n_a 0→1 ⇒ 1 ; b(0): n_b 0→10 ⇒ 10 ; a(0): n_a 1→2 ⇒ 2
        //   result = 1 + 10 + 2 = 13.
        // If the two `n`s aliased one global the calls would interleave on a
        // single cell: 1, 11, 12 ⇒ 24. So 13 proves they are independent.
        let src = "begin integer result; \
             integer procedure a(z); value z; integer z; \
                begin own integer n; n := n + 1; a := n end; \
             integer procedure b(z); value z; integer z; \
                begin own integer n; n := n + 10; b := n end; \
             result := a(0) + b(0) + a(0) end";
        assert_eq!(run_i64(src), 13);
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

    /// The `for … step … until` loop-guard comparisons must carry the **operand**
    /// type (`i64`), not the boolean result type — a code-gen backend reads the
    /// `type_hint` as the comparison operand width, so `"bool"` produced the
    /// invalid `icmp i1 <i64>, <i64>` that broke ALGOL `for` loops on LLVM.
    #[test]
    fn for_loop_guard_compares_at_operand_width_not_bool() {
        let src = "begin integer i, result; result := 0; for i := 1 step 1 until 5 do result := result + i end";
        let module = compile_source(src, "test").unwrap();
        let main = &module.functions[0];
        let guard_cmps: Vec<&IIRInstr> = main.instructions.iter()
            .filter(|i| matches!(i.op.as_str(), "cmp_le" | "cmp_ge"))
            .collect();
        assert!(!guard_cmps.is_empty(), "the for-loop emits step-sign and bound guards");
        for c in guard_cmps {
            assert_eq!(c.type_hint, "i64",
                "guard {:?} must compare at i64 (its integer operands), not {:?}", c.op, c.type_hint);
        }
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

    // ── AL4 — literal string output through E4 ──────────────────────────────

    #[test]
    fn al4_print_string_literal_lowers_to_e4_ops() {
        let module = compile_source("begin print('HI') end", "test").expect("print compiles");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        let str_dest = main
            .instructions
            .iter()
            .find(|i| {
                i.op == "str_const"
                    && matches!(i.srcs.first(), Some(Operand::Str(s)) if s == "HI")
            })
            .and_then(|i| i.dest.as_deref())
            .expect("string literal should lower to str_const");
        assert!(
            main.instructions.iter().any(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(v)) if v == str_dest)
            }),
            "print_str should consume the literal string slot"
        );
        assert!(
            !main
                .instructions
                .iter()
                .any(|i| i.op == "call" && i.srcs.first().and_then(|o| o.as_str_lit()) == Some("print")),
            "standard print should lower inline, not to an undeclared procedure call"
        );
    }

    #[test]
    fn al4_output_string_literal_alias_lowers_to_e4_ops() {
        let module = compile_source("begin output('A', 'B') end", "test")
            .expect("output compiles");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        let literals: Vec<&str> = main
            .instructions
            .iter()
            .filter_map(|i| match (i.op.as_str(), i.srcs.first()) {
                ("str_const", Some(Operand::Str(s))) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(literals, vec!["A", "B"]);
        let prints = main
            .instructions
            .iter()
            .filter(|i| i.op == "print_str")
            .count();
        assert_eq!(prints, 2, "each output literal prints once");
    }

    #[test]
    fn al4_output_two_string_variables_lowers_to_ordered_print_str() {
        let module = compile_source(
            "begin string s, t; s := 'O'; t := 'K'; output(s, t) end",
            "test",
        )
        .expect("output variables compile");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        let s_slot = main
            .instructions
            .iter()
            .find(|i| {
                i.op == "str_const"
                    && matches!(i.srcs.first(), Some(Operand::Str(s)) if s == "O")
            })
            .and_then(|i| i.dest.as_deref())
            .expect("s literal slot");
        let t_slot = main
            .instructions
            .iter()
            .find(|i| {
                i.op == "str_const"
                    && matches!(i.srcs.first(), Some(Operand::Str(s)) if s == "K")
            })
            .and_then(|i| i.dest.as_deref())
            .expect("t literal slot");
        let print_s = main
            .instructions
            .iter()
            .position(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(v)) if v == s_slot)
            })
            .expect("output should print s");
        let print_t = main
            .instructions
            .iter()
            .position(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(v)) if v == t_slot)
            })
            .expect("output should print t");
        assert!(print_s < print_t, "output(s, t) should preserve actual order");
    }

    #[test]
    fn al4_print_numeric_argument_rejects_as_wrong_type() {
        // `print` is a string-output procedure; a numeric argument is a type
        // error. Since E4d-AL added a general string-expression path, `print(42)`
        // now evaluates the argument and rejects it by *type* (a clearer message)
        // rather than by the old literal-only shape check.
        let err = compile_source("begin print(42) end", "test")
            .expect_err("numeric print is a type error for the string-output procedure");
        assert!(
            format!("{err:?}").contains("cannot print a integer value"),
            "expected an integer-type rejection, got: {err:?}"
        );
    }

    #[test]
    fn al4_string_variable_assignment_and_print_lowers_to_direct_slot() {
        let module = compile_source("begin string s; s := 'HI'; print(s) end", "test")
            .expect("string variable compiles");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        assert!(
            main.instructions.iter().any(|i| {
                i.op == "str_const"
                    && i.dest.as_deref() == Some("s")
                    && matches!(i.srcs.first(), Some(Operand::Str(text)) if text == "HI")
            }),
            "string assignment should materialize the variable slot with str_const"
        );
        assert!(
            main.instructions.iter().any(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == "s")
            }),
            "print(s) should consume the literal-backed string slot"
        );
        assert!(
            !main.instructions.iter().any(|i| i.op == "mov" && i.type_hint == "str"),
            "literal-backed string variables must stay direct str_const slots for static backends"
        );
    }

    #[test]
    fn al4_unassigned_string_variable_print_rejects() {
        let err = compile_source("begin string s; print(s) end", "test")
            .expect_err("unassigned string variables are not initialized");
        assert!(format!("{err:?}").contains("requires initialized string variable"));
    }

    // ── E4-dyn payoff (E4d-AL): string procedures ────────────────────────────

    /// A `string procedure` returns a runtime string. Here the result is chosen
    /// by control flow (`if n > 0 then pick := 'HI' else pick := 'LO'`), so the
    /// procedure name `pick` is the dest of `str_const` in two basic blocks — a
    /// genuinely runtime string the backends carry as a handle (E4-dyn). The
    /// call site `print(pick(1))` evaluates the call and prints the result.
    const STRING_PROC_PROG: &str =
        "begin string procedure pick(n); value n; integer n; \
             if n > 0 then pick := 'HI' else pick := 'LO'; \
         print(pick(1)) end";

    const RUNTIME_STRING_LOCAL_PROG: &str =
        "begin string s; integer result; \
             string procedure pick(n); value n; integer n; \
               if n > 0 then pick := 'HI' else pick := 'LO'; \
             s := pick(1); \
             if s = 'HI' then result := 42 else result := 0; \
             print(s) end";

    const RUNTIME_STRING_ORDERING_PROG: &str =
        "begin string s; integer result; \
             string procedure pick(n); value n; integer n; \
               if n > 0 then pick := 'HI' else pick := 'LO'; \
             s := pick(1); \
             if s < 'LO' then result := 42 else result := 0; \
             print(s) end";

    #[test]
    fn string_procedure_returns_runtime_string_and_prints() {
        let module = compile_source(STRING_PROC_PROG, "test")
            .expect("string procedure compiles");

        // The procedure lowers to its own function returning `str`.
        let pick = module
            .functions
            .iter()
            .find(|f| f.name == "pick")
            .expect("string procedure `pick` is a function");
        assert_eq!(pick.return_type, "str", "pick returns a runtime string handle");
        // Its result is assigned by str_const in the two branches — a runtime
        // (branch-selected) string, exactly the E4-dyn foothold shape.
        let branch_writes = pick
            .instructions
            .iter()
            .filter(|i| i.op == "str_const" && i.dest.as_deref() == Some("pick"))
            .count();
        assert!(
            branch_writes >= 2,
            "pick's result must be assigned in both branches (str_const×2): {:?}",
            pick.instructions
        );
        // It returns the result slot as a `str`.
        assert!(
            pick.instructions.iter().any(|i| {
                i.op == "ret"
                    && i.type_hint == "str"
                    && matches!(i.srcs.first(), Some(Operand::Var(v)) if v == "pick")
            }),
            "pick must `ret str pick`: {:?}",
            pick.instructions
        );

        // main calls pick and prints the returned handle.
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        let call = main
            .instructions
            .iter()
            .find(|i| {
                i.op == "call"
                    && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "pick")
            })
            .expect("main calls pick");
        let call_dest = call.dest.clone().expect("call has a dest");
        assert_eq!(call.type_hint, "str", "the call result is a runtime string");
        assert!(
            main.instructions.iter().any(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(v)) if *v == call_dest)
            }),
            "print(pick(1)) must print the call's runtime-string result: {:?}",
            main.instructions
        );
    }

    #[test]
    fn runtime_string_procedure_result_can_fill_a_scalar_variable() {
        let module = compile_source(RUNTIME_STRING_LOCAL_PROG, "test")
            .expect("runtime string local compiles");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        let call_result = main
            .instructions
            .iter()
            .find(|i| {
                i.op == "call"
                    && matches!(i.srcs.first(), Some(Operand::Var(name)) if name == "pick")
            })
            .and_then(|i| i.dest.as_deref())
            .expect("main calls pick into a result slot");
        assert!(
            main.instructions.iter().any(|i| {
                i.op == "str_concat"
                    && i.dest.as_deref() == Some("s")
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == call_result)
            }),
            "s := pick(1) must copy the runtime result into s: {:?}",
            main.instructions
        );
        assert!(
            main.instructions.iter().any(|i| {
                i.op == "str_eq"
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == "s")
            }),
            "s = 'HI' must compare the scalar string slot: {:?}",
            main.instructions
        );
        assert!(
            main.instructions.iter().any(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == "s")
            }),
            "print(s) must consume the initialized scalar string: {:?}",
            main.instructions
        );
    }

    #[test]
    fn runtime_string_procedure_result_can_be_lexically_ordered() {
        let module = compile_source(RUNTIME_STRING_ORDERING_PROG, "test")
            .expect("runtime string ordering compiles");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        assert!(
            main.instructions.iter().any(|i| {
                i.op == "str_cmp"
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == "s")
            }),
            "s < 'LO' must compare the runtime scalar string with str_cmp: {:?}",
            main.instructions
        );
        assert!(
            main.instructions.iter().any(|i| i.op == "cmp_lt" && i.type_hint == "i64"),
            "runtime ordering must compare str_cmp's integer result against zero: {:?}",
            main.instructions
        );
    }

    #[test]
    fn al4_string_variable_copy_lowers_to_concat_with_empty_suffix() {
        let module = compile_source("begin string s, t; s := 'HI'; t := s; print(t) end", "test")
            .expect("literal-backed string variable copy compiles");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        let empty_slot = main.instructions.iter()
            .find(|i| {
                i.op == "str_const"
                    && matches!(i.srcs.first(), Some(Operand::Str(text)) if text.is_empty())
            })
            .and_then(|i| i.dest.as_deref())
            .expect("copy should materialize an empty suffix");
        assert!(
            main.instructions.iter().any(|i| {
                i.op == "str_concat"
                    && i.dest.as_deref() == Some("t")
                    && matches!(i.srcs.as_slice(), [
                        Operand::Var(left),
                        Operand::Var(right)
                    ] if left == "s" && right == empty_slot)
            }),
            "t := s should copy through E4 str_concat with an empty suffix"
        );
        assert!(
            main.instructions.iter().any(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == "t")
            }),
            "print(t) should consume the copied literal-backed string slot"
        );
    }

    #[test]
    fn al4_string_variable_copy_survives_source_reassignment() {
        let module =
            compile_source("begin string s, t; s := 'OK'; t := s; s := 'NO'; print(t) end", "test")
                .expect("literal-backed string copy snapshot compiles");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        let copy_idx = main.instructions.iter()
            .position(|i| {
                i.op == "str_concat"
                    && i.dest.as_deref() == Some("t")
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == "s")
            })
            .expect("t := s should copy through str_concat");
        let reassign_idx = main.instructions.iter()
            .position(|i| {
                i.op == "str_const"
                    && i.dest.as_deref() == Some("s")
                    && matches!(i.srcs.first(), Some(Operand::Str(text)) if text == "NO")
            })
            .expect("s should be reassigned after the copy");
        let print_idx = main.instructions.iter()
            .position(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == "t")
            })
            .expect("print(t) should consume the copied target slot");
        assert!(
            copy_idx < reassign_idx && reassign_idx < print_idx,
            "copy must be observable independently of later source reassignment"
        );
    }

    #[test]
    fn al4_unassigned_string_variable_copy_rejects() {
        let err = compile_source("begin string s, t; t := s; print(t) end", "test")
            .expect_err("unassigned string variable copies are not initialized");
        assert!(format!("{err:?}").contains("requires initialized string variable"));
    }

    #[test]
    fn al4_string_equality_lowers_to_str_eq_zero_comparisons() {
        let module = compile_source(
            "begin string s; s := 'OK'; if s = 'OK' and s != 'NO' then print('YES') else print('NO') end",
            "test",
        )
        .expect("literal-backed string equality compiles");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        assert_eq!(
            main.instructions.iter().filter(|i| i.op == "str_eq").count(),
            2,
            "equality and inequality should both lower through E4 str_eq"
        );
        assert!(
            main.instructions.iter().any(|i| i.op == "cmp_ne" && i.type_hint == "i64"),
            "string equality should compare str_eq output against zero"
        );
        assert!(
            main.instructions.iter().any(|i| i.op == "cmp_eq" && i.type_hint == "i64"),
            "string inequality should compare str_eq output against zero"
        );
    }

    #[test]
    fn al4_string_ordering_lowers_to_str_cmp_zero_comparisons() {
        let module = compile_source(
            "begin string s; s := 'ALPHA'; if s < 'BETA' and 'BETA' > s then print('OK') else print('BAD') end",
            "test",
        )
        .expect("literal-backed string ordering compiles");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        assert_eq!(
            main.instructions.iter().filter(|i| i.op == "str_cmp").count(),
            2,
            "strict ordering in both operand orders should lower through E4 str_cmp"
        );
        assert!(
            main.instructions.iter().any(|i| i.op == "cmp_lt" && i.type_hint == "i64"),
            "s < literal should compare str_cmp output against zero"
        );
        assert!(
            main.instructions.iter().any(|i| i.op == "cmp_gt" && i.type_hint == "i64"),
            "literal > s should compare str_cmp output against zero"
        );
    }

    // ── AL8 — standard functions (§3.2.4) ───────────────────────────────────

    #[test]
    fn abs_of_negative_integer_runs() {
        // |0 - 42| = 42, preserving `integer`.
        assert_eq!(run_i64("begin integer result; result := abs(0 - 42) end"), 42);
    }

    #[test]
    fn abs_of_positive_integer_is_identity() {
        // The else branch: a non-negative argument passes through unchanged.
        assert_eq!(run_i64("begin integer result; result := abs(42) end"), 42);
    }

    #[test]
    fn abs_of_zero_is_zero() {
        // Boundary: `0 < 0` is false ⇒ else branch ⇒ 0 (not `-0`).
        assert_eq!(run_i64("begin integer result; result := abs(0) end"), 0);
    }

    #[test]
    fn abs_composes_in_an_expression() {
        // `abs` is a value expression like any other — usable mid-arithmetic.
        assert_eq!(
            run_i64("begin integer result; result := 40 + abs(0 - 2) end"),
            42
        );
    }

    #[test]
    fn abs_of_negative_real_runs() {
        // Real `abs` lowers the negation to `fsub` and compares at `f64` width.
        assert_eq!(
            run_f64("begin real result; result := abs(0.0 - 3.5) end"),
            3.5
        );
    }

    #[test]
    fn abs_of_positive_real_is_identity() {
        assert_eq!(run_f64("begin real result; result := abs(3.5) end"), 3.5);
    }

    #[test]
    fn abs_lowers_to_branches_not_a_call() {
        // The built-in must NOT emit a `call abs` (there is no such procedure):
        // it lowers inline to a compare + conditional negate.
        let module = compile_source("begin integer result; result := abs(0 - 1) end", "test")
            .expect("abs compiles");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("has main");
        assert!(
            main.instructions.iter().any(|i| i.op == "cmp_lt"),
            "abs should compare the operand against zero"
        );
        assert!(
            !main
                .instructions
                .iter()
                .any(|i| i.op == "call" && i.srcs.first().and_then(|o| o.as_str_lit()) == Some("abs")),
            "abs must not lower to a procedure call"
        );
    }

    #[test]
    fn user_declared_abs_overrides_the_builtin() {
        // The Report lets a program redeclare a standard function.  A user
        // `procedure abs` returning `x + 1` must win over the built-in, so
        // `abs(41)` ⇒ 42 (the built-in would give 41).
        let src = "begin integer result; \
                   integer procedure abs(x); value x; integer x; abs := x + 1; \
                   result := abs(41) end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn abs_rejects_wrong_arity() {
        let err = compile_source("begin integer result; result := abs(1, 2) end", "test")
            .expect_err("two-argument abs is a type error");
        assert!(format!("{err:?}").contains("abs expects 1 argument"));
    }

    #[test]
    fn sign_of_positive_is_one() {
        assert_eq!(run_i64("begin integer result; result := sign(7) end"), 1);
    }

    #[test]
    fn sign_of_negative_is_minus_one() {
        // `sign` returns -1, which as an exit code wraps to 255, so we test via
        // arithmetic: 43 + sign(0 - 9) = 43 + (-1) = 42.
        assert_eq!(
            run_i64("begin integer result; result := 43 + sign(0 - 9) end"),
            42
        );
    }

    #[test]
    fn sign_of_zero_is_zero() {
        assert_eq!(run_i64("begin integer result; result := sign(0) end"), 0);
    }

    #[test]
    fn sign_of_real_is_an_integer() {
        // The operand is `real`, but `sign` yields the *integer* 1 — usable in
        // an integer context with no real→integer coercion needed.
        assert_eq!(
            run_i64("begin integer result; result := sign(2.5) end"),
            1
        );
    }

    #[test]
    fn sign_of_negative_real_is_minus_one() {
        assert_eq!(
            run_i64("begin integer result; result := 43 + sign(0.0 - 2.5) end"),
            42
        );
    }

    #[test]
    fn sign_composes_with_abs() {
        // |sign(-4)| = |-1| = 1 — two standard functions nested.
        assert_eq!(
            run_i64("begin integer result; result := 41 + abs(sign(0 - 4)) end"),
            42
        );
    }

    #[test]
    fn user_declared_sign_overrides_the_builtin() {
        let src = "begin integer result; \
                   integer procedure sign(x); value x; integer x; sign := x + 1; \
                   result := sign(41) end";
        assert_eq!(run_i64(src), 42);
    }

    // ── LANG-FULL E8: ALGOL `entier` (real → integer floor) ──────────────

    #[test]
    fn entier_floors_a_positive_real() {
        // entier(2.7) = 2 (largest integer ≤ 2.7).
        assert_eq!(run_i64("begin integer result; result := entier(2.7) end"), 2);
        assert_eq!(run_i64("begin integer result; result := entier(42.9) end"), 42);
    }

    #[test]
    fn entier_rounds_toward_minus_infinity() {
        // entier(-2.7) = -3, NOT -2 — this is the floor-vs-truncate distinction
        // that justifies a distinct `real_to_int_floor` op. 45 + entier(-2.7) =
        // 45 + (-3) = 42.
        assert_eq!(
            run_i64("begin integer result; result := 45 + entier(0.0 - 2.7) end"),
            42
        );
    }

    #[test]
    fn entier_of_an_exact_integer_real_is_that_integer() {
        // entier(42.0) = 42 (already integral).
        assert_eq!(run_i64("begin integer result; result := entier(42.0) end"), 42);
    }

    #[test]
    fn entier_emits_a_single_real_to_int_floor_op() {
        // The lowering is one IIR op, not a synthesised conditional.
        let module = compile_source("begin integer result; result := entier(2.7) end", "test")
            .expect("entier compiles");
        let n_floor = module.functions.iter()
            .flat_map(|f| &f.instructions)
            .filter(|i| i.op == "real_to_int_floor")
            .count();
        assert_eq!(n_floor, 1, "entier lowers to exactly one real_to_int_floor");
    }

    #[test]
    fn entier_widens_an_integer_argument() {
        assert_eq!(
            run_i64("begin integer result; result := entier(7) end"),
            7
        );
    }

    #[test]
    fn entier_rejects_wrong_arity() {
        let err = compile_source("begin integer result; result := entier(1.0, 2.0) end", "test")
            .expect_err("two-argument entier is a type error");
        assert!(format!("{err:?}").contains("entier expects 1 argument"));
    }

    #[test]
    fn user_declared_entier_overrides_the_builtin() {
        // A user `integer procedure entier` wins over the standard function.
        let src = "begin integer result; \
                   integer procedure entier(x); value x; integer x; entier := x + 1; \
                   result := entier(41) end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn entier_composes_with_abs() {
        // abs(entier(-2.7)) = abs(-3) = 3 → 39 + 3 = 42.
        assert_eq!(
            run_i64("begin integer result; result := 39 + abs(entier(0.0 - 2.7)) end"),
            42
        );
    }

    #[test]
    fn sign_rejects_wrong_arity() {
        let err = compile_source("begin integer result; result := sign(1, 2) end", "test")
            .expect_err("two-argument sign is a type error");
        assert!(format!("{err:?}").contains("sign expects 1 argument"));
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

    // ---- AL3: procedures with value parameters ----

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
    fn zero_argument_procedures_run_with_explicit_empty_parentheses() {
        let src = "begin integer result; integer procedure answer; answer := 42; \
                   procedure store; result := answer(); store() end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "zero_arg_procedures").expect("compiles");
        let store = module.get_function("store").expect("store function exists");
        let calls: Vec<&IIRInstr> = store
            .instructions
            .iter()
            .filter(|instr| instr.op == "call")
            .collect();
        assert_eq!(calls.len(), 1, "store calls answer once");
        assert_eq!(calls[0].srcs.len(), 1, "zero-argument call has only callee");
        assert!(matches!(calls[0].srcs.first(), Some(Operand::Var(name)) if name == "answer"));
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

    // ---- AL8: one-dimensional array descriptor value parameters ----

    /// A typed array formal preserves the caller's non-unit lower bound and
    /// aliases its element storage. The callee writes 40 and 2 through `a`, then
    /// reads them back for the typed procedure result: `sum(values)` = 42.
    const AL8_ARRAY_PARAMETER_PROG: &str = "begin integer array values[4:5]; integer result; \
         integer procedure sum(a); value a; integer array a; \
         begin a[4] := 40; a[5] := 2; sum := a[4] + a[5] end; \
         result := sum(values) end";

    #[test]
    fn one_dimensional_integer_array_parameter_runs_on_vm() {
        assert_eq!(run_i64(AL8_ARRAY_PARAMETER_PROG), 42);
    }

    #[test]
    fn array_parameter_lowers_to_handle_and_lower_bound_iir_params() {
        let module = compile_source(AL8_ARRAY_PARAMETER_PROG, "array_param")
            .expect("array parameter program compiles");
        let sum = module.get_function("sum").expect("sum function exists");
        assert_eq!(
            sum.params,
            vec![
                ("a".to_string(), "array<i64>".to_string()),
                (array_param_lower_slot("a"), "i64".to_string()),
            ],
            "array formal must receive its handle plus declared lower bound"
        );

        let main = module.get_function("main").expect("main exists");
        let call = main
            .instructions
            .iter()
            .find(|instr| instr.op == "call")
            .expect("main calls sum");
        assert_eq!(call.srcs.len(), 3, "callee + handle + lower bound");
        assert!(matches!(call.srcs.first(), Some(Operand::Var(name)) if name == "sum"));

        assert!(
            matches!(sum.instructions.last(), Some(instr)
                if instr.op == "ret"
                    && matches!(instr.srcs.first(), Some(Operand::Var(name)) if name == "sum")),
            "the implicit procedure result must stay in its local return slot"
        );
        assert!(
            !sum.instructions.iter().any(|instr| {
                instr.op == "global_store"
                    && instr.srcs.first().and_then(Operand::as_str_lit) == Some("sum")
            }),
            "the implicit procedure result must never be treated as a captured global"
        );
    }

    #[test]
    fn array_parameter_accepts_real_elements() {
        let src = "begin real array values[4:5, -2:-1]; integer result; \
                   real procedure sum(a); value a; real array a; \
                   begin a[4,-2] := 40.0; a[5,-1] := 2.0; sum := a[4,-2] + a[5,-1] end; \
                   result := entier(sum(values)) end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn array_parameter_accepts_string_elements() {
        let src = "begin string array values[4:4, -2:-1]; integer result; \
                   procedure writeok(a); value a; string array a; \
                   begin a[4,-2] := 'OK'; if a[4,-2] = 'OK' then result := 42 else result := 0 end; \
                   writeok(values) end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn array_parameter_accepts_boolean_elements() {
        let src = "begin boolean array flags[-1:0]; integer result; \
                   procedure setflags(a); value a; boolean array a; \
                   begin a[-1] := true; a[0] := false end; \
                   setflags(flags); if flags[-1] and not flags[0] then result := 42 else result := 0 end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "boolean_array_param").expect("compiles");
        let proc_ = module
            .get_function("setflags")
            .expect("boolean array procedure exists");
        assert_eq!(
            proc_.params,
            vec![
                ("a".to_string(), "array<bool>".to_string()),
                (array_param_lower_slot("a"), "i64".to_string()),
            ],
            "boolean array formal must preserve the descriptor element type"
        );
    }

    #[test]
    fn multidimensional_boolean_array_parameter_preserves_full_descriptor() {
        let src = "begin boolean array flags[-1:0, 2:3]; integer result; procedure setflags(a); value a; boolean array a; begin a[-1,2] := true; a[-1,3] := false; a[0,2] := false; a[0,3] := true end; setflags(flags); if flags[-1,2] and not flags[-1,3] and not flags[0,2] and flags[0,3] then result := 42 else result := 0 end";
        assert_eq!(run_i64(src), 42);

        let module =
            compile_source(src, "multidimensional_boolean_array_param").expect("compiles");
        let proc_ = module
            .get_function("setflags")
            .expect("boolean array procedure exists");
        assert_eq!(
            proc_.params,
            vec![
                ("a".to_string(), "array<bool>".to_string()),
                (array_param_dim_lower_slot("a", 0), "i64".to_string()),
                (array_param_stride_slot("a", 0), "i64".to_string()),
                (array_param_dim_lower_slot("a", 1), "i64".to_string()),
            ],
            "2-D boolean array formals must preserve every lower bound and row-major stride"
        );
    }

    #[test]
    fn nested_procedure_captures_three_dimensional_boolean_array_parameter() {
        let src = "begin boolean array flags[-1:0, 2:3, 5:6]; integer result; \
                   procedure setflags(a); value a; boolean array a; \
                     begin procedure populate; begin a[-1,2,5] := true; a[-1,3,6] := false; a[0,2,5] := false; a[0,3,6] := true end; \
                           populate(); if a[-1,2,5] and not a[-1,3,6] and not a[0,2,5] and a[0,3,6] then result := 42 else result := 0 end; \
                   setflags(flags) end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "three_dimensional_boolean_array_formal_capture")
            .expect("compiles");
        let setflags = module
            .get_function("setflags")
            .expect("setflags function exists");
        assert_eq!(
            setflags.params,
            vec![
                ("a".to_string(), "array<bool>".to_string()),
                (array_param_dim_lower_slot("a", 0), "i64".to_string()),
                (array_param_stride_slot("a", 0), "i64".to_string()),
                (array_param_dim_lower_slot("a", 1), "i64".to_string()),
                (array_param_stride_slot("a", 1), "i64".to_string()),
                (array_param_dim_lower_slot("a", 2), "i64".to_string()),
            ],
            "a 3-D boolean array formal must retain its typed handle and complete descriptor"
        );

        let capture_slot = array_param_capture_slot("setflags", "a");
        let populate = module
            .get_function("populate")
            .expect("populate function exists");
        for (dim_index, field) in [
            (0, "lower"),
            (0, "stride"),
            (1, "lower"),
            (1, "stride"),
            (2, "lower"),
        ] {
            assert!(
                populate.instructions.iter().any(|instr| {
                    instr.op == "global_load"
                        && instr.srcs.first().and_then(Operand::as_str_lit)
                            == Some(array_dim_global_name(&capture_slot, dim_index, field).as_str())
                        && instr.type_hint == "i64"
                }),
                "nested boolean writes must reload captured dimension {dim_index} {field}"
            );
        }
        assert!(
            populate.instructions.iter().any(|instr| {
                instr.op == "global_load"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
                    && instr.type_hint == "array<bool>"
            }),
            "nested boolean writes must reload the captured 3-D array<bool> handle"
        );
    }

    #[test]
    fn nested_procedure_captures_three_dimensional_real_array_parameter() {
        let src = "begin real array values[-1:0, 2:3, 5:6]; integer result, total; \
                   procedure setvalues(a); value a; real array a; \
                     begin procedure populate; begin a[-1,2,5] := 30.0; a[-1,3,6] := 4.0; a[0,2,5] := 6.0; a[0,3,6] := 2.0 end; \
                           populate(); total := entier(a[-1,2,5] + a[-1,3,6] + a[0,2,5] + a[0,3,6]); if total = 42 then result := 42 else result := 0 end; \
                   setvalues(values) end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "three_dimensional_real_array_formal_capture")
            .expect("compiles");
        let setvalues = module
            .get_function("setvalues")
            .expect("setvalues function exists");
        assert_eq!(
            setvalues.params,
            vec![
                ("a".to_string(), "array<f64>".to_string()),
                (array_param_dim_lower_slot("a", 0), "i64".to_string()),
                (array_param_stride_slot("a", 0), "i64".to_string()),
                (array_param_dim_lower_slot("a", 1), "i64".to_string()),
                (array_param_stride_slot("a", 1), "i64".to_string()),
                (array_param_dim_lower_slot("a", 2), "i64".to_string()),
            ],
            "a 3-D real array formal must retain its typed handle and complete descriptor"
        );

        let capture_slot = array_param_capture_slot("setvalues", "a");
        let populate = module
            .get_function("populate")
            .expect("populate function exists");
        for (dim_index, field) in [
            (0, "lower"),
            (0, "stride"),
            (1, "lower"),
            (1, "stride"),
            (2, "lower"),
        ] {
            assert!(
                populate.instructions.iter().any(|instr| {
                    instr.op == "global_load"
                        && instr.srcs.first().and_then(Operand::as_str_lit)
                            == Some(array_dim_global_name(&capture_slot, dim_index, field).as_str())
                        && instr.type_hint == "i64"
                }),
                "nested real writes must reload captured dimension {dim_index} {field}"
            );
        }
        assert!(
            populate.instructions.iter().any(|instr| {
                instr.op == "global_load"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
                    && instr.type_hint == "array<f64>"
            }),
            "nested real writes must reload the captured 3-D array<f64> handle"
        );
    }

    #[test]
    fn nested_procedure_captures_four_dimensional_integer_array_parameter() {
        let src = "begin integer array values[-1:0, 2:3, 5:6, 8:9]; integer result; \
                   procedure setvalues(a); value a; integer array a; \
                     begin procedure populate; begin a[-1,2,5,8] := 30; a[-1,3,6,9] := 4; a[0,2,5,8] := 6; a[0,3,6,9] := 2 end; \
                           populate(); if a[-1,2,5,8] + a[-1,3,6,9] + a[0,2,5,8] + a[0,3,6,9] = 42 then result := 42 else result := 0 end; \
                   setvalues(values) end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "four_dimensional_integer_array_formal_capture")
            .expect("compiles");
        let setvalues = module
            .get_function("setvalues")
            .expect("setvalues function exists");
        assert_eq!(
            setvalues.params,
            vec![
                ("a".to_string(), "array<i64>".to_string()),
                (array_param_dim_lower_slot("a", 0), "i64".to_string()),
                (array_param_stride_slot("a", 0), "i64".to_string()),
                (array_param_dim_lower_slot("a", 1), "i64".to_string()),
                (array_param_stride_slot("a", 1), "i64".to_string()),
                (array_param_dim_lower_slot("a", 2), "i64".to_string()),
                (array_param_stride_slot("a", 2), "i64".to_string()),
                (array_param_dim_lower_slot("a", 3), "i64".to_string()),
            ],
            "a 4-D integer array formal must retain its typed handle and complete descriptor"
        );

        let capture_slot = array_param_capture_slot("setvalues", "a");
        let populate = module
            .get_function("populate")
            .expect("populate function exists");
        for (dim_index, field) in [
            (0, "lower"),
            (0, "stride"),
            (1, "lower"),
            (1, "stride"),
            (2, "lower"),
            (2, "stride"),
            (3, "lower"),
        ] {
            assert!(
                populate.instructions.iter().any(|instr| {
                    instr.op == "global_load"
                        && instr.srcs.first().and_then(Operand::as_str_lit)
                            == Some(array_dim_global_name(&capture_slot, dim_index, field).as_str())
                        && instr.type_hint == "i64"
                }),
                "nested integer writes must reload captured dimension {dim_index} {field}"
            );
        }
        assert!(
            populate.instructions.iter().any(|instr| {
                instr.op == "global_load"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
                    && instr.type_hint == "array<i64>"
            }),
            "nested integer writes must reload the captured 4-D array<i64> handle"
        );
    }

    #[test]
    fn captured_array_actual_reloads_its_descriptor_for_array_parameter() {
        let src = "begin integer array values[4:5, -2:-1]; integer result; \
                   procedure seed(a); value a; integer array a; \
                     begin a[4,-2] := 40; a[5,-1] := 2 end; \
                   procedure invoke; seed(values); \
                   invoke; result := values[4,-2] + values[5,-1] end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "captured_array_param").expect("compiles");
        let invoke = module.get_function("invoke").expect("invoke function exists");
        let descriptor_loads = invoke
            .instructions
            .iter()
            .filter(|instr| instr.op == "global_load")
            .count();
        assert!(
            descriptor_loads >= 4,
            "captured actual must reload its handle, bounds, and stride, got: {:?}",
            invoke.instructions
        );
    }

    const AL8_2D_ARRAY_PARAMETER_PROG: &str = "begin integer array values[-1:0, 4:5]; integer result; \
         integer procedure fill(a); value a; integer array a; \
         begin a[-1,4] := 40; a[0,5] := 2; fill := a[-1,4] + a[0,5] end; \
         result := fill(values) end";

    #[test]
    fn two_dimensional_integer_array_parameter_runs_on_vm() {
        // The two dimensions have distinct lower bounds, so this proves the
        // descriptor crosses both lower-bound values and the row-major stride.
        assert_eq!(run_i64(AL8_2D_ARRAY_PARAMETER_PROG), 42);
    }

    #[test]
    fn two_dimensional_array_parameter_lowers_complete_descriptor() {
        let module = compile_source(AL8_2D_ARRAY_PARAMETER_PROG, "array_param_2d")
            .expect("two-dimensional array parameter program compiles");
        let fill = module.get_function("fill").expect("fill function exists");
        assert_eq!(
            fill.params,
            vec![
                ("a".to_string(), "array<i64>".to_string()),
                (array_param_lower_slot("a"), "i64".to_string()),
                (array_param_stride_slot("a", 0), "i64".to_string()),
                (array_param_dim_lower_slot("a", 1), "i64".to_string()),
            ],
            "a 2-D formal must receive its handle, both lower bounds, and outer stride"
        );

        let main = module.get_function("main").expect("main exists");
        let call = main
            .instructions
            .iter()
            .find(|instr| instr.op == "call")
            .expect("main calls fill");
        assert_eq!(call.srcs.len(), 5, "callee + 2-D array descriptor");
    }

    #[test]
    fn nested_procedure_captures_multidimensional_array_parameter() {
        let src = "begin integer array values[-1:0, 4:5]; integer result; \
                   integer procedure fill(a); value a; integer array a; \
                     begin procedure seed; begin a[-1,4] := 40; a[0,5] := 2 end; \
                           seed(); fill := a[-1,4] + a[0,5] end; \
                   result := fill(values) end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "array_formal_capture").expect("compiles");
        let capture_slot = array_param_capture_slot("fill", "a");
        let fill = module.get_function("fill").expect("fill function exists");
        assert!(
            fill.instructions.iter().any(|instr| {
                instr.op == "global_store"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
            }),
            "fill must publish its incoming array handle for the nested procedure"
        );
        let seed = module.get_function("seed").expect("seed function exists");
        assert!(
            seed.instructions.iter().any(|instr| {
                instr.op == "global_load"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
            }),
            "seed must reload its captured array handle from the shared descriptor"
        );
    }

    #[test]
    fn nested_procedure_captures_multidimensional_string_array_parameter() {
        let src = "begin string array words[-1:0, 4:5]; integer result; \
                   procedure fill(a); value a; string array a; \
                     begin procedure seed; begin a[-1,4] := 'HI'; a[-1,5] := 'NO'; a[0,4] := 'LO'; a[0,5] := 'OK' end; \
                           seed(); if a[-1,4] < a[0,4] and a[0,5] = 'OK' and a[-1,5] != 'HI' then result := 42 else result := 0 end; \
                   fill(words) end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "string_array_formal_capture").expect("compiles");
        let fill = module.get_function("fill").expect("fill function exists");
        assert_eq!(
            fill.params,
            vec![
                ("a".to_string(), "array<str>".to_string()),
                (array_param_dim_lower_slot("a", 0), "i64".to_string()),
                (array_param_stride_slot("a", 0), "i64".to_string()),
                (array_param_dim_lower_slot("a", 1), "i64".to_string()),
            ],
            "a 2-D string array formal must retain its typed handle and complete descriptor"
        );

        let capture_slot = array_param_capture_slot("fill", "a");
        let seed = module.get_function("seed").expect("seed function exists");
        assert!(
            seed.instructions.iter().any(|instr| {
                instr.op == "global_load"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
                    && instr.type_hint == "array<str>"
            }),
            "nested string writes must reload the captured array<str> handle"
        );
    }

    #[test]
    fn nested_procedure_captures_three_dimensional_string_array_parameter() {
        let src = "begin string array words[-1:0, 4:5, 7:8]; integer result; \
                   procedure fill(a); value a; string array a; \
                     begin procedure seed; begin a[-1,4,7] := 'HI'; a[-1,5,8] := 'NO'; a[0,4,7] := 'LO'; a[0,5,8] := 'OK' end; \
                           seed(); if a[-1,4,7] < a[0,4,7] and a[0,5,8] = 'OK' and a[-1,5,8] != 'HI' then result := 42 else result := 0 end; \
                   fill(words) end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "three_dimensional_string_array_formal_capture")
            .expect("compiles");
        let fill = module.get_function("fill").expect("fill function exists");
        assert_eq!(
            fill.params,
            vec![
                ("a".to_string(), "array<str>".to_string()),
                (array_param_dim_lower_slot("a", 0), "i64".to_string()),
                (array_param_stride_slot("a", 0), "i64".to_string()),
                (array_param_dim_lower_slot("a", 1), "i64".to_string()),
                (array_param_stride_slot("a", 1), "i64".to_string()),
                (array_param_dim_lower_slot("a", 2), "i64".to_string()),
            ],
            "a 3-D string array formal must retain its typed handle and complete descriptor"
        );

        let capture_slot = array_param_capture_slot("fill", "a");
        let seed = module.get_function("seed").expect("seed function exists");
        for (dim_index, field) in [
            (0, "lower"),
            (0, "stride"),
            (1, "lower"),
            (1, "stride"),
            (2, "lower"),
        ] {
            assert!(
                seed.instructions.iter().any(|instr| {
                    instr.op == "global_load"
                        && instr.srcs.first().and_then(Operand::as_str_lit)
                            == Some(array_dim_global_name(&capture_slot, dim_index, field).as_str())
                        && instr.type_hint == "i64"
                }),
                "nested string writes must reload captured dimension {dim_index} {field}"
            );
        }
        assert!(
            seed.instructions.iter().any(|instr| {
                instr.op == "global_load"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
                    && instr.type_hint == "array<str>"
            }),
            "nested string writes must reload the captured 3-D array<str> handle"
        );
    }

    #[test]
    fn nested_procedure_captures_four_dimensional_string_array_parameter() {
        let src = "begin string array words[-1:0, 4:5, 7:8, 10:11]; integer result; \
                   procedure fill(a); value a; string array a; \
                     begin procedure seed; begin a[-1,4,7,10] := 'HI'; a[-1,5,8,11] := 'NO'; a[0,4,7,10] := 'LO'; a[0,5,8,11] := 'OK' end; \
                           seed(); if a[-1,4,7,10] < a[0,4,7,10] and a[0,5,8,11] = 'OK' and a[-1,5,8,11] != 'HI' then result := 42 else result := 0 end; \
                   fill(words) end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "four_dimensional_string_array_formal_capture")
            .expect("compiles");
        let fill = module.get_function("fill").expect("fill function exists");
        assert_eq!(
            fill.params,
            vec![
                ("a".to_string(), "array<str>".to_string()),
                (array_param_dim_lower_slot("a", 0), "i64".to_string()),
                (array_param_stride_slot("a", 0), "i64".to_string()),
                (array_param_dim_lower_slot("a", 1), "i64".to_string()),
                (array_param_stride_slot("a", 1), "i64".to_string()),
                (array_param_dim_lower_slot("a", 2), "i64".to_string()),
                (array_param_stride_slot("a", 2), "i64".to_string()),
                (array_param_dim_lower_slot("a", 3), "i64".to_string()),
            ],
            "a 4-D string array formal must retain its typed handle and complete descriptor"
        );

        let capture_slot = array_param_capture_slot("fill", "a");
        let seed = module.get_function("seed").expect("seed function exists");
        for (dim_index, field) in [
            (0, "lower"),
            (0, "stride"),
            (1, "lower"),
            (1, "stride"),
            (2, "lower"),
            (2, "stride"),
            (3, "lower"),
        ] {
            assert!(
                seed.instructions.iter().any(|instr| {
                    instr.op == "global_load"
                        && instr.srcs.first().and_then(Operand::as_str_lit)
                            == Some(array_dim_global_name(&capture_slot, dim_index, field).as_str())
                        && instr.type_hint == "i64"
                }),
                "nested string writes must reload captured dimension {dim_index} {field}"
            );
        }
        assert!(
            seed.instructions.iter().any(|instr| {
                instr.op == "global_load"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
                    && instr.type_hint == "array<str>"
            }),
            "nested string writes must reload the captured 4-D array<str> handle"
        );
    }

    #[test]
    fn nested_procedure_forwards_captured_four_dimensional_string_array_parameter() {
        let src = "begin string array words[-1:0, 4:5, 7:8, 10:11]; integer result; \
                   procedure fill(a); value a; string array a; \
                     begin procedure seed(b); value b; string array b; begin b[-1,4,7,10] := 'HI'; b[-1,5,8,11] := 'NO'; b[0,4,7,10] := 'LO'; b[0,5,8,11] := 'OK' end; \
                           procedure invoke; seed(a); \
                           invoke(); if a[-1,4,7,10] < a[0,4,7,10] and a[0,5,8,11] = 'OK' and a[-1,5,8,11] != 'HI' then result := 42 else result := 0 end; \
                   fill(words) end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "captured_four_dimensional_string_array_forwarding")
            .expect("compiles");
        let seed = module.get_function("seed").expect("seed function exists");
        assert_eq!(
            seed.params,
            vec![
                ("b".to_string(), "array<str>".to_string()),
                (array_param_dim_lower_slot("b", 0), "i64".to_string()),
                (array_param_stride_slot("b", 0), "i64".to_string()),
                (array_param_dim_lower_slot("b", 1), "i64".to_string()),
                (array_param_stride_slot("b", 1), "i64".to_string()),
                (array_param_dim_lower_slot("b", 2), "i64".to_string()),
                (array_param_stride_slot("b", 2), "i64".to_string()),
                (array_param_dim_lower_slot("b", 3), "i64".to_string()),
            ],
            "the 4-D string callee must receive the complete descriptor"
        );

        let capture_slot = array_param_capture_slot("fill", "a");
        let invoke = module
            .get_function("invoke")
            .expect("invoke function exists");
        for (dim_index, field) in [
            (0, "lower"),
            (0, "stride"),
            (1, "lower"),
            (1, "stride"),
            (2, "lower"),
            (2, "stride"),
            (3, "lower"),
        ] {
            assert!(
                invoke.instructions.iter().any(|instr| {
                    instr.op == "global_load"
                        && instr.srcs.first().and_then(Operand::as_str_lit)
                            == Some(array_dim_global_name(&capture_slot, dim_index, field).as_str())
                        && instr.type_hint == "i64"
                }),
                "nested forwarding must reload captured dimension {dim_index} {field}"
            );
        }
        assert!(
            invoke.instructions.iter().any(|instr| {
                instr.op == "global_load"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
                    && instr.type_hint == "array<str>"
            }),
            "nested forwarding must reload the captured 4-D array<str> handle"
        );
        let forward = invoke
            .instructions
            .iter()
            .find(|instr| {
                instr.op == "call"
                    && matches!(instr.srcs.first(), Some(Operand::Var(name)) if name == "seed")
            })
            .expect("invoke forwards the captured array to seed");
        assert_eq!(
            forward.srcs.len(),
            9,
            "the forwarded 4-D descriptor needs a callee, handle, four lowers, and three strides"
        );
    }

    #[test]
    fn nested_procedure_forwards_captured_four_dimensional_real_array_parameter() {
        let src = "begin real array values[-1:0, 2:3, 5:6, 8:9]; integer result, total; \
                   procedure fill(a); value a; real array a; \
                     begin procedure seed(b); value b; real array b; begin b[-1,2,5,8] := 30.0; b[-1,3,6,9] := 4.0; b[0,2,5,8] := 6.0; b[0,3,6,9] := 2.0 end; \
                           procedure invoke; seed(a); \
                           invoke(); total := entier(a[-1,2,5,8] + a[-1,3,6,9] + a[0,2,5,8] + a[0,3,6,9]); if total = 42 then result := 42 else result := 0 end; \
                   fill(values) end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "captured_four_dimensional_real_array_forwarding")
            .expect("compiles");
        let seed = module.get_function("seed").expect("seed function exists");
        assert_eq!(
            seed.params,
            vec![
                ("b".to_string(), "array<f64>".to_string()),
                (array_param_dim_lower_slot("b", 0), "i64".to_string()),
                (array_param_stride_slot("b", 0), "i64".to_string()),
                (array_param_dim_lower_slot("b", 1), "i64".to_string()),
                (array_param_stride_slot("b", 1), "i64".to_string()),
                (array_param_dim_lower_slot("b", 2), "i64".to_string()),
                (array_param_stride_slot("b", 2), "i64".to_string()),
                (array_param_dim_lower_slot("b", 3), "i64".to_string()),
            ],
            "the 4-D real callee must receive the complete descriptor"
        );

        let capture_slot = array_param_capture_slot("fill", "a");
        let invoke = module
            .get_function("invoke")
            .expect("invoke function exists");
        for (dim_index, field) in [
            (0, "lower"),
            (0, "stride"),
            (1, "lower"),
            (1, "stride"),
            (2, "lower"),
            (2, "stride"),
            (3, "lower"),
        ] {
            assert!(
                invoke.instructions.iter().any(|instr| {
                    instr.op == "global_load"
                        && instr.srcs.first().and_then(Operand::as_str_lit)
                            == Some(array_dim_global_name(&capture_slot, dim_index, field).as_str())
                        && instr.type_hint == "i64"
                }),
                "nested forwarding must reload captured dimension {dim_index} {field}"
            );
        }
        assert!(
            invoke.instructions.iter().any(|instr| {
                instr.op == "global_load"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
                    && instr.type_hint == "array<f64>"
            }),
            "nested forwarding must reload the captured 4-D array<f64> handle"
        );
        let forward = invoke
            .instructions
            .iter()
            .find(|instr| {
                instr.op == "call"
                    && matches!(instr.srcs.first(), Some(Operand::Var(name)) if name == "seed")
            })
            .expect("invoke forwards the captured array to seed");
        assert_eq!(
            forward.srcs.len(),
            9,
            "the forwarded 4-D descriptor needs a callee, handle, four lowers, and three strides"
        );
    }

    #[test]
    fn nested_array_capture_infers_rank_from_nested_use() {
        let src = "begin integer array values[-1:0, 4:5]; integer result; \
                   integer procedure fill(a); value a; integer array a; \
                     begin procedure seed; begin a[-1,4] := 40; a[0,5] := 2 end; \
                           seed(); fill := 42 end; \
                   result := fill(values) end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "nested_array_formal_rank").expect("compiles");
        let fill = module.get_function("fill").expect("fill function exists");
        assert_eq!(fill.params.len(), 4, "2-D array descriptor parameters");
    }

    #[test]
    fn nested_array_capture_allows_sibling_formal_to_shadow_it() {
        let src = "begin integer array values[-1:0, 4:5]; integer result; \
                   integer procedure fill(a); value a; integer array a; \
                     begin procedure seed; begin a[-1,4] := 40; a[0,5] := 2 end; \
                           integer procedure shadow(a); value a; integer a; shadow := a; \
                           seed(); fill := a[-1,4] + a[0,5] end; \
                   result := fill(values) end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn nested_procedure_captures_scalar_value_parameter() {
        let src = "begin integer result; \
                   integer procedure total(seed); value seed; integer seed; \
                     begin integer procedure bump; begin seed := seed + 2; bump := seed end; \
                           total := bump() end; \
                   result := total(40) end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "scalar_formal_capture").expect("compiles");
        let capture_slot = scalar_param_capture_slot("total", "seed");
        let total = module.get_function("total").expect("total function exists");
        assert!(
            total.instructions.iter().any(|instr| {
                instr.op == "global_store"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
            }),
            "total must publish its incoming scalar parameter before the nested call"
        );
        let bump = module.get_function("bump").expect("bump function exists");
        assert!(
            bump.instructions.iter().any(|instr| {
                instr.op == "global_load"
                    && instr.srcs.first().and_then(Operand::as_str_lit)
                        == Some(capture_slot.as_str())
            }),
            "bump must reload its captured scalar parameter from shared storage"
        );
    }

    #[test]
    fn nested_formal_shadow_does_not_capture_enclosing_block_scalar() {
        let src = "begin integer seed, result; \
                   procedure invoke; \
                     begin integer procedure local(seed); value seed; integer seed; \
                           local := seed + 1; \
                           result := local(1) end; \
                   seed := 41; invoke; result := result + seed end";
        assert_eq!(run_i64(src), 43);

        let module = compile_source(src, "nested_formal_shadow").expect("compiles");
        assert!(
            module.functions.iter().flat_map(|function| &function.instructions).all(|instr| {
                !matches!(instr.op.as_str(), "global_load" | "global_store")
                    || instr.srcs.first().and_then(Operand::as_str_lit) != Some("seed")
            }),
            "a nested formal named seed must shadow, not capture, the enclosing block scalar"
        );
    }

    #[test]
    fn three_dimensional_array_parameter_runs_on_vm() {
        let src = "begin integer array values[1:2, 3:4, 5:6]; integer result; \
                   integer procedure pick(a); value a; integer array a; \
                   begin a[1,3,5] := 40; a[2,4,6] := 2; pick := a[1,3,5] + a[2,4,6] end; \
                   result := pick(values) end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn array_parameter_rejects_rank_mismatch() {
        let err = compile_source(
            "begin integer array values[1:2]; integer result; \
             integer procedure first(a); value a; integer array a; first := a[1,1]; \
             result := first(values) end",
            "array_param_rank",
        )
        .expect_err("rank-mismatched array parameter must fail");
        assert!(
            matches!(err, CompileError::Type(ref message) if message.contains("formal is 2-dimensional")),
            "expected rank diagnostic, got {err:?}"
        );
    }

    #[test]
    fn array_parameter_rejects_inconsistent_formal_subscripts() {
        let err = compile_source(
            "begin integer array values[1:2]; integer result; \
             integer procedure first(a); value a; integer array a; \
             begin first := a[1]; first := a[1,1] end; \
             result := first(values) end",
            "array_param_inconsistent_rank",
        )
        .expect_err("inconsistent array formal ranks must fail");
        assert!(
            matches!(err, CompileError::Type(ref message) if message.contains("both 1 and 2 subscripts")),
            "expected inconsistent-rank diagnostic, got {err:?}"
        );
    }

    #[test]
    fn proper_procedure_statement_mutates_enclosing_scalar() {
        let src = "begin integer result; procedure bump(d); value d; integer d; \
                   result := result + d; result := 40; bump(2) end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn proper_procedure_emits_void_function_and_no_dest_call() {
        let module = compile_source(
            "begin integer result; procedure bump(d); value d; integer d; \
             result := result + d; result := 40; bump(2) end",
            "proper_proc",
        )
        .expect("proper procedure should compile");
        let bump = module
            .get_function("bump")
            .expect("bump is a sibling function");
        assert_eq!(bump.params, vec![("d".to_string(), "i64".to_string())]);
        assert_eq!(bump.return_type, "void");
        assert!(bump.instructions.iter().any(|i| i.op == "ret_void"));

        let main = module.get_function("main").expect("main exists");
        let call = main
            .instructions
            .iter()
            .find(|i| i.op == "call")
            .expect("main calls bump");
        assert!(call.dest.is_none(), "proper procedure calls have no dest");
        assert_eq!(call.type_hint, "void");
        assert!(matches!(call.srcs.first(), Some(Operand::Var(s)) if s == "bump"));
    }

    #[test]
    fn rejects_proper_procedure_in_value_position() {
        let err = compile_source(
            "begin integer result; procedure bump(d); value d; integer d; \
             result := result + d; result := bump(2) end",
            "bad",
        )
        .expect_err("proper procedure does not yield a value");
        assert!(err.to_string().contains("no return value"));
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

    // ---- AL13: real-returning typed procedures ----

    #[test]
    fn real_procedure_runs() {
        // scale(7.0) = 7.0 × 6.0 = 42.0; entier(42.0) = 42.
        // Proves the implicit `scale` result slot is seeded as f64 and that
        // `emit_entier` accepts a real procedure call as its argument.
        let src = "begin real procedure scale(x); value x; real x; scale := x * 6.0; \
                   integer result; result := entier(scale(7.0)) end";
        assert_eq!(run_i64(src), 42);
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
    fn conditional_switch_element_evaluates_at_goto_time() {
        let src = "begin integer result, i; boolean chooseyes; \
                   switch s := if chooseyes then yes else no, fallback; \
                   chooseyes := true; i := 1; goto s[i]; \
                   yes: result := 40; goto done; no: result := 1; goto done; \
                   fallback: result := 2; done: result := result + 2 end";
        assert_eq!(run_i64(src), 42);

        let else_src = src.replace("chooseyes := true", "chooseyes := false");
        assert_eq!(run_i64(&else_src), 3);
    }

    #[test]
    fn nested_switch_element_resolves_selected_designator() {
        let src = "begin integer result, i; switch inner := yes, no; \
                   switch outer := inner[i]; i := 2; goto outer[1]; \
                   yes: result := 1; goto done; no: result := 40; \
                   done: result := result + 2 end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn rejects_cyclic_switch_list_elements() {
        let err = compile_source(
            "begin integer result; switch s := s[1]; goto s[1]; result := 0 end",
            "bad",
        )
        .expect_err("recursive switch expansion must be rejected");
        assert!(err.to_string().contains("cyclic switch"));
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
    fn mixed_integer_and_real_widens_to_real() {
        let src = "begin real r; integer result; r := 1 + 2.5; \
                   if 7 = 7.0 then result := entier(r * 12) else result := 0 end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "mixed_numeric").expect("mixed arithmetic compiles");
        let main = module.get_function("main").expect("main exists");
        assert!(
            main.instructions.iter().any(|instr| instr.op == "int_to_real"),
            "mixed arithmetic and comparison must widen through int_to_real"
        );
    }

    #[test]
    fn integer_division_widens_to_real() {
        assert_eq!(
            run_f64("begin real result; result := 7 / 2 end"),
            3.5,
            "`/` is real division even for integer operands"
        );
    }

    #[test]
    fn promotion_flows_through_real_arrays_and_parameters() {
        let src = "begin integer i, result; real r; real array a[1:1]; \
                   real procedure scale(x); value x; real x; scale := x * 6; \
                   i := 7; a[1] := i; r := i; \
                   if a[1] = i then result := entier(scale(r)) else result := 0 end";
        assert_eq!(run_i64(src), 42);
    }

    #[test]
    fn real_standard_functions_widen_integer_arguments() {
        assert_eq!(
            run_i64("begin integer result; result := entier(sqrt(49)) + entier(sin(0)) end"),
            7
        );
    }

    // ---------------------------------------------------------------------
    // LANG-FULL E5 — one-dimensional arrays (AL2)
    // ---------------------------------------------------------------------

    /// Declare, store into, and read back an element: `A[2]` round-trips.
    #[test]
    fn array_store_and_load_roundtrips() {
        let src = "begin integer array A[1:3]; integer result; \
                   A[2] := 42; result := A[2] end";
        assert_eq!(run_i64(src), 42);
    }

    /// Boolean arrays use the same bounds-aware E5 descriptor as numeric arrays
    /// while keeping their `bool` element type through the load/store path.
    #[test]
    fn boolean_array_store_and_load_roundtrips() {
        let src = "begin boolean array flags[-1:0]; integer result; \
                   flags[-1] := true; flags[0] := false; \
                   if flags[-1] and not flags[0] then result := 42 else result := 0 end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "boolean_array").expect("compiles");
        let main = module.get_function("main").expect("main exists");
        assert!(
            main.instructions.iter().any(|instr| {
                instr.op == "alloc_array" && instr.type_hint == "array<bool>"
            }),
            "boolean declaration must allocate an array<bool>"
        );
        assert!(
            main.instructions.iter().any(|instr| {
                instr.op == "array_get" && instr.type_hint == "bool"
            }),
            "boolean read must retain the bool element type"
        );
    }

    /// The 1-based ALGOL lower bound is honoured: writing `A[1]` and reading it
    /// back proves the `i - lower` translation lands on element 0, not 1.
    #[test]
    fn array_lower_bound_is_one_based() {
        let src = "begin integer array A[1:3]; integer result; \
                   A[1] := 7; A[3] := 9; result := A[1] + A[3] end";
        assert_eq!(run_i64(src), 16);
    }

    /// A non-unit lower bound (`A[5:7]`) still indexes correctly.
    #[test]
    fn array_nonunit_lower_bound() {
        let src = "begin integer array A[5:7]; integer result; \
                   A[5] := 100; A[7] := 1; result := A[5] + A[7] end";
        assert_eq!(run_i64(src), 101);
    }

    /// The headline AL2 proof: fill an array in a `for` loop, sum it in another.
    /// `A[i] := i*i` then `sum += A[i]` for i=1..5 ⇒ 1+4+9+16+25 = 55.
    #[test]
    fn array_filled_and_summed_in_loops() {
        let src = "begin integer array A[1:5]; integer i, result; \
                   for i := 1 step 1 until 5 do A[i] := i * i; \
                   result := 0; \
                   for i := 1 step 1 until 5 do result := result + A[i] end";
        assert_eq!(run_i64(src), 55);
    }

    /// Multiple names in one segment (`A, B[1:2]`) are independent arrays.
    #[test]
    fn array_segment_declares_distinct_arrays() {
        let src = "begin integer array A, B[1:2]; integer result; \
                   A[1] := 3; B[1] := 100; result := A[1] + B[1] end";
        assert_eq!(run_i64(src), 103);
    }

    /// An out-of-bounds subscript traps at run time (bounds-checked by design).
    #[test]
    fn array_out_of_bounds_subscript_traps() {
        let src = "begin integer array A[1:3]; integer result; result := A[9] end";
        let err = execute_source(src, "test").unwrap_err();
        assert!(
            matches!(err, CompileError::Runtime(_)),
            "out-of-bounds read should trap at run time, got {err:?}"
        );
    }

    /// Subscripting a scalar is a compile-time type error.
    #[test]
    fn rejects_subscripting_a_scalar() {
        let err = compile_source(
            "begin integer x, result; x := 1; result := x[1] end", "test").unwrap_err();
        assert!(matches!(err, CompileError::Type(_)),
            "subscripting a scalar should be a Type error, got {err:?}");
    }

    /// A 2-D integer array: element count is 2×2=4; write and read at [2,1].
    /// Row-major flat index for [2, 1] with bounds [1:2, 1:2]:
    ///   stride[0]=2, flat=(2−1)*2+(1−1)=2 → element at index 2.
    #[test]
    fn two_d_array_store_and_load() {
        let src = "begin integer array M[1:2, 1:2]; integer result; \
                   M[2, 1] := 42; result := M[2, 1] end";
        assert_eq!(run_i64(src), 42);
    }

    /// Four distinct cells of a 2×2 array survive independent writes.
    #[test]
    fn two_d_array_all_four_cells() {
        let src = "begin integer array M[1:2, 1:2]; integer result; \
                   M[1,1] := 10; M[1,2] := 20; M[2,1] := 5; M[2,2] := 7; \
                   result := M[1,1] + M[1,2] + M[2,1] + M[2,2] end";
        assert_eq!(run_i64(src), 42);
    }

    /// A 2×3 array proves non-square shapes and the stride=3 calculation.
    /// flat index for [i,j] (bounds [1:2, 1:3]): (i−1)*3 + (j−1).
    #[test]
    fn two_d_array_non_square() {
        // M[1,1]=1  M[1,2]=4  M[1,3]=9
        // M[2,1]=2  M[2,2]=8  M[2,3]=18  → sum = 42
        let src = "begin integer array M[1:2, 1:3]; integer result; \
                   M[1,1] := 1;  M[1,2] := 4;  M[1,3] := 9; \
                   M[2,1] := 2;  M[2,2] := 8;  M[2,3] := 18; \
                   result := M[1,1] + M[1,2] + M[1,3] + \
                             M[2,1] + M[2,2] + M[2,3] end";
        assert_eq!(run_i64(src), 42);
    }

    /// Fill a 3×3 integer array with loop indices and sum the diagonal.
    /// Diagonal: M[1,1]=1, M[2,2]=4, M[3,3]=9 → sum = 14.
    #[test]
    fn two_d_array_filled_with_loops() {
        let src = "begin integer array M[1:3, 1:3]; integer i, j, result; \
                   result := 0; \
                   for i := 1 step 1 until 3 do \
                     for j := 1 step 1 until 3 do \
                       M[i,j] := i * j; \
                   for i := 1 step 1 until 3 do \
                     result := result + M[i,i] \
                   end";
        assert_eq!(run_i64(src), 14);
    }

    /// A 3-D integer array (AL-multidim-3D): exercises the **N-dimensional**
    /// generality of the multidim code — nothing is hardcoded to 2-D.  For
    /// `M[1:2, 1:2, 1:2]` the strides are computed right-to-left:
    ///   stride[2] = 1 (elided), stride[1] = size[2] = 2,
    ///   stride[0] = size[1] * stride[1] = 2 * 2 = 4.
    /// Flat index of `M[i,j,k]` = (i−1)*4 + (j−1)*2 + (k−1).
    /// `M[2,2,2]` → 1*4 + 1*2 + 1 = flat index 7 (the last of 8 cells).
    #[test]
    fn three_d_array_store_and_load() {
        let src = "begin integer array M[1:2, 1:2, 1:2]; integer result; \
                   M[2,2,2] := 42; result := M[2,2,2] end";
        assert_eq!(run_i64(src), 42);
    }

    /// All eight cells of a 2×2×2 array are independently addressable: store the
    /// flat index into each cell via a nested triple loop, then read three
    /// corner cells whose flat indices are 0, 4, and 7 → 0 + 4 + 7 = 11.
    #[test]
    fn three_d_array_all_eight_cells() {
        let src = "begin integer array M[1:2, 1:2, 1:2]; \
                   integer i, j, k, result; \
                   for i := 1 step 1 until 2 do \
                     for j := 1 step 1 until 2 do \
                       for k := 1 step 1 until 2 do \
                         M[i,j,k] := (i-1)*4 + (j-1)*2 + (k-1); \
                   result := M[1,1,1] + M[2,1,1] + M[2,2,2] end";
        assert_eq!(run_i64(src), 11);
    }

    /// A non-cubic 3-D array `M[1:2, 1:3, 1:4]` proves the general stride
    /// product: stride[2]=1, stride[1]=4, stride[0]=3*4=12; 24 elements total.
    /// `M[2,3,4]` is the final cell: flat = 1*12 + 2*4 + 3 = 23.
    #[test]
    fn three_d_array_non_cubic() {
        let src = "begin integer array M[1:2, 1:3, 1:4]; integer result; \
                   M[1,1,1] := 100; M[2,3,4] := 42; \
                   result := M[2,3,4] end";
        assert_eq!(run_i64(src), 42);
    }

    /// A 2-D array with **arbitrary (non-1) lower bounds** per dimension
    /// (AL-multidim-bounds).  ALGOL arrays carry an explicit lower bound
    /// `[lo:hi]`, so each subscript is translated to `sub − lower` before the
    /// row-major stride is applied: `flat = Σ_d (sub[d] − lower[d]) * stride[d]`.
    /// For `M[0:1, 2:4]` the sizes are `(2, 3)`, strides `[3, 1]`, and the flat
    /// index of `M[i,j]` is `(i−0)*3 + (j−2)`.  `M[1,4]` is the last cell:
    /// `1*3 + 2 = 5`.  Proves the per-dimension lower-bound subtraction composes
    /// with multidim strides (the 1:N cells never exercised `lower ≠ 1`).
    #[test]
    fn two_d_array_arbitrary_lower_bounds() {
        let src = "begin integer array M[0:1, 2:4]; integer result; \
                   M[0,2] := 100; M[1,4] := 42; result := M[1,4] end";
        assert_eq!(run_i64(src), 42);
    }

    /// A 2-D array with **negative** lower bounds.  `M[-1:0, 0:1]` has sizes
    /// `(2, 2)`, strides `[2, 1]`; `M[i,j]` → `(i−(−1))*2 + (j−0) = (i+1)*2 + j`.
    /// `M[-1,0]` is flat 0, `M[0,1]` is flat `1*2 + 1 = 3`; storing 40 and 2 and
    /// summing gives 42.
    #[test]
    fn two_d_array_negative_lower_bounds() {
        let src = "begin integer array M[0-2 : 0-1, 0:1]; integer result; \
                   M[0-2, 0] := 40; M[0-1, 1] := 2; \
                   result := M[0-2, 0] + M[0-1, 1] end";
        assert_eq!(run_i64(src), 42);
    }

    /// Wrong number of subscripts for a 2-D array is a type error.
    #[test]
    fn rejects_wrong_subscript_count_for_2d_array() {
        let err = compile_source(
            "begin integer array M[1:2, 1:2]; integer result; result := M[1] end", "test")
            .unwrap_err();
        assert!(matches!(err, CompileError::Type(_)),
            "wrong subscript count should be a Type error, got {err:?}");
    }

    /// A `real` array round-trips a double, exercising the f64 element path.
    #[test]
    fn real_array_roundtrips() {
        let src = "begin real array A[1:2]; real result; \
                   A[1] := 2.5; result := A[1] end";
        assert_eq!(run_f64(src), 2.5);
    }

    /// A `string array` shares the E5 aggregate substrate with numeric arrays:
    /// literals store as `str` elements, reads feed lexical ordering, and the
    /// VM observes the selected element as a real string value.
    #[test]
    fn string_array_elements_feed_lexical_ordering() {
        let src = "begin string array words[1:2]; integer result; \
                   words[1] := 'HI'; words[2] := 'LO'; \
                   if words[1] < words[2] then result := 42 else result := 0 end";
        assert_eq!(run_i64(src), 42);

        let module = compile_source(src, "test").expect("string array program compiles");
        let main = module.get_function("main").expect("has main");
        assert!(
            main.instructions.iter().any(|instr| instr.op == "alloc_array" && instr.type_hint == "array<str>"),
            "string array declaration must retain the str element type: {:?}",
            main.instructions
        );
        assert!(
            main.instructions.iter().any(|instr| instr.op == "array_get" && instr.type_hint == "str"),
            "string array read must produce a str value: {:?}",
            main.instructions
        );
    }

    /// An initialized scalar string can populate an array element without
    /// losing its runtime handle; the read then remains a normal `str` value.
    #[test]
    fn string_array_accepts_initialized_scalar_values() {
        let src = "begin string array words[1:1]; string greeting; integer result; \
                   greeting := 'HI'; words[1] := greeting; \
                   if words[1] = greeting then result := 42 else result := 0 end";
        assert_eq!(run_i64(src), 42);
    }

    // ── AL-pow: ALGOL 60 `↑` exponentiation (spelled `^`) ───────────────────

    /// `integer ↑ integer-literal` unrolls to repeated integer multiply and
    /// keeps the `integer` type: `2 ↑ 10` = the integer 1024.
    #[test]
    fn integer_power_literal_exponent() {
        assert_eq!(run_i64("begin integer result; result := 2 ^ 10 end"), 1024);
    }

    /// The common `x ↑ 2` (square) case.
    #[test]
    fn integer_power_squared() {
        assert_eq!(run_i64("begin integer result; result := 5 ^ 2 end"), 25);
    }

    /// `x ↑ 0` is `1` (of the base's type) with no multiply emitted.
    #[test]
    fn power_of_zero_is_one() {
        assert_eq!(run_i64("begin integer result; result := 7 ^ 0 end"), 1);
    }

    /// `x ↑ 1` is `x` itself (zero multiplies).
    #[test]
    fn power_of_one_is_identity() {
        assert_eq!(run_i64("begin integer result; result := 9 ^ 1 end"), 9);
    }

    /// Exponentiation binds tighter than `*`: `3 * 2 ↑ 3` = `3 * 8` = 24.
    #[test]
    fn power_binds_tighter_than_multiply() {
        assert_eq!(run_i64("begin integer result; result := 3 * 2 ^ 3 end"), 24);
    }

    /// A `real` base with an integer-literal exponent unrolls with f64 multiply
    /// and stays `real`: `2.5 ↑ 2` = 6.25.
    #[test]
    fn real_base_integer_literal_exponent() {
        assert_eq!(run_f64("begin real result; result := 2.5 ^ 2 end"), 6.25);
    }

    /// `real ↑ real` lowers to the `f64_pow` op (libm `pow`): `2.0 ↑ 3.0` = 8.0.
    #[test]
    fn real_power_real_via_pow() {
        assert_eq!(run_f64("begin real result; result := 2.0 ^ 3.0 end"), 8.0);
    }

    /// An integer base widens when a real exponent selects the `f64_pow` path.
    #[test]
    fn integer_base_real_exponent_widens_to_real() {
        assert_eq!(
            run_f64("begin real result; result := 2 ^ 3.0 end"),
            8.0
        );
    }

    /// A 2-D **`real`** array (AL-multidim-real): the multidim flat-index path
    /// carries `f64` elements.  The `elem_ty` recorded at declaration is
    /// `Real`, so `alloc_array`/`array_set`/`array_get` ride the same 8-byte
    /// slots as 1-D real arrays — only the index computation is multidim.
    /// Store four doubles into `M[1:2, 1:2]` and read one back.
    #[test]
    fn two_d_real_array_roundtrips() {
        let src = "begin real array M[1:2, 1:2]; real result; \
                   M[1,1] := 1.5; M[1,2] := 2.5; M[2,1] := 3.5; M[2,2] := 4.5; \
                   result := M[2,2] end";
        assert_eq!(run_f64(src), 4.5);
    }

    /// A 2-D `real` array sums all four cells: 1.5+2.5+3.5+4.5 = 12.0.
    /// Proves independent cells and the f64 add path over multidim reads.
    #[test]
    fn two_d_real_array_sum() {
        let src = "begin real array M[1:2, 1:2]; real result; \
                   M[1,1] := 1.5; M[1,2] := 2.5; M[2,1] := 3.5; M[2,2] := 4.5; \
                   result := M[1,1] + M[1,2] + M[2,1] + M[2,2] end";
        assert_eq!(run_f64(src), 12.0);
    }

    // ── AL4 string procedure parameters ─────────────────────────────────────

    /// A procedure with a string value parameter compiles: `specifier_scalar_type`
    /// now returns `Ok(ScalarType::String)` for `"string"`.
    /// Body is a single `print(s)` — the integer result defaults to 0.
    #[test]
    fn al4_string_parameter_procedure_compiles() {
        // ALGOL procedure body is one statement; `begin…end` groups more than one.
        // Here `print(s)` is the whole body; the integer result is left at its
        // default 0. The call discards the return value.
        let src = "begin \
                   integer procedure msg(s); value s; string s; print(s); \
                   msg('HELLO') \
                   end";
        compile_source(src, "test").expect("string parameter procedure should compile");
    }

    /// The procedure body emits a `print_str` on the parameter slot, not on an
    /// intermediate copy — the parameter is initialized at procedure entry.
    #[test]
    fn al4_string_parameter_body_emits_print_str() {
        let src = "begin \
                   integer procedure echo(s); value s; string s; print(s); \
                   echo('HI') \
                   end";
        let module = compile_source(src, "test").expect("compiles");
        let echo_fn = module.functions.iter()
            .find(|f| f.name == "echo")
            .expect("procedure echo should exist");
        assert!(
            echo_fn.instructions.iter().any(|i| i.op == "print_str"),
            "print(s) inside echo should lower to print_str, not a dynamic call"
        );
    }

    /// The procedure's IIR parameter list carries a `str`-typed entry for the
    /// string formal.
    #[test]
    fn al4_string_parameter_has_str_iir_type() {
        let src = "begin \
                   integer procedure echo(s); value s; string s; print(s); \
                   echo('HI') \
                   end";
        let module = compile_source(src, "test").expect("compiles");
        let echo_fn = module.functions.iter()
            .find(|f| f.name == "echo")
            .expect("procedure echo should exist");
        assert!(
            echo_fn.params.iter().any(|(_, ty)| ty == "str"),
            "the string formal parameter should carry type `str` in IIRFunction::params"
        );
    }

    /// Call site emits a `call` targeting `echo` with the string-slot argument.
    #[test]
    fn al4_string_parameter_call_site_emits_call() {
        let src = "begin \
                   integer procedure echo(s); value s; string s; print(s); \
                   echo('HI') \
                   end";
        let module = compile_source(src, "test").expect("compiles");
        let main_fn = module.functions.iter().find(|f| f.name == "main").expect("main");
        assert!(
            main_fn.instructions.iter().any(|i| {
                i.op == "call"
                    && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "echo")
            }),
            "call site should emit `call echo …`"
        );
    }

    /// Passing a named string variable (not just a literal) to a string parameter.
    #[test]
    fn al4_string_variable_passed_to_string_parameter() {
        let src = "begin \
                   string greeting; \
                   integer procedure echo(s); value s; string s; print(s); \
                   greeting := 'WORLD'; \
                   echo(greeting) \
                   end";
        compile_source(src, "test").expect("named string variable as actual compiles");
    }

    /// Type mismatch: passing an integer where a string parameter is expected.
    #[test]
    fn al4_string_parameter_rejects_integer_actual() {
        let src = "begin \
                   integer procedure echo(s); value s; string s; print(s); \
                   echo(42) \
                   end";
        let err = compile_source(src, "test")
            .expect_err("integer actual to string parameter should be a type error");
        assert!(
            matches!(err, CompileError::Type(_)),
            "expected Type error, got {err:?}"
        );
    }
}
