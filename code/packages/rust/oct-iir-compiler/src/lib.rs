//! # `oct-iir-compiler` — Oct source → `IIRModule` (OCT02 phase 3).
//!
//! Bridges parsed and type-checked Oct programs into the LANG VM AOT
//! chain by emitting [`interpreter_ir::IIRModule`].  Sits between
//! [`coding_adventures_oct_parser`] + [`oct_type_checker`] and the
//! shared backend that powers Twig, Nib, Brainfuck, and Dartmouth BASIC.
//!
//! ```text
//! Oct source
//!     │
//!     ▼ oct-lexer + oct-parser            (OCT02 phase 1)
//! GrammarASTNode
//!     │
//!     ▼ oct-type-checker                  (OCT02 phase 2)
//! verified AST
//!     │
//!     ▼ oct-iir-compiler                  ← THIS CRATE (phase 3)
//! IIRModule
//!     │
//!     ▼ lang-aot                          (phase 4)
//! native executable
//! ```
//!
//! ## V1 scope
//!
//! What lowers (matches the Oct grammar):
//!
//! - Integer / bool / hex / binary literals.
//! - Arithmetic `+` `-`, bitwise `&` `|` `^`, comparisons (`==` `!=` `<` `>` `<=` `>=`),
//!   logical `&&` `||`, unary `!` `~`.
//! - `if expr block [else block]`, `while expr block`, `loop block`, `break`.
//! - User-defined `fn`s with parameters and return values.
//! - Recursion (uses the cross-function `call` reloc landed in LANG43).
//! - Local variables and `static` decls (lowered to `mov`/load-store).
//!
//! What doesn't (each fails with a clean `OctError` variant):
//!
//! - **8008 intrinsics** (`in`, `out`, `adc`, `sbb`, `rlc`, `rrc`, `ral`,
//!   `rar`, `carry`, `parity`) → `OctError::Unsupported8008Intrinsic`.
//!   The LANG VM has no port-I/O abstraction; the Intel-8008 simulator
//!   remains the right home for these programs.
//! - **Strings** → `OctError::StringsNotYetSupported`.  The Oct grammar
//!   has no string literals today, but if a future revision adds them
//!   we'll reject them until LANG77 lands.
//!
//! ## Entry-point convention
//!
//! Oct programs declare `fn main()` with void return.  The LANG VM AOT
//! chain expects `main` to return `i64` (the value becomes the process
//! exit code via the C runtime's `exit()` truncation to `& 0xFF`).  This
//! crate rewrites Oct's void `main` to return `i64 0` so the chain's
//! convention holds.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use coding_adventures_oct_parser::parse_oct;
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use interpreter_ir::source_loc::SourceLoc;
use oct_type_checker::check_ast;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

/// Extract a `SourceLoc` from a `GrammarASTNode`, falling back to
/// `SYNTHETIC` when the parser couldn't attach position info.
///
/// Used to tag every emitted IIR instruction with the source position
/// of the AST node that produced it.  The resulting `source_map`
/// powers line-based breakpoints in the debugger and source-line
/// reporting in stack traces.
fn node_loc(node: &GrammarASTNode) -> SourceLoc {
    match (node.start_line, node.start_column) {
        (Some(line), Some(col)) => SourceLoc::new(line, col),
        _ => SourceLoc::SYNTHETIC,
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Errors that can surface from the compile pipeline.
#[derive(Debug, Clone)]
pub enum OctError {
    /// Parser rejected the source (unmatched braces, malformed expr, …).
    Parse(String),
    /// Type checker rejected the program.  Carries one human-readable
    /// message per diagnostic.
    Type(Vec<String>),
    /// The program calls an 8008 hardware intrinsic that has no LANG VM
    /// equivalent.  Carries the intrinsic name (`in`, `out`, …).
    Unsupported8008Intrinsic(String),
    /// V1 doesn't compile string-literal expressions; LANG77 will add
    /// `.rodata` packaging.  (Currently unreachable — the Oct grammar
    /// has no string literals — but reserved for forwards compatibility.)
    StringsNotYetSupported,
    /// AST shape didn't match our expectations.  A parser change
    /// probably requires updating this crate.
    Malformed(String),
}

impl std::fmt::Display for OctError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OctError::Parse(s) => write!(f, "oct parse: {s}"),
            OctError::Type(errs) => {
                write!(f, "oct type-check failed:\n{}", errs.join("\n"))
            }
            OctError::Unsupported8008Intrinsic(name) => write!(f,
                "oct: 8008 intrinsic '{name}' is not supported on the LANG VM \
                 AOT chain — use the dedicated Intel-8008 simulator backend"),
            OctError::StringsNotYetSupported => write!(f,
                "oct: string literals require LANG77 (.rodata packaging); \
                 not yet supported"),
            OctError::Malformed(s) => write!(f, "oct AST malformed: {s}"),
        }
    }
}

impl std::error::Error for OctError {}

/// Compile an Oct source string to an [`IIRModule`].
///
/// The pipeline runs:
///
/// 1. `oct-parser` — produces the AST.
/// 2. `oct-type-checker` — verifies invariants.
/// 3. This crate — walks the typed AST emitting IIR.
///
/// On success the returned module has `entry_point = Some("main")` and
/// `main` returns `i64`.  See the crate doc for the entry-point
/// convention.
pub fn compile_source(source: &str, module_name: &str)
    -> Result<IIRModule, OctError>
{
    let ast = parse_oct(source).map_err(|e| OctError::Parse(format!("{e}")))?;
    let type_result = check_ast(&ast);
    if !type_result.ok {
        return Err(OctError::Type(
            type_result.errors.into_iter().map(|e| e.message).collect()
        ));
    }
    compile_ast(&ast, module_name)
}

/// Compile a pre-parsed (but not necessarily type-checked) AST.
///
/// Most callers want [`compile_source`]; this is exposed for testing
/// and for callers that have already invoked the parser.
pub fn compile_ast(ast: &GrammarASTNode, module_name: &str)
    -> Result<IIRModule, OctError>
{
    let mut comp = Compiler::default();
    comp.compile_program(ast)?;
    let mut module = IIRModule::new(module_name, "oct");
    module.functions = comp.functions;
    module.entry_point = Some("main".to_string());
    Ok(module)
}

// ===========================================================================
// Compiler
// ===========================================================================

struct Compiler {
    functions: Vec<IIRFunction>,
    /// Loop-end labels for `break` to jump to (innermost on top).
    break_stack: Vec<String>,
    /// Per-function: counter for temp register names.
    tmp_counter: usize,
    /// Per-function: counter for synthesised label families.
    label_counter: usize,
    /// Per-function: per-instruction source positions, built in
    /// lockstep with the function body.  Reset at the start of each
    /// `compile_function` call.
    source_map: Vec<SourceLoc>,
    /// "Currently compiling" source position.  Updated by every
    /// `compile_stmt` / `compile_expr` entry point and read by
    /// [`emit`] when it appends to the instruction stream.  Using a
    /// `Cell` so `emit(&self, ...)` can stay non-mutable for the
    /// callers that already pass `&self`.
    current_loc: std::cell::Cell<SourceLoc>,
    /// Names of top-level `static` declarations (LANG-FULL O3).
    ///
    /// Oct keeps locals in registers keyed by their source name (a
    /// `let x` lowers to a register literally called `x`).  A `static`
    /// must instead outlive the frame that touches it and be visible to
    /// *every* function, so it lowers to a **module global** — the IIR
    /// `global_load "x"` / `global_store "x"` ops (LANG32, the same path
    /// ALGOL's enclosing-block scalars use).  This set lets the read site
    /// (`compile_token_primary`) and the write site (`compile_assign`)
    /// recognise a name as a static and emit the global op instead of a
    /// register move.  Populated by a pre-pass in `compile_program`
    /// before any function body is lowered.
    statics: std::collections::HashSet<String>,
    /// Names of user functions declared with **no return type** (`void`).
    ///
    /// A call to a void function must NOT bind a result register: the IIR
    /// `call` instruction's `dest` has to be `None`, otherwise a strict
    /// backend rejects it (LLVM: "instructions returning void cannot have a
    /// name" — `%t = call void @f()` is malformed).  `compile_call_expr`
    /// consults this set to decide whether to bind a `dest`.  Populated by the
    /// same pre-pass that collects [`statics`], so a call can be classified
    /// even when the callee is declared *after* the call site.  (`main` is
    /// materialised as `i64` for the AOT exit-code convention and is never
    /// user-called, so its membership here is harmless.)
    void_fns: std::collections::HashSet<String>,
}

impl Default for Compiler {
    fn default() -> Self {
        Compiler {
            functions: Vec::new(),
            break_stack: Vec::new(),
            tmp_counter: 0,
            label_counter: 0,
            source_map: Vec::new(),
            current_loc: std::cell::Cell::new(SourceLoc::SYNTHETIC),
            statics: std::collections::HashSet::new(),
            void_fns: std::collections::HashSet::new(),
        }
    }
}

impl Compiler {
    fn fresh_tmp(&mut self) -> String {
        let i = self.tmp_counter;
        self.tmp_counter += 1;
        format!("_t{i}")
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let i = self.label_counter;
        self.label_counter += 1;
        format!("{prefix}_{i}")
    }

    fn emit(&mut self, out: &mut Vec<IIRInstr>, op: &str, dest: Option<&str>,
            srcs: Vec<Operand>, ty: &str)
    {
        out.push(IIRInstr::new(op, dest.map(|s| s.to_string()), srcs, ty));
        // Tag this instruction with the "currently compiling" source
        // position.  Statement-level compile_* entry points set
        // self.current_loc on entry, so every instruction emitted
        // while compiling that statement (including from sub-
        // expressions) inherits the statement's source line.
        //
        // Line-based debuggers care about which line a breakpoint
        // sits on, not the per-expression column.  Statement-level
        // granularity is both sufficient and cheaper than per-
        // expression threading.
        self.source_map.push(self.current_loc.get());
    }

    /// Update the "currently compiling" source position.  Subsequent
    /// `emit` calls tag their instructions with this position via the
    /// `source_map` field.
    fn set_loc(&self, loc: SourceLoc) {
        self.current_loc.set(loc);
    }

    fn compile_program(&mut self, ast: &GrammarASTNode) -> Result<(), OctError> {
        if ast.rule_name != "program" {
            return Err(OctError::Malformed(format!(
                "expected root `program`, got `{}`", ast.rule_name)));
        }
        // ── Pass 1 — statics (LANG-FULL O3) ────────────────────────────────
        //
        // A top-level `static counter: u8 = 40;` becomes a **module global**,
        // not a register: it must be readable and writable from *every*
        // function and must survive across calls.  We collect the static
        // names here (so the per-function read/write sites recognise them and
        // emit `global_load`/`global_store`) and remember each one's
        // initialiser expression so `main` can run it once at start-up.
        //
        // Why initialise in `main` rather than at "module load"?  The AOT
        // backends have no module-init hook; `main` is the single entry every
        // backend runs first, so emitting `global_store "name", <init>` at the
        // top of `main` gives the static its declared value before any other
        // code observes it.  (`global_load` of an unwritten global already
        // reads 0 on every backend, so a static with a literal-0 initialiser
        // would even be correct without this — but we always emit the store so
        // non-zero initialisers like `= 40` work.)
        //
        // `static_inits` borrows the initialiser nodes out of `ast`, which
        // outlives this whole function, so the references stay valid until the
        // second pass consumes them.
        let mut static_inits: Vec<(String, &GrammarASTNode)> = Vec::new();
        for top in child_nodes(ast) {
            if top.rule_name != "top_decl" { continue; }
            for inner in child_nodes(top) {
                // Record void-returning functions so calls to them omit a
                // result register (see `void_fns`).  Forward references work
                // because this pre-pass sees every declaration first.
                if inner.rule_name == "fn_decl" {
                    if let Some(fname) = first_name_token(inner) {
                        if extract_return_type(inner).is_none() {
                            self.void_fns.insert(fname);
                        }
                    }
                    continue;
                }
                if inner.rule_name != "static_decl" { continue; }
                let name = first_name_token(inner).ok_or_else(|| {
                    OctError::Malformed("static_decl missing NAME".into())
                })?;
                // `static_decl = "static" NAME COLON type EQ expr SEMICOLON`.
                // The single non-`type` child node is the initialiser expr.
                let init = child_nodes(inner)
                    .into_iter()
                    .find(|n| n.rule_name != "type")
                    .ok_or_else(|| {
                        OctError::Malformed(
                            "static_decl missing initialiser".into(),
                        )
                    })?;
                self.statics.insert(name.clone());
                static_inits.push((name, init));
            }
        }

        // ── Pass 2 — lower every function ──────────────────────────────────
        //
        // `main` receives the static initialiser list so it can emit the
        // start-up stores; every other function ignores it.
        for top in child_nodes(ast) {
            if top.rule_name != "top_decl" { continue; }
            for inner in child_nodes(top) {
                if inner.rule_name == "fn_decl" {
                    self.compile_fn(inner, &static_inits)?;
                }
            }
        }
        Ok(())
    }

    fn compile_fn(
        &mut self,
        fn_decl: &GrammarASTNode,
        static_inits: &[(String, &GrammarASTNode)],
    ) -> Result<(), OctError> {
        // Reset per-function counters for stable register naming.
        self.tmp_counter = 0;
        self.label_counter = 0;
        self.break_stack.clear();
        // Reset per-function source-map state.  The vec accumulates
        // one `SourceLoc` per emitted instruction so the resulting
        // `IIRFunction.source_map` stays in lockstep with
        // `instructions` — the lockstep invariant the IIR consumers
        // (debugger, stack traces, AOT debug info) require.
        self.source_map.clear();
        // Default position for instructions emitted before any
        // statement (e.g. the synthesised main epilogue): the fn
        // declaration's own line.
        self.set_loc(node_loc(fn_decl));

        let name = first_name_token(fn_decl)
            .ok_or_else(|| OctError::Malformed("fn_decl missing NAME".into()))?;
        let params = extract_params(fn_decl);
        let return_type = extract_return_type(fn_decl);

        let mut body: Vec<IIRInstr> = Vec::new();

        // Static initialisers run once, at the very top of `main`, before any
        // user statement (LANG-FULL O3 — see `compile_program` for the
        // rationale).  Each `static counter: u8 = <expr>;` lowers to
        // "evaluate <expr>, then `global_store "counter", <value>`".
        if name == "main" {
            for (sname, init_node) in static_inits {
                let v = self.compile_expr(init_node, &mut body)?;
                self.emit(
                    &mut body,
                    "global_store",
                    None,
                    vec![Operand::Str(sname.clone()), Operand::Var(v)],
                    "i64",
                );
            }
        }

        if let Some(block) = child_nodes(fn_decl).into_iter()
            .find(|n| n.rule_name == "block")
        {
            self.compile_block(block, &mut body)?;
        }

        // Synthesize an i64 ret on `main` so the AOT chain's exit-code
        // convention works.  Oct's `main` is declared void; we materialise
        // it as `i64` returning 0.
        //
        // A non-void return type must materialise as `i64` too — every Oct value
        // flows through 64-bit slots (params are widened to `i64` below, and the
        // body's arithmetic/`ret` emit `i64`), so a function signature declaring the
        // narrow source type (`u8` → LLVM `i8`) would mismatch the `i64` value the
        // body returns ("value doesn't match function result type 'i8'").  Matches
        // how Nib materialises its function types.
        let actual_return_type = if name == "main" {
            "i64".to_string()
        } else {
            match return_type.as_deref() {
                None | Some("void") => "void".to_string(),
                Some(_) => "i64".to_string(), // u8 / bool / … all flow as i64
            }
        };

        if name == "main" {
            // Ensure main ends with `const 0; ret 0`.  If user wrote an
            // explicit `return;` we already emitted `ret_void`, but the
            // AOT chain needs `ret i64`.  Replace any trailing `ret_void`
            // with `const r=0; ret r`, and append the same if missing.
            if let Some(last) = body.last() {
                if last.op == "ret_void" {
                    body.pop();
                }
            }
            if body.last().is_none_or(|i| i.op != "ret") {
                let r = self.fresh_tmp();
                self.emit(&mut body, "const", Some(&r),
                          vec![Operand::Int(0)], "i64");
                self.emit(&mut body, "ret", None,
                          vec![Operand::Var(r)], "i64");
            }
        } else if !body.iter().any(|i| i.op.starts_with("ret")) {
            // Defensive epilogue: void → `ret_void`; typed → `const 0; ret 0`.
            match return_type.as_deref() {
                Some(_) => {
                    let r = self.fresh_tmp();
                    self.emit(&mut body, "const", Some(&r),
                              vec![Operand::Int(0)], "i64");
                    self.emit(&mut body, "ret", None,
                              vec![Operand::Var(r)], "i64");
                }
                None => {
                    self.emit(&mut body, "ret_void", None, vec![], "void");
                }
            }
        }

        let mut iir_fn = IIRFunction::new(
            &name,
            params.into_iter().map(|(n, _ty)| (n, "i64".to_string())).collect(),
            &actual_return_type,
            body,
        );
        // Override `IIRFunction::new`'s automatic `infer_type_status` —
        // it returns `PartiallyTyped` for Oct because control-flow ops
        // (`label`, `jmp`, `jmp_if_false`, `ret_void`) use `"void"`
        // type hints and `"void"` is NOT in
        // `interpreter_ir::opcodes::CONCRETE_TYPES`.  Every Oct
        // instruction is in fact statically known (no `"any"` hints
        // anywhere), so the function is genuinely fully typed for the
        // JIT's threshold-zero compile path.  Mirrors Brainfuck +
        // Dartmouth BASIC.
        iir_fn.type_status = interpreter_ir::function::FunctionTypeStatus::FullyTyped;
        // Move the accumulated source positions onto the function.
        // The lockstep invariant (one entry per instruction) is
        // enforced by [`emit`]: every push to `body` pairs with a
        // push to `source_map`.  We defensively pad with the fn
        // declaration's own location in case any pre-source_map code
        // path slipped through (this branch is dead today but cheap).
        let body_len = iir_fn.instructions.len();
        while self.source_map.len() < body_len {
            self.source_map.push(node_loc(fn_decl));
        }
        if self.source_map.len() > body_len {
            self.source_map.truncate(body_len);
        }
        iir_fn.source_map = std::mem::take(&mut self.source_map);
        self.functions.push(iir_fn);
        Ok(())
    }

    // ── Statement lowering ─────────────────────────────────────────────────

    fn compile_block(&mut self, block: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<(), OctError>
    {
        for stmt in child_nodes(block) {
            if stmt.rule_name == "stmt" {
                self.compile_stmt(stmt, out)?;
            }
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<(), OctError>
    {
        // Tag every instruction emitted while compiling this statement
        // (including from sub-expressions) with the statement's source
        // position.  See `emit`'s documentation for why statement-level
        // granularity is correct for line-based breakpoints.
        self.set_loc(node_loc(stmt));
        // `stmt` wraps exactly one of the *_stmt rules.
        let inner = child_nodes(stmt).into_iter().next()
            .ok_or_else(|| OctError::Malformed("empty stmt".into()))?;
        match inner.rule_name.as_str() {
            "let_stmt"     => self.compile_let(inner, out),
            "static_decl"  => Ok(()), // global-level; ignored inside body
            "assign_stmt"  => self.compile_assign(inner, out),
            "return_stmt"  => self.compile_return(inner, out),
            "if_stmt"      => self.compile_if(inner, out),
            "while_stmt"   => self.compile_while(inner, out),
            "loop_stmt"    => self.compile_loop(inner, out),
            "break_stmt"   => self.compile_break(inner, out),
            "expr_stmt"    => self.compile_expr_stmt(inner, out),
            other => Err(OctError::Malformed(format!("unknown stmt `{other}`"))),
        }
    }

    fn compile_let(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<(), OctError>
    {
        let name = first_name_token(node)
            .ok_or_else(|| OctError::Malformed("let_stmt missing NAME".into()))?;
        let expr = child_nodes(node).into_iter()
            .find(|n| n.rule_name != "type")
            .ok_or_else(|| OctError::Malformed("let_stmt missing initialiser".into()))?;
        let v = self.compile_expr(expr, out)?;
        self.emit(out, "mov", Some(&name), vec![Operand::Var(v)], "i64");
        Ok(())
    }

    fn compile_assign(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<(), OctError>
    {
        let name = first_name_token(node)
            .ok_or_else(|| OctError::Malformed("assign_stmt missing NAME".into()))?;
        let expr = child_nodes(node).into_iter().next()
            .ok_or_else(|| OctError::Malformed("assign_stmt missing expr".into()))?;
        let v = self.compile_expr(expr, out)?;
        if self.statics.contains(&name) {
            // Assigning a `static` (LANG-FULL O3) — write the module global,
            // not a register, so the new value is visible to other functions
            // and the next call.
            self.emit(out, "global_store", None,
                      vec![Operand::Str(name), Operand::Var(v)], "i64");
        } else {
            self.emit(out, "mov", Some(&name), vec![Operand::Var(v)], "i64");
        }
        Ok(())
    }

    fn compile_return(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<(), OctError>
    {
        match child_nodes(node).into_iter().next() {
            Some(expr) => {
                let v = self.compile_expr(expr, out)?;
                self.emit(out, "ret", None, vec![Operand::Var(v)], "i64");
            }
            None => {
                self.emit(out, "ret_void", None, vec![], "void");
            }
        }
        Ok(())
    }

    fn compile_if(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<(), OctError>
    {
        let mut cond: Option<&GrammarASTNode> = None;
        let mut blocks: Vec<&GrammarASTNode> = Vec::new();
        for c in child_nodes(node) {
            if c.rule_name == "block" { blocks.push(c); }
            else if cond.is_none() { cond = Some(c); }
        }
        let cond = cond.ok_or_else(|| OctError::Malformed("if missing cond".into()))?;
        let then_block = blocks.first().copied()
            .ok_or_else(|| OctError::Malformed("if missing then-block".into()))?;
        let else_block = blocks.get(1).copied();

        let cond_v = self.compile_expr(cond, out)?;
        let else_lbl = self.fresh_label("if_else");
        let end_lbl  = self.fresh_label("if_end");

        self.emit(out, "jmp_if_false", None,
            vec![Operand::Var(cond_v), Operand::Var(else_lbl.clone())], "void");
        self.compile_block(then_block, out)?;
        self.emit(out, "jmp", None, vec![Operand::Var(end_lbl.clone())], "void");
        self.emit(out, "label", None, vec![Operand::Var(else_lbl)], "void");
        if let Some(eb) = else_block {
            self.compile_block(eb, out)?;
        }
        self.emit(out, "label", None, vec![Operand::Var(end_lbl)], "void");
        Ok(())
    }

    fn compile_while(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<(), OctError>
    {
        let mut cond: Option<&GrammarASTNode> = None;
        let mut block: Option<&GrammarASTNode> = None;
        for c in child_nodes(node) {
            if c.rule_name == "block" { block = Some(c); }
            else if cond.is_none() { cond = Some(c); }
        }
        let cond = cond.ok_or_else(|| OctError::Malformed("while missing cond".into()))?;
        let block = block.ok_or_else(|| OctError::Malformed("while missing body".into()))?;

        let top = self.fresh_label("while_top");
        let end = self.fresh_label("while_end");
        self.break_stack.push(end.clone());

        self.emit(out, "label", None, vec![Operand::Var(top.clone())], "void");
        let cond_v = self.compile_expr(cond, out)?;
        self.emit(out, "jmp_if_false", None,
            vec![Operand::Var(cond_v), Operand::Var(end.clone())], "void");
        self.compile_block(block, out)?;
        self.emit(out, "jmp", None, vec![Operand::Var(top)], "void");
        self.emit(out, "label", None, vec![Operand::Var(end)], "void");

        self.break_stack.pop();
        Ok(())
    }

    fn compile_loop(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<(), OctError>
    {
        let block = child_nodes(node).into_iter()
            .find(|n| n.rule_name == "block")
            .ok_or_else(|| OctError::Malformed("loop missing body".into()))?;
        let top = self.fresh_label("loop_top");
        let end = self.fresh_label("loop_end");
        self.break_stack.push(end.clone());

        self.emit(out, "label", None, vec![Operand::Var(top.clone())], "void");
        self.compile_block(block, out)?;
        self.emit(out, "jmp", None, vec![Operand::Var(top)], "void");
        self.emit(out, "label", None, vec![Operand::Var(end)], "void");

        self.break_stack.pop();
        Ok(())
    }

    fn compile_break(&mut self, _node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<(), OctError>
    {
        // The type checker doesn't verify break-in-loop today, but a
        // structurally well-formed source can't reach here otherwise
        // (the parser requires `break` to appear inside a block).
        let end = self.break_stack.last().cloned()
            .ok_or_else(|| OctError::Malformed("`break` outside of a loop".into()))?;
        self.emit(out, "jmp", None, vec![Operand::Var(end)], "void");
        Ok(())
    }

    fn compile_expr_stmt(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<(), OctError>
    {
        let inner = child_nodes(node).into_iter().next()
            .ok_or_else(|| OctError::Malformed("expr_stmt missing expr".into()))?;
        let _ = self.compile_expr(inner, out)?;
        Ok(())
    }

    // ── Expression lowering ───────────────────────────────────────────────

    fn compile_expr(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<String, OctError>
    {
        match node.rule_name.as_str() {
            "expr" => {
                let inner = child_nodes(node).into_iter().next()
                    .ok_or_else(|| OctError::Malformed("empty expr".into()))?;
                self.compile_expr(inner, out)
            }
            // `&&` / `||` must SHORT-CIRCUIT (LANG-FULL O1): the right operand is
            // evaluated only when the left doesn't already decide the result. They
            // cannot go through `compile_binary` (which eagerly evaluates both sides).
            "or_expr"  => self.compile_short_circuit(node, false, out),
            "and_expr" => self.compile_short_circuit(node, true, out),
            "eq_expr" | "cmp_expr"
            | "add_expr" | "bitwise_expr" => self.compile_binary(node, out),
            "unary_expr" => self.compile_unary(node, out),
            "primary"    => self.compile_primary(node, out),
            _ => {
                // Unknown wrapper — recurse into first child.
                let inner = child_nodes(node).into_iter().next()
                    .ok_or_else(|| OctError::Malformed(format!(
                        "unknown expr rule `{}`", node.rule_name)))?;
                self.compile_expr(inner, out)
            }
        }
    }

    // Explicit loop with an internal break reads clearer than while-let (allow 1.97 while_let_loop).
    #[allow(clippy::while_let_loop)]
    fn compile_binary(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<String, OctError>
    {
        // Pass-through detection: if there's only one operand child the
        // higher-precedence rule will handle it; recurse.
        let operand_count = node.children.iter().filter(|c| match c {
            ASTNodeOrToken::Node(_) => true,
            ASTNodeOrToken::Token(t) => !is_binary_op_token(&token_kind(t)),
        }).count();
        if operand_count == 1 {
            let first = node.children.iter().find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                _ => None,
            }).ok_or_else(|| OctError::Malformed(format!(
                "{} has no node child", node.rule_name)))?;
            return self.compile_expr(first, out);
        }

        // Walk pairwise: left, op, right, op, right, …
        let mut iter = node.children.iter();
        let first = iter.next().ok_or_else(|| OctError::Malformed(
            format!("empty {}", node.rule_name)))?;
        let mut acc = self.compile_child(first, out)?;

        loop {
            let Some(op_child) = iter.next() else { break; };
            let op_kind = match op_child {
                ASTNodeOrToken::Token(t) => token_kind(t),
                _ => break,
            };
            let Some(rhs_child) = iter.next() else { break; };
            let rhs = self.compile_child(rhs_child, out)?;
            let cir_op = match op_kind.as_str() {
                "PLUS" => "add",
                "MINUS" => "sub",
                "AMP" => "and",
                "PIPE" => "or",
                "CARET" => "xor",
                "EQ_EQ" => "cmp_eq",
                "NEQ" => "cmp_ne",
                "LT" => "cmp_lt",
                "GT" => "cmp_gt",
                "LEQ" => "cmp_le",
                "GEQ" => "cmp_ge",
                // `LAND` / `LOR` no longer reach here — `compile_expr` routes
                // `and_expr` / `or_expr` to `compile_short_circuit` (LANG-FULL O1).
                other => return Err(OctError::Malformed(format!(
                    "unknown operator token `{other}`"))),
            };
            let dest = self.fresh_tmp();
            // LANG-FULL O2 — u8 width & wrap. Oct's only integer type is `u8` (the
            // 8008's byte; see the grammar — `bool` is the only other type), and the
            // language spec says arithmetic "wraps modulo 256". So an arithmetic /
            // bitwise op carries the `u8` type_hint and every backend masks its result
            // mod-2⁸ (the E2 value-mask): `200 + 100 = 44`, not 300. A **comparison**
            // (`cmp_*`) yields a 0/1 `bool` that must NOT be masked (and its operands
            // ride i64 slots), so it stays `i64` — emitting `u8` would mis-type the
            // LLVM `icmp`. (There is no width to *track* here the way Nib does: Oct has
            // exactly one integer width, so every integer op is u8 by construction.)
            let hint = if cir_op.starts_with("cmp_") { "i64" } else { "u8" };
            self.emit(out, cir_op, Some(&dest),
                vec![Operand::Var(acc), Operand::Var(rhs)], hint);
            acc = dest;
        }
        Ok(acc)
    }

    /// Compile a short-circuiting `&&` (`is_and = true`) or `||` chain (LANG-FULL O1).
    ///
    /// `&&` / `||` evaluate the right operand only when the left does not already
    /// decide the result.  This matters once an operand has a side effect — in Oct,
    /// a comparison whose side is a function call that `out`-puts (`f() == 1`): under
    /// the old eager bitwise lowering that call ran unconditionally.  We lower to a
    /// result slot guarded by branches, using only `jmp_if_false` / `jmp` / `label`
    /// (the portable subset every backend lowers — the CLR textual `.il` path has no
    /// `jmp_if_true`).  Single-operand `and_expr`/`or_expr` (no real operator) fall
    /// through to `compile_binary`.
    fn compile_short_circuit(&mut self, node: &GrammarASTNode, is_and: bool,
        out: &mut Vec<IIRInstr>) -> Result<String, OctError>
    {
        let operands: Vec<&GrammarASTNode> = node.children.iter().filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            _ => None,
        }).collect();
        if operands.len() < 2 {
            // No real `&&`/`||` operator at this level — pass through.
            return self.compile_binary(node, out);
        }

        let result = self.fresh_tmp();
        let end_lbl = self.fresh_label("sc_end");

        // result = first operand
        let v0 = self.compile_expr(operands[0], out)?;
        self.emit(out, "mov", Some(&result), vec![Operand::Var(v0)], "i64");

        for operand in &operands[1..] {
            if is_and {
                // If the accumulator is already false, short-circuit (stays false).
                self.emit(out, "jmp_if_false", None,
                    vec![Operand::Var(result.clone()), Operand::Var(end_lbl.clone())], "void");
                let v = self.compile_expr(operand, out)?;
                self.emit(out, "mov", Some(&result), vec![Operand::Var(v)], "i64");
            } else {
                // `||`: with only `jmp_if_false`, false → evaluate next; true → jump end.
                let eval_lbl = self.fresh_label("sc_eval");
                self.emit(out, "jmp_if_false", None,
                    vec![Operand::Var(result.clone()), Operand::Var(eval_lbl.clone())], "void");
                self.emit(out, "jmp", None, vec![Operand::Var(end_lbl.clone())], "void");
                self.emit(out, "label", None, vec![Operand::Var(eval_lbl)], "void");
                let v = self.compile_expr(operand, out)?;
                self.emit(out, "mov", Some(&result), vec![Operand::Var(v)], "i64");
            }
        }

        self.emit(out, "label", None, vec![Operand::Var(end_lbl)], "void");
        Ok(result)
    }

    fn compile_unary(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<String, OctError>
    {
        // `unary = (BANG | TILDE) unary | primary`
        let kids: Vec<&ASTNodeOrToken> = node.children.iter().collect();
        let first = kids.first().ok_or_else(|| OctError::Malformed("empty unary".into()))?;
        if let ASTNodeOrToken::Token(t) = first {
            let kind = token_kind(t);
            if kind == "BANG" || kind == "TILDE" {
                let operand = kids.get(1).ok_or_else(|| OctError::Malformed(
                    "unary missing operand".into()))?;
                let v = self.compile_child(operand, out)?;
                let dest = self.fresh_tmp();
                if kind == "TILDE" {
                    // LANG-FULL O2: `~u8` is bitwise complement at the byte width, so
                    // every backend must mask the shared IIR `not` result to 8 bits.
                    self.emit(out, "not", Some(&dest),
                        vec![Operand::Var(v)], "u8");
                    return Ok(dest);
                }

                // LANG-FULL O-!: `!bool` is logical negation, not bitwise complement.
                // Oct booleans are materialised as 0/1 i64 values; lowering through
                // branches keeps the result a portable clean 0/1 instead of `not 0`
                // -> -1 or `not 1` -> -2.
                let false_lbl = self.fresh_label("not_false");
                let end_lbl = self.fresh_label("not_end");
                self.emit(out, "jmp_if_false", None,
                    vec![Operand::Var(v), Operand::Var(false_lbl.clone())], "void");
                self.emit(out, "const", Some(&dest), vec![Operand::Int(0)], "i64");
                self.emit(out, "jmp", None, vec![Operand::Var(end_lbl.clone())], "void");
                self.emit(out, "label", None, vec![Operand::Var(false_lbl)], "void");
                self.emit(out, "const", Some(&dest), vec![Operand::Int(1)], "i64");
                self.emit(out, "label", None, vec![Operand::Var(end_lbl)], "void");
                return Ok(dest);
            }
        }
        // Pass-through.
        self.compile_child(first, out)
    }

    fn compile_primary(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<String, OctError>
    {
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => match n.rule_name.as_str() {
                    "intrinsic_call" => return self.compile_intrinsic(n, out),
                    "call_expr"      => return self.compile_call_expr(n, out),
                    "expr"           => return self.compile_expr(n, out),
                    _ => return self.compile_expr(n, out),
                },
                ASTNodeOrToken::Token(t) => {
                    let kind = token_kind(t);
                    if kind == "LPAREN" || kind == "RPAREN" { continue; }
                    return self.compile_token_primary(t, out);
                }
            }
        }
        Err(OctError::Malformed(format!(
            "empty primary node (kids={})", node.children.len())))
    }

    fn compile_token_primary(&mut self, tok: &lexer::token::Token, out: &mut Vec<IIRInstr>)
        -> Result<String, OctError>
    {
        let kind = token_kind(tok);
        let v = match kind.as_str() {
            "INT_LIT" => tok.value.parse::<i64>().unwrap_or(0),
            "HEX_LIT" => i64::from_str_radix(
                tok.value.trim_start_matches("0x"), 16).unwrap_or(0),
            "BIN_LIT" => i64::from_str_radix(
                tok.value.trim_start_matches("0b"), 2).unwrap_or(0),
            "true"    => 1,
            "false"   => 0,
            "NAME"    => {
                // A `static` (LANG-FULL O3) lives in a module global, not a
                // register, so reading it means `global_load "name" -> tmp`.
                if self.statics.contains(&tok.value) {
                    let dest = self.fresh_tmp();
                    self.emit(out, "global_load", Some(&dest),
                              vec![Operand::Str(tok.value.clone())], "i64");
                    return Ok(dest);
                }
                // Bare local identifier — already in a register slot keyed
                // by name (see compile_let / compile_assign / fn param).
                return Ok(tok.value.clone());
            }
            _ => return Err(OctError::Malformed(format!(
                "unknown primary token kind `{kind}`"))),
        };
        let dest = self.fresh_tmp();
        self.emit(out, "const", Some(&dest), vec![Operand::Int(v)], "i64");
        Ok(dest)
    }

    fn compile_child(&mut self, child: &ASTNodeOrToken, out: &mut Vec<IIRInstr>)
        -> Result<String, OctError>
    {
        match child {
            ASTNodeOrToken::Node(n) => self.compile_expr(n, out),
            ASTNodeOrToken::Token(t) => self.compile_token_primary(t, out),
        }
    }

    fn compile_call_expr(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<String, OctError>
    {
        let name = first_name_token(node)
            .ok_or_else(|| OctError::Malformed("call_expr missing NAME".into()))?;
        let is_void = self.void_fns.contains(&name);
        let mut srcs = vec![Operand::Var(name)];
        if let Some(arg_list) = child_nodes(node).into_iter()
            .find(|n| n.rule_name == "arg_list")
        {
            for c in child_nodes(arg_list) {
                let v = self.compile_expr(c, out)?;
                srcs.push(Operand::Var(v));
            }
        }
        if is_void {
            // A void function call binds NO result — the IIR `call`'s `dest`
            // must be `None` (LLVM rejects `%t = call void @f()`).  The value
            // is never read (the type checker forbids using a void call as an
            // operand), so we return an empty placeholder for the discarding
            // statement context.
            self.emit(out, "call", None, srcs, "void");
            Ok(String::new())
        } else {
            let dest = self.fresh_tmp();
            self.emit(out, "call", Some(&dest), srcs, "i64");
            Ok(dest)
        }
    }

    fn compile_intrinsic(&mut self, node: &GrammarASTNode, out: &mut Vec<IIRInstr>)
        -> Result<String, OctError>
    {
        let name = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) => {
                let v = &t.value;
                if matches!(v.as_str(),
                    "in" | "out" | "adc" | "sbb" | "rlc"
                    | "rrc" | "ral" | "rar" | "carry" | "parity")
                { Some(v.clone()) } else { None }
            }
            _ => None,
        }).unwrap_or_else(|| "?".to_string());

        // `out(port, value)` — the 8008 writes `value` to I/O `port` (LANG-FULL
        // O-OUT).  Real hardware has 24 output ports; on the general LANG backends
        // they all collapse to **stdout**, lowered as `call_builtin "print_i64"` —
        // the same print builtin Dartmouth BASIC's `PRINT` uses, already wired on
        // every backend (VM/JIT register it; LLVM `@__print_i64`, JVM `System.out`,
        // CLR `Console.WriteLine`, WASM `env.__print_i64`).  This gives Oct its first
        // *observable* output, so its behaviour can be verified by running — Oct's
        // `main` is void (always exits 0), so the exit code can never witness a
        // computed result.  The `port` argument is a compile-time-constant hardware
        // port selector; with all ports mapped to stdout it has no effect, so we
        // don't evaluate it.
        if name == "out" {
            let arg_nodes = child_nodes(node); // [port_expr, value_expr]
            let value_node = arg_nodes.get(1).ok_or_else(|| {
                OctError::Malformed("out() expects (port, value)".into())
            })?;
            let value = self.compile_expr(value_node, out)?;
            self.emit(out, "call_builtin", None,
                vec![Operand::Var("print_i64".to_string()), Operand::Var(value)], "void");
            // `out` is statement-shaped with no value; return a fresh `0` for the
            // (discarded) expression slot so callers get a valid name.
            let dummy = self.fresh_tmp();
            self.emit(out, "const", Some(&dummy), vec![Operand::Int(0)], "i64");
            return Ok(dummy);
        }

        // The remaining 8008 intrinsics (`in`, `adc`, `sbb`, the rotations,
        // `carry`, `parity`) have no general-backend model yet and stay rejected.
        Err(OctError::Unsupported8008Intrinsic(name))
    }
}

// ===========================================================================
// AST helpers
// ===========================================================================

fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        _ => None,
    }).collect()
}

fn first_name_token(node: &GrammarASTNode) -> Option<String> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if token_kind(t) == "NAME" {
                return Some(t.value.clone());
            }
        }
    }
    None
}

fn token_kind(t: &lexer::token::Token) -> String {
    t.effective_type_name().to_string()
}

fn is_binary_op_token(kind: &str) -> bool {
    matches!(kind,
        "LOR" | "LAND" | "EQ_EQ" | "NEQ" | "LT" | "GT" | "LEQ" | "GEQ"
        | "PLUS" | "MINUS" | "AMP" | "PIPE" | "CARET"
    )
}

fn extract_params(fn_decl: &GrammarASTNode) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Some(plist) = child_nodes(fn_decl).into_iter()
        .find(|n| n.rule_name == "param_list") else { return out; };
    for p in child_nodes(plist) {
        if p.rule_name != "param" { continue; }
        let Some(name) = first_name_token(p) else { continue; };
        // V1 represents every Oct type as `i64` at the IIR level (u8 and
        // bool both fit; type-check has already verified usage).
        out.push((name, "i64".to_string()));
    }
    out
}

fn extract_return_type(fn_decl: &GrammarASTNode) -> Option<String> {
    let mut saw_arrow = false;
    for c in &fn_decl.children {
        match c {
            ASTNodeOrToken::Token(t) if token_kind(t) == "ARROW" => saw_arrow = true,
            ASTNodeOrToken::Node(n) if saw_arrow && n.rule_name == "type" => {
                return first_name_token(n);
            }
            _ => {}
        }
    }
    None
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn ops(m: &IIRModule, fn_name: &str) -> Vec<String> {
        m.functions.iter().find(|f| f.name == fn_name).unwrap()
            .instructions.iter().map(|i| i.op.clone()).collect()
    }

    #[test]
    fn compiles_minimal_main() {
        let m = compile_source("fn main() { }", "test").expect("ok");
        assert_eq!(m.entry_point.as_deref(), Some("main"));
        let main = m.functions.iter().find(|f| f.name == "main").expect("main");
        assert_eq!(main.return_type, "i64",
                   "main must be rewritten to i64 for AOT");
        // body must end in `ret`.
        assert_eq!(main.instructions.last().unwrap().op, "ret");
    }

    #[test]
    fn compiles_let_and_arithmetic() {
        let src = "fn main() { let x: u8 = 30; let y: u8 = x + 12; }";
        let m = compile_source(src, "test").expect("ok");
        let o = ops(&m, "main");
        assert!(o.contains(&"const".to_string()));
        assert!(o.contains(&"add".to_string()));
        assert!(o.contains(&"mov".to_string()));
    }

    /// The `(op, type_hint)` pairs of a function's body — for asserting the O2 widths.
    fn op_hints(m: &IIRModule, fn_name: &str) -> Vec<(String, String)> {
        m.functions.iter().find(|f| f.name == fn_name).unwrap()
            .instructions.iter().map(|i| (i.op.clone(), i.type_hint.clone())).collect()
    }

    #[test]
    fn o2_arithmetic_carries_u8_hint_so_it_wraps() {
        // LANG-FULL O2: Oct's only integer type is u8, and arithmetic wraps mod-256, so
        // an `add`/`sub`/bitwise op must carry the `u8` type_hint (the backends then mask
        // the result). A `cmp_*` stays `i64` (its 0/1 bool result must not be masked).
        let m = compile_source(
            "fn main() { let x: u8 = 200 + 100; if x == 44 { let y: u8 = 1; } }",
            "test",
        ).expect("ok");
        let oh = op_hints(&m, "main");
        assert!(oh.iter().any(|(op, h)| op == "add" && h == "u8"),
            "Oct `add` must carry the u8 hint so it wraps; got: {oh:?}");
        assert!(oh.iter().any(|(op, h)| op == "cmp_eq" && h == "i64"),
            "Oct `cmp_eq` must stay i64 (its bool result is unmasked); got: {oh:?}");
    }

    #[test]
    fn o2_bitwise_not_carries_u8_hint() {
        // LANG-FULL O2: `~` lowers to the IIR `not` op with the `u8` hint so the
        // complement masks to 8 bits (`~0u8 = 255`), not the i64 all-ones (`-1`).
        let m = compile_source("fn main() { let x: u8 = ~0; }", "test").expect("ok");
        let oh = op_hints(&m, "main");
        assert!(oh.iter().any(|(op, h)| op == "not" && h == "u8"),
            "Oct `~` must lower to a u8-hinted `not`; got: {oh:?}");
    }

    #[test]
    fn o3_static_read_lowers_to_global_load() {
        // LANG-FULL O3: reading a top-level `static` must emit `global_load`
        // (a module global), never a bare register reference.
        let src = "static counter: u8 = 40; \
                   fn main() { out(1, counter); }";
        let m = compile_source(src, "test").expect("ok");
        let o = ops(&m, "main");
        assert!(o.contains(&"global_load".to_string()),
            "reading a static must lower to global_load; got: {o:?}");
    }

    #[test]
    fn o3_static_write_lowers_to_global_store() {
        // LANG-FULL O3: assigning a `static` must emit `global_store`, not a
        // register `mov` (which would be invisible to other functions).
        let src = "static counter: u8 = 0; \
                   fn bump() { counter = counter + 1; } \
                   fn main() { }";
        let m = compile_source(src, "test").expect("ok");
        let o = ops(&m, "bump");
        assert!(o.contains(&"global_store".to_string()),
            "assigning a static must lower to global_store; got: {o:?}");
        // The read inside `counter + 1` is also a global_load.
        assert!(o.contains(&"global_load".to_string()),
            "reading a static inside bump must lower to global_load; got: {o:?}");
    }

    #[test]
    fn o3_main_initialises_statics_first() {
        // The static's declared initialiser runs once at the top of `main`,
        // before any user statement — so `main`'s FIRST emitted op for a
        // `static counter: u8 = 40;` program is the const, immediately
        // followed by the `global_store` that seeds the global.
        let src = "static counter: u8 = 40; \
                   fn main() { out(1, counter); }";
        let m = compile_source(src, "test").expect("ok");
        let o = ops(&m, "main");
        assert_eq!(o.first().map(String::as_str), Some("const"),
            "main must start by materialising the static initialiser; got: {o:?}");
        // The init store precedes the first read of the static.
        let store_idx = o.iter().position(|op| op == "global_store");
        let load_idx = o.iter().position(|op| op == "global_load");
        assert!(store_idx.is_some() && load_idx.is_some(),
            "main must both seed (store) and read (load) the static; got: {o:?}");
        assert!(store_idx < load_idx,
            "the initialiser store must precede any read of the static; got: {o:?}");
    }

    #[test]
    fn void_call_binds_no_result_register() {
        // A call to a void-returning user function must emit a `call` whose
        // `dest` is `None`.  Binding a result (`%t = call void @f()`) is
        // malformed LLVM ("instructions returning void cannot have a name");
        // this was a latent bug the O3 proof's `bump()` surfaced — every prior
        // Oct program only ever called the non-void `side()`.
        let src = "fn act() { let x: u8 = 1; } \
                   fn main() { act(); }";
        let m = compile_source(src, "test").expect("ok");
        let call = m.functions.iter().find(|f| f.name == "main").unwrap()
            .instructions.iter().find(|i| i.op == "call")
            .expect("main must contain a call to act()");
        assert!(call.dest.is_none(),
            "a void call must not bind a result register; got dest={:?}", call.dest);
        assert_eq!(call.type_hint, "void");
    }

    #[test]
    fn non_void_call_still_binds_result() {
        // The void fix must not regress value-returning calls: `side()`
        // returns u8, so its call keeps a `dest` (its result feeds `let v`).
        let src = "fn side() -> u8 { return 5; } \
                   fn main() { let v: u8 = side(); }";
        let m = compile_source(src, "test").expect("ok");
        let call = m.functions.iter().find(|f| f.name == "main").unwrap()
            .instructions.iter().find(|i| i.op == "call")
            .expect("main must contain a call to side()");
        assert!(call.dest.is_some(),
            "a value-returning call must bind a result register; got dest=None");
    }

    #[test]
    fn o3_locals_are_unaffected_by_statics() {
        // A `let` local must still lower to a register `mov` — only declared
        // statics route through globals. (Guards against the read site
        // mis-classifying every NAME as a global.)
        let src = "static s: u8 = 1; \
                   fn main() { let x: u8 = 5; x = x + 1; }";
        let m = compile_source(src, "test").expect("ok");
        let o = ops(&m, "main");
        assert!(o.contains(&"mov".to_string()),
            "a plain local `let`/assign must still use mov; got: {o:?}");
    }

    #[test]
    fn compiles_if_else() {
        let src = "fn main() { let x: u8 = 0; if x == 0 { x = 1; } else { x = 2; } }";
        let m = compile_source(src, "test").expect("ok");
        let o = ops(&m, "main");
        assert!(o.contains(&"cmp_eq".to_string()));
        assert!(o.contains(&"jmp_if_false".to_string()));
        assert!(o.contains(&"jmp".to_string()));
        assert!(o.iter().filter(|op| *op == "label").count() >= 2);
    }

    #[test]
    fn compiles_while_loop() {
        let src = "fn main() { let n: u8 = 0; while n < 10 { n = n + 1; } }";
        let m = compile_source(src, "test").expect("ok");
        let o = ops(&m, "main");
        assert!(o.contains(&"cmp_lt".to_string()));
        assert!(o.contains(&"jmp_if_false".to_string()));
    }

    #[test]
    fn compiles_loop_break() {
        let src = "fn main() { loop { break; } }";
        let m = compile_source(src, "test").expect("ok");
        let o = ops(&m, "main");
        // loop_top label + loop_end label + body jmp + epilogue.
        assert!(o.iter().filter(|op| *op == "label").count() >= 2);
        assert!(o.contains(&"jmp".to_string()));
    }

    #[test]
    fn compiles_cross_function_call() {
        let src = "fn add(a: u8, b: u8) -> u8 { return a + b; } \
                   fn main() { let x: u8 = add(1, 2); }";
        let m = compile_source(src, "test").expect("ok");
        let main_o = ops(&m, "main");
        assert!(main_o.contains(&"call".to_string()),
                "main must contain a call instruction; got {main_o:?}");
        let add_o = ops(&m, "add");
        assert!(add_o.contains(&"add".to_string()));
        assert!(add_o.contains(&"ret".to_string()));
    }

    #[test]
    fn rejects_8008_intrinsic() {
        let src = "fn main() { let x: u8 = in(1); }";
        let err = compile_source(src, "test").unwrap_err();
        match err {
            OctError::Unsupported8008Intrinsic(name) => assert_eq!(name, "in"),
            other => panic!("expected Unsupported8008Intrinsic, got {other:?}"),
        }
    }

    #[test]
    fn rejects_carry_intrinsic() {
        let src = "fn main() { let b: bool = carry(); }";
        let err = compile_source(src, "test").unwrap_err();
        assert!(matches!(err, OctError::Unsupported8008Intrinsic(_)));
    }

    #[test]
    fn rejects_type_error() {
        // u8 → bool not allowed.
        let src = "fn main() { let x: u8 = 5; let y: bool = x; }";
        let err = compile_source(src, "test").unwrap_err();
        assert!(matches!(err, OctError::Type(_)),
                "expected Type error, got {err:?}");
    }

    #[test]
    fn rejects_parse_error() {
        let err = compile_source("fn main(", "test").unwrap_err();
        assert!(matches!(err, OctError::Parse(_)));
    }

    /// Recursion: type checker accepts it, and the IIR compiler emits the
    /// `call` opcode normally — the AOT chain's cross-function reloc patch
    /// from PR #3331 handles self-calls.
    #[test]
    fn compiles_recursive_function() {
        let src = "fn fact(n: u8) -> u8 { \
                     if n == 0 { return 1; } \
                     return n + fact(n); \
                   } \
                   fn main() { let x: u8 = fact(5); }";
        let m = compile_source(src, "test").expect("ok");
        // fact must contain at least one `call` to itself.
        let fact_o = ops(&m, "fact");
        assert!(fact_o.contains(&"call".to_string()),
                "fact must self-call; got {fact_o:?}");
    }

    // ── Source-map invariants (OCT05 — debugger prerequisite) ──────────

    #[test]
    fn source_map_lockstep_with_instructions() {
        // Every function's source_map must have exactly one entry per
        // instruction.  Without this invariant, the debugger's
        // sidecar can't map a paused IIR PC back to a source line.
        let m = compile_source(
            "fn main() { let x: u8 = 30; let y: u8 = 12; }",
            "test",
        ).expect("ok");
        for f in &m.functions {
            assert_eq!(
                f.source_map.len(),
                f.instructions.len(),
                "fn {} source_map ({}) must be lockstep with instructions ({})",
                f.name, f.source_map.len(), f.instructions.len(),
            );
        }
    }

    #[test]
    fn source_map_carries_real_line_numbers() {
        // The compiler should thread real source positions through the
        // emitted IIR, not just SYNTHETIC (line=0, col=0).  Without
        // real positions, line-based breakpoints cannot resolve.
        //
        // We construct a 3-line program and assert that at least one
        // instruction is tagged with each non-empty line.
        let src = "fn main() {\n\
                   let x: u8 = 30;\n\
                   let y: u8 = 12;\n\
                   }\n";
        let m = compile_source(src, "test").expect("ok");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        let lines_seen: std::collections::BTreeSet<u32> = main.source_map.iter()
            .filter(|l| **l != SourceLoc::SYNTHETIC)
            .map(|l| l.line)
            .collect();
        // We expect at least lines 2 and 3 to be tagged (the two let
        // statements).  The synthesised main epilogue may carry line 1
        // (the fn declaration) or be SYNTHETIC — either is acceptable.
        assert!(lines_seen.contains(&2),
            "expected line 2 (first let stmt) to appear in source_map; got: {lines_seen:?}");
        assert!(lines_seen.contains(&3),
            "expected line 3 (second let stmt) to appear in source_map; got: {lines_seen:?}");
    }

    #[test]
    fn out_intrinsic_lowers_to_print_i64() {
        // LANG-FULL O-OUT: `out(port, value)` prints `value` to stdout via
        // `call_builtin "print_i64"` (the value, not the port).
        let m = compile_source("fn main() { out(1, 200); }", "test").expect("ok");
        let body = &m.functions[0].instructions;
        let has_print = body.iter().any(|i| i.op == "call_builtin"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "print_i64"));
        assert!(has_print, "out() must emit call_builtin print_i64; got {body:?}");
        // The printed value 200 is materialised.
        assert!(body.iter().any(|i| i.op == "const"
            && matches!(i.srcs.first(), Some(Operand::Int(200)))),
            "the printed value 200 must be a const; got {body:?}");
    }

    #[test]
    fn out_of_computed_value() {
        // `out(1, 100 + 100)` computes the value first (an `add`), then prints it.
        let m = compile_source("fn main() { out(1, 100 + 100); }", "test").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "add"), "out() arg must compute via add; got {body:?}");
        assert!(body.iter().any(|i| i.op == "call_builtin"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "print_i64")),
            "out() must print the computed value; got {body:?}");
    }

    #[test]
    fn other_intrinsics_still_rejected() {
        // Only `out` is wired (it's the output port). `in` and the arithmetic/rotation
        // intrinsics have no general-backend model yet and stay cleanly rejected.
        for src in [
            "fn main() { let x: u8 = in(1); }",
            "fn main() { let x: u8 = adc(1, 2); }",
            "fn main() { let x: u8 = rlc(1); }",
        ] {
            let err = compile_source(src, "test").unwrap_err();
            assert!(matches!(err, OctError::Unsupported8008Intrinsic(_)),
                "expected Unsupported8008Intrinsic for {src:?}; got {err:?}");
        }
    }

    #[test]
    fn logical_and_short_circuits() {
        // LANG-FULL O1: `a && b` lowers to a result slot guarded by a `jmp_if_false`
        // BEFORE the right operand — so the right side runs only when the left is true.
        // The two operands are comparisons (`cmp_eq`); the second must be emitted AFTER
        // the short-circuit guard.
        let m = compile_source(
            "fn main() { if 1 == 2 && 3 == 4 { out(1, 1); } else { out(1, 0); } }", "test",
        ).expect("ok");
        let ops: Vec<&str> = m.functions.iter().find(|f| f.name == "main").unwrap()
            .instructions.iter().map(|i| i.op.as_str()).collect();
        let first_guard = ops.iter().position(|o| *o == "jmp_if_false").expect("a jmp_if_false");
        let cmps: Vec<usize> =
            ops.iter().enumerate().filter(|(_, o)| **o == "cmp_eq").map(|(i, _)| i).collect();
        assert_eq!(cmps.len(), 2, "both operands compiled; got {ops:?}");
        assert!(cmps[1] > first_guard,
            "right operand must be guarded by the short-circuit jmp_if_false; got {ops:?}");
        // The old eager lowering used a bitwise `and` on the two bools — must be gone.
        assert!(!ops.contains(&"and"),
            "&& must not lower to an eager bitwise `and`; got {ops:?}");
    }

    #[test]
    fn logical_or_short_circuits() {
        let m = compile_source(
            "fn main() { if 1 == 1 || 3 == 4 { out(1, 1); } else { out(1, 0); } }", "test",
        ).expect("ok");
        let ops: Vec<&str> = m.functions.iter().find(|f| f.name == "main").unwrap()
            .instructions.iter().map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"jmp_if_false"), "|| must emit a short-circuit guard; got {ops:?}");
        assert!(ops.contains(&"jmp"), "|| must emit the short-circuit jump; got {ops:?}");
        assert!(!ops.contains(&"or"), "|| must not lower to an eager bitwise `or`; got {ops:?}");
    }

    #[test]
    fn logical_not_lowers_to_truthiness_branch() {
        let m = compile_source(
            "fn main() { if !(1 == 2) { out(1, 42); } else { out(1, 0); } }", "test",
        ).expect("ok");
        let body = &m.functions.iter().find(|f| f.name == "main").unwrap().instructions;
        let ops: Vec<&str> = body.iter().map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"jmp_if_false"), "! must test truthiness via branch; got {ops:?}");
        assert!(ops.contains(&"jmp"), "! must skip the false arm after emitting 0; got {ops:?}");
        assert!(ops.contains(&"label"), "! must join the branch result; got {ops:?}");
        assert!(!ops.contains(&"not"),
            "! must not lower to bitwise not (`not 0` = -1, `not 1` = -2); got {ops:?}");
        let const_values: Vec<i64> = body.iter().filter_map(|i| {
            if i.op == "const" {
                match i.srcs.first() {
                    Some(Operand::Int(v)) => Some(*v),
                    _ => None,
                }
            } else {
                None
            }
        }).collect();
        assert!(const_values.contains(&0), "! lowering must materialise false as 0");
        assert!(const_values.contains(&1), "! lowering must materialise true as 1");
    }

    #[test]
    fn typed_function_return_materialises_as_i64() {
        // A non-void user function's signature must be `i64` (not the narrow source
        // `u8`), matching the i64 value its body returns — else the IIR-to-LLVM backend
        // emits `define i8 @f()` and the `ret` mismatches (LANG-FULL O1 fix).
        let m = compile_source("fn answer() -> u8 { return 42; } fn main() { out(1, answer()); }", "test")
            .expect("ok");
        let f = m.functions.iter().find(|f| f.name == "answer").expect("answer fn");
        assert_eq!(f.return_type, "i64", "a typed return must materialise as i64; got {:?}", f.return_type);
    }
}
