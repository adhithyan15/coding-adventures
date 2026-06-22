//! # `dartmouth-basic-iir-compiler` — Dartmouth BASIC → `IIRModule`.
//!
//! Lowers parsed 1964 Dartmouth BASIC into the language-agnostic
//! [`interpreter_ir::IIRModule`] consumed by the LANG VM AOT chain
//! (twig-aot / lang-aot → x86_64-backend / aarch64-backend → object →
//! system linker → native executable).
//!
//! Distinct from the existing `dartmouth-basic-ir-compiler` crate,
//! which targets the GE-225 simulator's custom `compiler_ir::IrProgram`
//! shape — that IR is **not** pluggable into the LANG VM chain.  PL05
//! introduces this *new* crate that emits `IIRModule` directly so
//! BASIC programs get the same Linux / Windows / macOS native
//! pipeline Twig and Nib enjoy.
//!
//! ## V1 scope
//!
//! Integer BASIC only — floats are deferred until the backends grow
//! SSE2 support.  Programs that use floating-point literals (`3.14`)
//! get the integer-cast (`3`) silently.
//!
//! | Statement     | Lowering |
//! |---------------|----------|
//! | `LET A = expr` | `<eval expr → t>; mov_i64 A = t` |
//! | `PRINT expr`   | `<eval expr → v>; call_builtin "print_i64", v` |
//! | `INPUT X`      | `call_builtin "input_i64" -> X` |
//! | `IF cond THEN m` | `<eval cond → c>; jmp_if_true c, "line_m"` |
//! | `GOTO m`       | `jmp "line_m"` |
//! | `FOR I = a TO b STEP s` / `NEXT I` | classic counter loop with `for_<n>_test` / `for_<n>_end` labels |
//! | `END`          | `const_i64 0 -> r; ret r` |
//! | `REM …`        | no-op |
//! | `GOSUB` / `RETURN` | **deferred** — V1 returns `UnsupportedStatement` |
//! | `READ` / `DATA` / `RESTORE` | **deferred** — needs data pool |
//! | `DIM A(n)`     | `alloc_array A = <n+1>` (0-based, inclusive) — BA3 / E5 |
//! | `LET A(i)=e`   | `<eval i → x>; <eval e → v>; array_set A, x, v` (BA3) |
//! | `A(i)` (rvalue) | `<eval i → x>; array_get A, x -> t` (BA3) |
//! | `STOP`         | same as `END` for V1 |
//! | `DEF FNx(P)=e` | sibling `IIRFunction` + `call` (BA5); `FNx(arg)` → `call` |
//!
//! ## Strings
//!
//! Strings (e.g. `PRINT "HELLO"`) are deferred — they need LANG77
//! `.rodata` support.  V1 errors out cleanly on `PRINT "…"`.
//!
//! ## Variables
//!
//! BASIC's `A..Z` and `A0..Z9` variable names map 1:1 to IIR slot
//! names — the IIR compiler emits them directly.  An array `A` (declared
//! with `DIM A(n)`) uses the same-named register to hold its *handle*; a
//! scalar `A` and an array `A` are distinct variables in Dartmouth BASIC.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use coding_adventures_dartmouth_basic_parser::parse_dartmouth_basic;
use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use interpreter_ir::source_loc::SourceLoc;

// Re-export the JIT backend so downstream consumers (tests, future
// `dartmouth-basic-vm` wrappers) can use it without depending on the
// internal module path.
pub mod jit_backend;
pub use jit_backend::{BasicCirJit, DEFAULT_OUTPUT_CAP, DEFAULT_STEP_CAP};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

/// Extract a [`SourceLoc`] from a `GrammarASTNode`, falling back to
/// [`SourceLoc::SYNTHETIC`] when the parser couldn't attach position
/// info (rare — mostly synthesised wrapper nodes).
///
/// Used by the BASIC compiler to tag every emitted IIR instruction
/// with the source position of the AST node that produced it.  The
/// resulting `IIRFunction.source_map` powers line-based breakpoints
/// in the future `basic-dap` debugger crate and source-line reporting
/// in stack traces.
fn node_loc(node: &GrammarASTNode) -> SourceLoc {
    match (node.start_line, node.start_column) {
        (Some(line), Some(col)) => SourceLoc::new(line, col),
        _ => SourceLoc::SYNTHETIC,
    }
}

// ===========================================================================
// Public surface
// ===========================================================================

/// Errors raised by the compiler.
#[derive(Debug)]
#[allow(dead_code)]
pub enum CompileError {
    /// V1 doesn't support this statement family (GOSUB/RETURN/READ/DIM/DEF).
    UnsupportedStatement(String),
    /// A BASIC construct exists in source but V1 doesn't lower it (e.g.
    /// string literals, array indexing, user-defined functions).
    Unsupported(String),
    /// AST shape didn't match our expectations — a parser change probably
    /// requires updating this crate.
    Malformed(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::UnsupportedStatement(s) =>
                write!(f, "dartmouth-basic V1 doesn't compile {s} statements yet"),
            CompileError::Unsupported(s) =>
                write!(f, "dartmouth-basic V1 doesn't compile {s} yet"),
            CompileError::Malformed(s) =>
                write!(f, "dartmouth-basic AST malformed: {s}"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile a Dartmouth BASIC source string to an [`IIRModule`] ready
/// for the LANG VM AOT chain.
///
/// The module contains exactly one function named `main` whose return
/// type is `i64` (so its return value becomes the process exit code
/// per the LANG VM AOT entry-point convention).
pub fn compile_source(source: &str, module_name: &str)
    -> Result<IIRModule, CompileError>
{
    let ast = parse_dartmouth_basic(source);
    compile_program(&ast, module_name)
}

// ===========================================================================
// Compiler core
// ===========================================================================

/// Compile a parsed `program` AST node into an `IIRModule`.
fn compile_program(ast: &GrammarASTNode, module_name: &str)
    -> Result<IIRModule, CompileError>
{
    let mut comp = Compiler::default();
    comp.emit_program(ast)?;
    // Override `IIRFunction::new`'s automatic `infer_type_status` —
    // which returns `PartiallyTyped` because BASIC's control-flow ops
    // (label, jmp, jmp_if_*, ret, call_builtin "print_i64") use
    // `"void"` hints and `"void"` is not in `CONCRETE_TYPES`.  Every
    // BASIC instruction is in fact statically known (no `"any"` hints
    // anywhere), so the function is genuinely fully typed for the
    // JIT's threshold-zero compile path.  This mirrors Brainfuck's
    // `IIRFunction { … type_status: FullyTyped, … }` construction.
    let body_len = comp.instrs.len();
    let mut main = IIRFunction::new(
        "main",
        vec![],   // no parameters
        "i64",    // return i64 for the exit code
        comp.instrs,
    );
    main.type_status = FunctionTypeStatus::FullyTyped;
    // Move the accumulated source positions onto the function.  The
    // lockstep invariant (one entry per instruction) is enforced by
    // [`Compiler::emit`]: every push to `instrs` pairs with a push to
    // `source_map`.  We defensively pad with `SYNTHETIC` in case any
    // pre-source_map code path slipped through (this branch is dead
    // today but cheap to keep).
    while comp.source_map.len() < body_len {
        comp.source_map.push(SourceLoc::SYNTHETIC);
    }
    if comp.source_map.len() > body_len {
        comp.source_map.truncate(body_len);
    }
    main.source_map = std::mem::take(&mut comp.source_map);
    let mut module = IIRModule::new(module_name, "dartmouth-basic");
    module.functions.push(main);
    // Sibling user-defined functions (`DEF FNx`, BA5) follow `main`.  They
    // were lowered out of line into their own emission contexts during
    // `emit_program`; a same-module `call` resolves each by name.
    for func in std::mem::take(&mut comp.functions) {
        module.functions.push(func);
    }
    module.entry_point = Some("main".to_string());
    Ok(module)
}

struct Compiler {
    instrs: Vec<IIRInstr>,
    /// Next synthetic register name (`_t0`, `_t1`, …) for temporaries.
    temp_counter: usize,
    /// Maps each `FOR I` to its limit / step / test-label / end-label so
    /// the matching `NEXT I` can close the loop.  Stack discipline is
    /// straightforward because BASIC's FOR/NEXT are properly nested in
    /// every example from the 1964 manual.
    open_fors: Vec<ForState>,
    /// Counter for unique `for_<n>` label families.
    for_counter: usize,
    /// Per-instruction source positions, built in lockstep with
    /// `instrs`.  Moved onto `IIRFunction.source_map` at end of
    /// [`compile_program`].
    source_map: Vec<SourceLoc>,
    /// "Currently compiling" source position.  Updated by every
    /// statement-level entry point (`emit_line`, `emit_statement`)
    /// and read by [`emit`] when it appends to the instruction
    /// stream.  Using a [`Cell`](std::cell::Cell) so a future move to
    /// `emit(&self, ...)` would not require a re-shuffle of mut
    /// signatures.
    current_loc: std::cell::Cell<SourceLoc>,
    /// Sibling `IIRFunction`s lowered out of line — one per `DEF FNx`
    /// user-defined function (BA5).  Pushed onto the module *after*
    /// `main`, so a same-module `call` resolves the callee by name.
    functions: Vec<IIRFunction>,
    /// Set of `USER_FN` names (`fna`..`fnz`) declared anywhere in the
    /// program, collected in a pre-pass so a call site can be lowered
    /// before the `DEF` line is reached (BASIC permits forward use).
    defined_fns: std::collections::HashSet<String>,
    /// When lowering a `DEF` body this holds the function's single
    /// formal parameter name.  A body may reference *only* its
    /// parameter (and numeric literals / other functions) — global
    /// access from inside a function needs the host global table the
    /// code-gen backends reject (enabler **E6**), so any other variable
    /// reference is a clean `Unsupported` error rather than an
    /// undefined-register miscompile.  `None` while lowering `main`.
    current_fn_param: Option<String>,
    /// Names declared as arrays via `DIM` (BA3 / enabler **E5**).  Each
    /// name maps to the IIR register holding the array *handle* — we use
    /// the BASIC variable name itself as that register (an array `A` and a
    /// scalar `A` are different variables in Dartmouth BASIC, and tiny
    /// programs never collide them).  Membership also tells the `LET` and
    /// expression paths whether a subscripted `A(I)` is a real array
    /// access (→ `array_set`/`array_get`) or an undeclared use (an error).
    arrays: std::collections::HashSet<String>,
    /// The `DATA` pool (BA6): every numeric literal from every `DATA`
    /// statement, gathered in line-number order by a pre-pass.  `READ`
    /// consumes these sequentially through a run-time pointer; `RESTORE`
    /// rewinds the pointer.  Because the BASIC program is a single `main`
    /// function (no `GOSUB` yet), the pool is materialised once at the top of
    /// `main` as an `array<i64>` ([[E5]] arrays) plus an `i64` pointer
    /// register — no module global is needed.  Integer literals only in v1;
    /// a real (`f64`) DATA value is a follow-up (the i64 value model, like
    /// integer `DIM` arrays).
    data_pool: Vec<i64>,
}

impl Default for Compiler {
    fn default() -> Self {
        Compiler {
            instrs: Vec::new(),
            temp_counter: 0,
            open_fors: Vec::new(),
            for_counter: 0,
            source_map: Vec::new(),
            current_loc: std::cell::Cell::new(SourceLoc::SYNTHETIC),
            functions: Vec::new(),
            defined_fns: std::collections::HashSet::new(),
            current_fn_param: None,
            arrays: std::collections::HashSet::new(),
            data_pool: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // limit_slot kept for future use (e.g. cmp_ge on negative STEP)
struct ForState {
    var: String,
    limit_slot: String,
    step_slot: String,
    test_label: String,
    end_label: String,
}

impl Compiler {
    fn fresh_temp(&mut self) -> String {
        let i = self.temp_counter;
        self.temp_counter += 1;
        format!("_t{i}")
    }

    fn emit(&mut self, op: &str, dest: Option<&str>,
            srcs: Vec<Operand>, type_hint: &str)
    {
        self.instrs.push(IIRInstr::new(op,
            dest.map(|s| s.to_string()), srcs, type_hint));
        // Tag this instruction with the "currently compiling" source
        // position.  Statement-level entry points (`emit_line`,
        // `emit_statement`) set `self.current_loc` so every
        // instruction emitted while compiling that statement —
        // including from sub-expressions — inherits the statement's
        // source line.
        //
        // Line-based debuggers care about which line a breakpoint
        // sits on, not the per-expression column.  Statement-level
        // granularity is both sufficient and cheaper than per-
        // expression threading.
        self.source_map.push(self.current_loc.get());
    }

    /// Update the "currently compiling" source position.  Subsequent
    /// [`emit`] calls tag their instructions with this position via
    /// the `source_map` field.
    fn set_loc(&self, loc: SourceLoc) {
        self.current_loc.set(loc);
    }

    fn emit_program(&mut self, ast: &GrammarASTNode) -> Result<(), CompileError> {
        if ast.rule_name != "program" {
            return Err(CompileError::Malformed(format!(
                "expected root `program`, got `{}`", ast.rule_name)));
        }

        // Default position for instructions emitted before any line
        // (e.g. the synthesised end-of-program epilogue): the program
        // root's own position.
        self.set_loc(node_loc(ast));

        // Pre-pass — register every `DEF FNx` name *before* lowering any
        // statement.  BASIC lets a program call `FNS(7)` on an earlier
        // line than the `DEF FNS(X) = …` that defines it, so a call site
        // must be able to resolve the name even though its body has not
        // been compiled yet.
        for child in &ast.children {
            if let ASTNodeOrToken::Node(line) = child {
                if line.rule_name == "line" {
                    self.register_def_names(line);
                }
            }
        }

        // Pre-pass — gather the `DATA` pool (BA6).  Every `DATA` statement's
        // numeric literals are collected across the whole program in
        // line-number order (the children are already in source order), so a
        // `READ` on an earlier line can consume a value from a `DATA` on a
        // later line — exactly the BASIC semantics.
        for child in &ast.children {
            if let ASTNodeOrToken::Node(line) = child {
                if line.rule_name == "line" {
                    self.collect_data(line)?;
                }
            }
        }
        // Materialise the pool once at the top of `main`: an `array<i64>` of
        // the literals plus the `__basic_data_ptr` register initialised to 0.
        self.emit_data_pool_init();

        // Walk every `line` child, in order.
        for child in &ast.children {
            if let ASTNodeOrToken::Node(line) = child {
                if line.rule_name == "line" {
                    self.emit_line(line)?;
                }
            }
        }

        // Defensive epilogue: if the program didn't explicitly END, fall
        // through to a `const 0; ret 0` so we don't run off the function.
        // Most well-formed BASIC programs do end with `END`.
        if !self.instrs.last().map_or(false, |i| i.op.starts_with("ret")) {
            self.emit_end();
        }
        Ok(())
    }

    fn emit_line(&mut self, line: &GrammarASTNode) -> Result<(), CompileError> {
        // Tag every instruction emitted while compiling this line
        // (including the synthetic `label line_N` and the statement
        // body) with the line's source position.  The inner
        // `emit_statement` may overwrite this with the wrapped
        // statement node's own position — typically the same line.
        self.set_loc(node_loc(line));
        let line_num = extract_line_num(line)
            .ok_or_else(|| CompileError::Malformed(
                "line missing LINE_NUM token".into()))?;
        // Every line becomes a label `line_<n>`.  Forward references
        // (GOTO 100 before line 100 appears) work because the label is
        // emitted at the right point in the instruction stream — the
        // backend's pre-pass collects labels first.
        let label = format!("line_{line_num}");
        self.emit("label", None, vec![Operand::Var(label)], "void");

        // The optional `statement` child.
        let stmt = child_nodes(line).into_iter()
            .find(|n| n.rule_name == "statement");
        let Some(stmt) = stmt else {
            // Bare line number (no statement) — like a BASIC comment.
            return Ok(());
        };
        // Statement node wraps exactly one of the *_stmt children.
        let inner = child_nodes(stmt).into_iter().next()
            .ok_or_else(|| CompileError::Malformed(
                "statement missing inner node".into()))?;
        self.emit_statement(inner)
    }

    fn emit_statement(&mut self, stmt: &GrammarASTNode)
        -> Result<(), CompileError>
    {
        // Re-tag the "currently compiling" loc with the inner
        // statement node's own position.  In practice this is almost
        // always the same line as `emit_line` already set, but the
        // grammar permits multiple statements on one line (via `:`
        // separators in some BASIC dialects) and the AST may have a
        // tighter (line, col) range for the inner node — so the more
        // specific one wins.
        self.set_loc(node_loc(stmt));
        match stmt.rule_name.as_str() {
            "let_stmt"     => self.emit_let(stmt),
            "print_stmt"   => self.emit_print(stmt),
            "input_stmt"   => self.emit_input(stmt),
            "if_stmt"      => self.emit_if(stmt),
            "goto_stmt"    => self.emit_goto(stmt),
            "for_stmt"     => self.emit_for(stmt),
            "next_stmt"    => self.emit_next(stmt),
            "end_stmt" | "stop_stmt"
                           => { self.emit_end(); Ok(()) },
            "rem_stmt"     => Ok(()),
            // Explicitly-deferred V1 statements.
            "gosub_stmt"   => Err(CompileError::UnsupportedStatement("GOSUB".into())),
            "return_stmt"  => Err(CompileError::UnsupportedStatement("RETURN".into())),
            "read_stmt"    => self.emit_read(stmt),
            // A `DATA` statement emits nothing at its position — its values
            // were gathered into the pool by the `collect_data` pre-pass and
            // materialised once at the top of `main` (BA6).
            "data_stmt"    => Ok(()),
            "restore_stmt" => self.emit_restore(),
            "dim_stmt"     => self.emit_dim(stmt),
            "def_stmt"     => self.emit_def(stmt),
            other => Err(CompileError::Malformed(
                format!("unknown statement `{other}`"))),
        }
    }

    // -- Per-statement emitters --------------------------------------------

    fn emit_let(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `LET` KEYWORD, `variable` node, `EQ` token, `expr` node.
        let var_node = child_nodes(stmt).into_iter()
            .find(|n| n.rule_name == "variable")
            .ok_or_else(|| CompileError::Malformed("LET missing variable".into()))?;
        let expr_node = child_nodes(stmt).into_iter()
            .find(|n| n.rule_name == "expr")
            .ok_or_else(|| CompileError::Malformed("LET missing expr".into()))?;

        // `LET A(I) = e` stores into an array element (BA3 / E5); `LET x = e`
        // is a plain register move.  We compute the right-hand side *first* in
        // both cases so the index expression's temporaries don't interleave
        // with it confusingly — the order is irrelevant to correctness (no
        // shared state) but keeps the emitted IR readable.
        if let Some(index_expr) = array_subscript_index(var_node) {
            let name = array_target_name(var_node)?;
            if !self.arrays.contains(&name) {
                return Err(CompileError::Unsupported(format!(
                    "assignment to `{name}(...)` but `{name}` was never DIMmed \
                     — declare it with `DIM {name}(n)` first")));
            }
            let idx = self.emit_expr(index_expr)?;
            let val = self.emit_expr(expr_node)?;
            // `array_set handle, idx, value` — BASIC subscripts are already
            // 0-based (`DIM A(N)` gives `A(0)..A(N)`), so the subscript IS the
            // IIR index; no lower-bound subtraction (unlike ALGOL's `[lo:hi]`).
            self.emit("array_set", None,
                vec![Operand::Var(name), Operand::Var(idx), Operand::Var(val)],
                "i64");
            return Ok(());
        }

        let var_name = scalar_variable_name(var_node)?;
        let val = self.emit_expr(expr_node)?;
        // `mov_i64 dest = src` — the backend handles this via `emit_binop`/etc.
        self.emit("mov", Some(&var_name),
                  vec![Operand::Var(val)], "i64");
        Ok(())
    }

    /// Lower `DIM A(n) [, B(m) …]` to one `alloc_array` per declared name
    /// (BA3 / enabler **E5**).
    ///
    /// Dartmouth BASIC arrays are **0-based and inclusive**: `DIM A(10)`
    /// declares the eleven elements `A(0)` through `A(10)`.  So the element
    /// count `alloc_array` needs is `n + 1`, and a subscript needs no
    /// adjustment — `A(I)` indexes element `I` directly.  (Contrast ALGOL's
    /// `array A[lo:hi]`, which carries an arbitrary lower bound and subtracts
    /// it on every access.)  Each handle lives in the register named after the
    /// BASIC array, and the name is recorded so `LET`/expression subscripts
    /// resolve to `array_set`/`array_get`.
    fn emit_dim(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        for decl in child_nodes(stmt).into_iter()
            .filter(|n| n.rule_name == "dim_decl")
        {
            // `dim_decl = NAME LPAREN NUMBER RPAREN`.
            let name = first_name_token_value(decl).ok_or_else(|| {
                CompileError::Malformed("DIM decl missing array name".into())
            })?;
            let max_sub = dim_decl_bound(decl)?;
            if max_sub < 0 {
                return Err(CompileError::Unsupported(format!(
                    "DIM {name}({max_sub}) — array bound must be non-negative")));
            }
            // Element count = max subscript + 1 (inclusive, 0-based).
            // `dim_decl_bound` already rejects bounds above `MAX_DIM_BOUND`, so
            // this `+ 1` cannot overflow; the `checked_add` is a belt-and-braces
            // guard that keeps the compile path panic-free regardless.
            let count = max_sub.checked_add(1).ok_or_else(|| {
                CompileError::Unsupported(format!(
                    "DIM {name}({max_sub}) — array bound too large"))
            })?;
            let len = self.fresh_temp();
            self.emit("const", Some(&len),
                vec![Operand::Int(count)], "i64");
            self.emit("alloc_array", Some(&name),
                vec![Operand::Var(len)], "array<i64>");
            self.arrays.insert(name);
        }
        Ok(())
    }

    /// BA6 pre-pass: append a `DATA` statement's numeric literals to
    /// [`data_pool`].  Called once per line before any statement is lowered,
    /// so the pool ends up in line-number (source) order.  Integer literals
    /// only — a non-integer `DATA` value is a clean `Unsupported` error (real
    /// DATA is a follow-up, like integer-only `DIM` arrays).
    fn collect_data(&mut self, line: &GrammarASTNode) -> Result<(), CompileError> {
        let Some(stmt) = child_nodes(line).into_iter()
            .find(|n| n.rule_name == "statement")
        else { return Ok(()); };
        let Some(inner) = child_nodes(stmt).into_iter().next() else { return Ok(()); };
        if inner.rule_name != "data_stmt" { return Ok(()); }
        // `data_stmt = "DATA" NUMBER { COMMA NUMBER }` — collect every NUMBER
        // token (the `DATA` keyword and `,` separators are not NUMBERs).
        for c in &inner.children {
            if let ASTNodeOrToken::Token(t) = c {
                if t.effective_type_name() != "NUMBER" { continue; }
                let raw = t.value.trim();
                let f = raw.parse::<f64>().map_err(|_| CompileError::Malformed(
                    format!("DATA value `{raw}` is not a number")))?;
                if !f.is_finite() || f.fract() != 0.0 {
                    return Err(CompileError::Unsupported(format!(
                        "non-integer DATA value `{raw}` — only integer DATA is \
                         supported so far (real DATA is a follow-up)")));
                }
                // `f.fract() == 0.0` and finite, so the cast is exact for any
                // value an `f64` can represent; guard the i64 range explicitly.
                if f < i64::MIN as f64 || f > i64::MAX as f64 {
                    return Err(CompileError::Unsupported(format!(
                        "DATA value `{raw}` is out of the i64 range")));
                }
                self.data_pool.push(f as i64);
            }
        }
        Ok(())
    }

    /// BA6: materialise the gathered `DATA` pool at the top of `main`.  Emits
    /// nothing when there is no `DATA`.  Otherwise allocates an `array<i64>`
    /// of the literals (one `array_set` per value) and seeds the read pointer
    /// `__basic_data_ptr` to 0.  Both live in `main`'s register file — the
    /// program is a single function, so no module global is needed for the
    /// pointer to persist across `READ`s.
    fn emit_data_pool_init(&mut self) {
        if self.data_pool.is_empty() {
            return;
        }
        let count = self.data_pool.len() as i64;
        let len = self.fresh_temp();
        self.emit("const", Some(&len), vec![Operand::Int(count)], "i64");
        self.emit("alloc_array", Some(BASIC_DATA_ARRAY),
            vec![Operand::Var(len.clone())], "array<i64>");
        // Fill the pool.  `self.data_pool` is cloned to a local first so the
        // immutable borrow doesn't clash with the `&mut self` emits.
        let values: Vec<i64> = self.data_pool.clone();
        for (i, value) in values.into_iter().enumerate() {
            let idx = self.fresh_temp();
            self.emit("const", Some(&idx), vec![Operand::Int(i as i64)], "i64");
            let val = self.fresh_temp();
            self.emit("const", Some(&val), vec![Operand::Int(value)], "i64");
            self.emit("array_set", None,
                vec![Operand::Var(BASIC_DATA_ARRAY.into()),
                     Operand::Var(idx), Operand::Var(val)], "i64");
        }
        // Read pointer starts at the first value.
        let zero = self.fresh_temp();
        self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
        self.emit("mov", Some(BASIC_DATA_PTR),
            vec![Operand::Var(zero)], "i64");
    }

    /// BA6: `READ var { , var }` — consume the next `DATA` value(s) through
    /// the run-time pointer.  Each variable gets `array_get __basic_data,
    /// __basic_data_ptr`, then the pointer is advanced by 1.  Reading past the
    /// end of the pool traps (the bounds-checked `array_get`), which is the
    /// "out of DATA" run-time error.  A scalar target is a `mov`; an array
    /// element `READ A(I)` is an `array_set` (BA3).
    fn emit_read(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        if self.data_pool.is_empty() {
            return Err(CompileError::Unsupported(
                "READ with no DATA statement in the program".into()));
        }
        for var_node in child_nodes(stmt).into_iter()
            .filter(|n| n.rule_name == "variable")
        {
            // Fetch `__basic_data[__basic_data_ptr]`.
            let value = self.fresh_temp();
            self.emit("array_get", Some(&value),
                vec![Operand::Var(BASIC_DATA_ARRAY.into()),
                     Operand::Var(BASIC_DATA_PTR.into())], "i64");
            // Store it into the target — array element or scalar.
            if let Some(index_expr) = array_subscript_index(var_node) {
                let name = array_target_name(var_node)?;
                if !self.arrays.contains(&name) {
                    return Err(CompileError::Unsupported(format!(
                        "READ into `{name}(...)` but `{name}` was never DIMmed")));
                }
                let idx = self.emit_expr(index_expr)?;
                self.emit("array_set", None,
                    vec![Operand::Var(name), Operand::Var(idx),
                         Operand::Var(value)], "i64");
            } else {
                let target = scalar_variable_name(var_node)?;
                self.emit("mov", Some(&target),
                    vec![Operand::Var(value)], "i64");
            }
            // Advance the pointer: `__basic_data_ptr = __basic_data_ptr + 1`.
            let one = self.fresh_temp();
            self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
            self.emit("add", Some(BASIC_DATA_PTR),
                vec![Operand::Var(BASIC_DATA_PTR.into()), Operand::Var(one)],
                "i64");
        }
        Ok(())
    }

    /// BA6: `RESTORE` — rewind the `DATA` read pointer to the first value.
    fn emit_restore(&mut self) -> Result<(), CompileError> {
        if self.data_pool.is_empty() {
            // RESTORE with no DATA is harmless in BASIC (nothing to rewind);
            // keep it a clean error so a typo'd program isn't silently a no-op.
            return Err(CompileError::Unsupported(
                "RESTORE with no DATA statement in the program".into()));
        }
        let zero = self.fresh_temp();
        self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
        self.emit("mov", Some(BASIC_DATA_PTR), vec![Operand::Var(zero)], "i64");
        Ok(())
    }

    fn emit_print(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `PRINT` optional `print_list`.  V1: only numeric `print_item`s
        // are supported; strings error out.
        let Some(list) = child_nodes(stmt).into_iter()
            .find(|n| n.rule_name == "print_list")
        else {
            // Bare `PRINT` — emits a blank line.  For V1 we just no-op;
            // a future spec can emit `__twig_putchar(10)`.
            return Ok(());
        };
        for item in child_nodes(list).into_iter()
            .filter(|n| n.rule_name == "print_item")
        {
            let inner = child_nodes(item).into_iter().next();
            match inner {
                Some(expr_node) if expr_node.rule_name == "expr" => {
                    let v = self.emit_expr(expr_node)?;
                    self.emit("call_builtin", None,
                        vec![Operand::Var("print_i64".into()),
                             Operand::Var(v)],
                        "void");
                }
                _ => {
                    // The other allowed `print_item` form is STRING — a
                    // direct token child rather than a node.  We don't
                    // support strings until LANG77 lands.
                    return Err(CompileError::Unsupported(
                        "string literals in PRINT (need LANG77)".into()));
                }
            }
        }
        Ok(())
    }

    fn emit_input(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `INPUT` variable { COMMA variable }
        // For each variable, emit `call_builtin "input_i64" -> X`.
        // V1 only handles plain NAMEs (no array elements).
        for v in child_nodes(stmt).into_iter()
            .filter(|n| n.rule_name == "variable")
        {
            let name = scalar_variable_name(v)?;
            self.emit("call_builtin", Some(&name),
                vec![Operand::Var("input_i64".into())],
                "i64");
        }
        Ok(())
    }

    fn emit_if(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `IF` expr relop expr `THEN` NUMBER
        let exprs: Vec<&GrammarASTNode> = child_nodes(stmt).into_iter()
            .filter(|n| n.rule_name == "expr").collect();
        if exprs.len() != 2 {
            return Err(CompileError::Malformed(
                format!("IF expected 2 exprs, got {}", exprs.len())));
        }
        let relop_node = child_nodes(stmt).into_iter()
            .find(|n| n.rule_name == "relop")
            .ok_or_else(|| CompileError::Malformed("IF missing relop".into()))?;
        let cmp_op = extract_relop_op(relop_node)?;

        let lhs = self.emit_expr(exprs[0])?;
        let rhs = self.emit_expr(exprs[1])?;
        let cond = self.fresh_temp();
        // The `type_hint` on a comparison is the OPERAND width, not the (always
        // boolean) result — the IIR-to-* backends size the machine compare from
        // it (`i1 sgt` truncates to a 1-bit compare, the LANG-FULL BA0 bug).
        // BASIC's scalars materialise as `i64`, matching Nib/Oct/ALGOL.
        self.emit(cmp_op, Some(&cond),
            vec![Operand::Var(lhs), Operand::Var(rhs)],
            "i64");

        // THEN target — find the NUMBER token (after the THEN keyword).
        let target_line = extract_if_target(stmt)?;
        self.emit("jmp_if_true", None,
            vec![Operand::Var(cond), Operand::Var(format!("line_{target_line}"))],
            "void");
        Ok(())
    }

    fn emit_goto(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        let target = first_number_token(stmt)
            .ok_or_else(|| CompileError::Malformed("GOTO missing target".into()))?;
        self.emit("jmp", None,
            vec![Operand::Var(format!("line_{target}"))],
            "void");
        Ok(())
    }

    fn emit_for(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `FOR` NAME `=` expr `TO` expr [ `STEP` expr ]
        let var = first_name_token_value(stmt)
            .ok_or_else(|| CompileError::Malformed("FOR missing NAME".into()))?;
        let exprs: Vec<&GrammarASTNode> = child_nodes(stmt).into_iter()
            .filter(|n| n.rule_name == "expr").collect();
        if exprs.len() < 2 {
            return Err(CompileError::Malformed(
                "FOR needs at least 2 exprs (start, end)".into()));
        }
        let start_v = self.emit_expr(exprs[0])?;
        let limit_v = self.emit_expr(exprs[1])?;
        let step_v = if let Some(step_expr) = exprs.get(2) {
            self.emit_expr(step_expr)?
        } else {
            // STEP defaults to 1.
            let t = self.fresh_temp();
            self.emit("const", Some(&t), vec![Operand::Int(1)], "i64");
            t
        };

        // Stash limit and step in named slots so NEXT can read them later
        // (they're computed once at FOR entry, not re-evaluated each pass).
        let id = self.for_counter;
        self.for_counter += 1;
        let limit_slot = format!("_for_{id}_limit");
        let step_slot  = format!("_for_{id}_step");
        let test_label = format!("for_{id}_test");
        let end_label  = format!("for_{id}_end");

        self.emit("mov", Some(&var), vec![Operand::Var(start_v)], "i64");
        self.emit("mov", Some(&limit_slot), vec![Operand::Var(limit_v)], "i64");
        self.emit("mov", Some(&step_slot),  vec![Operand::Var(step_v)],  "i64");

        // Test label + per-iteration guard.
        self.emit("label", None, vec![Operand::Var(test_label.clone())], "void");
        let cond = self.fresh_temp();
        // BASIC FOR with a positive STEP is `var <= limit` to continue.
        // V1 doesn't check the step sign at compile time — if STEP is
        // negative, the program will exit the loop immediately because
        // `var <= limit` is false from the start.  This matches the
        // PL05 spec's "two-test loop semantics like the BASIC manual" —
        // the user can manually compute the right comparison.
        // Operand width is `i64` (see the IF note above) — `"bool"` made the
        // backends emit a 1-bit `i1` compare, breaking FOR on LLVM/WASM.
        self.emit("cmp_le", Some(&cond),
            vec![Operand::Var(var.clone()), Operand::Var(limit_slot.clone())],
            "i64");
        self.emit("jmp_if_false", None,
            vec![Operand::Var(cond), Operand::Var(end_label.clone())],
            "void");

        // Push this open FOR onto the stack so the matching NEXT can
        // close it.  Properly-nested BASIC programs work; mis-nested
        // ones (e.g. NEXT for the outer loop appearing before the
        // inner one's NEXT) will produce wrong IIR — that's a runtime
        // semantic question, not a parser-shape one.
        self.open_fors.push(ForState {
            var,
            limit_slot,
            step_slot,
            test_label,
            end_label,
        });
        Ok(())
    }

    fn emit_next(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        let var = first_name_token_value(stmt)
            .ok_or_else(|| CompileError::Malformed("NEXT missing NAME".into()))?;
        // Pop the matching FOR from the stack.
        let top = self.open_fors.pop()
            .ok_or_else(|| CompileError::Malformed(
                format!("NEXT {var} has no matching FOR")))?;
        if top.var != var {
            return Err(CompileError::Malformed(format!(
                "NEXT {var} doesn't match enclosing FOR {} (mis-nested loop?)",
                top.var)));
        }
        // Increment the loop variable by STEP and jump back to the test.
        let new_val = self.fresh_temp();
        self.emit("add", Some(&new_val),
            vec![Operand::Var(top.var.clone()), Operand::Var(top.step_slot)],
            "i64");
        self.emit("mov", Some(&top.var),
            vec![Operand::Var(new_val)], "i64");
        self.emit("jmp", None,
            vec![Operand::Var(top.test_label)], "void");
        self.emit("label", None,
            vec![Operand::Var(top.end_label)], "void");
        Ok(())
    }

    fn emit_end(&mut self) {
        let r = self.fresh_temp();
        self.emit("const", Some(&r), vec![Operand::Int(0)], "i64");
        self.emit("ret", None, vec![Operand::Var(r)], "i64");
    }

    // -- Expression emitter ------------------------------------------------
    //
    // The grammar's precedence cascade (expr > term > power > unary >
    // primary) means each rule has the shape `<higher> { OP <higher> }`.
    // We walk children pairwise, emit one IIR instruction per operator,
    // and produce a fresh temp for each intermediate.

    fn emit_expr(&mut self, node: &GrammarASTNode)
        -> Result<String, CompileError>
    {
        match node.rule_name.as_str() {
            "expr"  => self.emit_left_assoc_chain(node),
            "term"  => self.emit_left_assoc_chain(node),
            "power" => self.emit_power(node),
            "unary" => self.emit_unary(node),
            "primary" => self.emit_primary(node),
            _ => {
                // Fallthrough: unwrap single-child wrappers.
                let kids = child_nodes(node);
                if let Some(c) = kids.first() {
                    return self.emit_expr(c);
                }
                Err(CompileError::Malformed(format!(
                    "unknown expr rule `{}`", node.rule_name)))
            }
        }
    }

    fn emit_left_assoc_chain(&mut self, node: &GrammarASTNode)
        -> Result<String, CompileError>
    {
        // node = first_sub_expr { OP next_sub_expr }
        //
        // Children alternate: Node, Token, Node, Token, Node, …
        // The first must be a Node; subsequent come in (Token, Node) pairs.
        let mut iter = node.children.iter();
        let first = iter.next()
            .ok_or_else(|| CompileError::Malformed(
                format!("empty {}", node.rule_name)))?;
        let mut acc = match first {
            ASTNodeOrToken::Node(n) => self.emit_expr(n)?,
            ASTNodeOrToken::Token(_) =>
                return Err(CompileError::Malformed(
                    format!("{} starts with token", node.rule_name))),
        };

        loop {
            let op = match iter.next() {
                Some(ASTNodeOrToken::Token(t)) => t,
                None => break,
                _ => return Err(CompileError::Malformed(
                    format!("{}: expected operator token", node.rule_name))),
            };
            let rhs = match iter.next() {
                Some(ASTNodeOrToken::Node(n)) => self.emit_expr(n)?,
                _ => return Err(CompileError::Malformed(
                    format!("{}: dangling operator", node.rule_name))),
            };
            let dest = self.fresh_temp();
            let cir_op = binary_op_name(&op.value)
                .ok_or_else(|| CompileError::Unsupported(
                    format!("operator `{}`", op.value)))?;
            self.emit(cir_op, Some(&dest),
                vec![Operand::Var(acc), Operand::Var(rhs)],
                "i64");
            acc = dest;
        }
        Ok(acc)
    }

    fn emit_power(&mut self, node: &GrammarASTNode)
        -> Result<String, CompileError>
    {
        // `power = unary [ CARET power ]` — right-associative.  V1 doesn't
        // support exponentiation (would need a runtime helper or
        // repeated mul); rejects with `Unsupported`.
        let kids = node.children.iter().collect::<Vec<_>>();
        if kids.len() == 1 {
            // Pass through to the single `unary` child.
            if let ASTNodeOrToken::Node(n) = kids[0] {
                return self.emit_expr(n);
            }
        }
        // CARET present in children → exponentiation, deferred.
        if kids.iter().any(|c| matches!(c, ASTNodeOrToken::Token(t)
            if t.value == "^"))
        {
            return Err(CompileError::Unsupported(
                "exponentiation (^) — needs runtime helper".into()));
        }
        // Otherwise just unwrap the single Node child.
        for c in kids {
            if let ASTNodeOrToken::Node(n) = c {
                return self.emit_expr(n);
            }
        }
        Err(CompileError::Malformed("empty `power` node".into()))
    }

    fn emit_unary(&mut self, node: &GrammarASTNode)
        -> Result<String, CompileError>
    {
        // `unary = MINUS primary | primary`.
        let has_minus = node.children.iter().any(|c| matches!(c,
            ASTNodeOrToken::Token(t) if t.value == "-"));
        let inner_node = child_nodes(node).into_iter().next()
            .ok_or_else(|| CompileError::Malformed("empty `unary`".into()))?;
        let v = self.emit_expr(inner_node)?;
        if !has_minus { return Ok(v); }
        // Emit `neg dest, v`.
        let dest = self.fresh_temp();
        self.emit("neg", Some(&dest), vec![Operand::Var(v)], "i64");
        Ok(dest)
    }

    fn emit_primary(&mut self, node: &GrammarASTNode)
        -> Result<String, CompileError>
    {
        // primary = NUMBER | BUILTIN_FN(expr) | USER_FN(expr) | variable | (expr)
        //
        // V1 supports NUMBER, `variable`, and `USER_FN(expr)` calls (BA5).
        // Built-in maths functions (SIN/ABS/…) stay deferred until E3 (reals).
        for c in &node.children {
            match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NUMBER" => {
                    // Truncate floats to i64 — V1 is integer-only.
                    let v: i64 = t.value.trim().parse::<f64>()
                        .map(|f| f as i64)
                        .unwrap_or(0);
                    let dest = self.fresh_temp();
                    self.emit("const", Some(&dest),
                        vec![Operand::Int(v)], "i64");
                    return Ok(dest);
                }
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "USER_FN" => {
                    // `USER_FN LPAREN expr RPAREN` — a call to a user-defined
                    // function (BA5).  Lower to the same IIR `call` convention
                    // ALGOL's value procedures use: `call dest = callee, arg`.
                    return self.emit_user_fn_call(&t.value, node);
                }
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "BUILTIN_FN" => {
                    return Err(CompileError::Unsupported(format!(
                        "built-in function `{}` (needs E3 reals)", t.value)));
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "variable" => {
                    // `A(I)` reads an array element (BA3 / E5) → `array_get`.
                    if let Some(index_expr) = array_subscript_index(n) {
                        let name = array_target_name(n)?;
                        if !self.arrays.contains(&name) {
                            return Err(CompileError::Unsupported(format!(
                                "`{name}(...)` is read but `{name}` was never \
                                 DIMmed — declare it with `DIM {name}(n)` first")));
                        }
                        let idx = self.emit_expr(index_expr)?;
                        let dest = self.fresh_temp();
                        self.emit("array_get", Some(&dest),
                            vec![Operand::Var(name), Operand::Var(idx)], "i64");
                        return Ok(dest);
                    }
                    let name = scalar_variable_name(n)?;
                    // Inside a `DEF` body, only the formal parameter is in
                    // scope.  Referencing any other variable would read an
                    // undefined register on the code-gen backends (a function
                    // can't see `main`'s globals without the host global table
                    // those backends reject — enabler E6), so reject it
                    // cleanly instead of miscompiling.
                    if let Some(param) = &self.current_fn_param {
                        if &name != param {
                            return Err(CompileError::Unsupported(format!(
                                "DEF FN body references `{name}` — only its \
                                 parameter `{param}` is in scope (global access \
                                 from a function needs enabler E6)")));
                        }
                    }
                    return Ok(name);
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "expr" => {
                    return self.emit_expr(n);
                }
                _ => {}
            }
        }
        Err(CompileError::Malformed(format!(
            "unrecognised `primary` shape: children={}", node.children.len())))
    }

    /// Lower a user-defined function *call* `FNx(arg)` to an IIR `call`.
    ///
    /// `name` is the `USER_FN` token value (`fns`, `fna`, …); `node` is the
    /// enclosing `primary` whose single `expr` child is the argument.  The
    /// emitted instruction is `call dest = [Var(name), Var(arg)]` with an
    /// `i64` return hint — the calling convention every backend understands
    /// (`srcs[0]` names the callee, the rest are argument slots).  This is
    /// the BASIC counterpart of ALGOL's value-procedure calls (AL3), which
    /// already run on native/LLVM/WASM/JVM/CLR/VM/JIT.
    fn emit_user_fn_call(&mut self, name: &str, node: &GrammarASTNode)
        -> Result<String, CompileError>
    {
        if !self.defined_fns.contains(name) {
            return Err(CompileError::Unsupported(format!(
                "call to undefined function `{name}` (no matching DEF FN)")));
        }
        // The single argument is the lone `expr` child of the primary.
        let arg_node = child_nodes(node).into_iter()
            .find(|n| n.rule_name == "expr")
            .ok_or_else(|| CompileError::Malformed(format!(
                "user function call `{name}` missing argument expr")))?;
        let arg = self.emit_expr(arg_node)?;
        let dest = self.fresh_temp();
        self.emit("call", Some(&dest),
            vec![Operand::Var(name.to_string()), Operand::Var(arg)], "i64");
        Ok(dest)
    }

    /// Pre-pass: record every `DEF FNx` name declared on `line` so a call
    /// site lowered earlier in the program can resolve it (forward use).
    fn register_def_names(&mut self, line: &GrammarASTNode) {
        let Some(stmt) = child_nodes(line).into_iter()
            .find(|n| n.rule_name == "statement")
        else { return };
        let Some(inner) = child_nodes(stmt).into_iter().next() else { return };
        if inner.rule_name != "def_stmt" { return; }
        if let Some(name) = user_fn_token_value(inner) {
            self.defined_fns.insert(name);
        }
    }

    /// Lower a `DEF FNx(P) = expr` definition (BA5) into a sibling
    /// `IIRFunction` pushed onto `self.functions`.  Emits *nothing* into the
    /// caller's (`main`'s) instruction stream — a `DEF` is a declaration, not
    /// a runtime statement.
    ///
    /// ```text
    ///   10 DEF FNS(X) = X * X      ⇒   fn fns(X: i64) -> i64 { ret X * X }
    /// ```
    ///
    /// The body is lowered in a swapped-in emission context (mirroring
    /// ALGOL's `compile_procedure`, AL3) so its temporaries and the parameter
    /// register live in the function's own namespace, independent of `main`.
    fn emit_def(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // def_stmt = "DEF" USER_FN LPAREN NAME RPAREN EQ expr
        let name = user_fn_token_value(stmt)
            .ok_or_else(|| CompileError::Malformed(
                "DEF missing USER_FN name".into()))?;
        let param = first_name_token_value(stmt)
            .ok_or_else(|| CompileError::Malformed(
                "DEF missing parameter NAME".into()))?;
        let body = child_nodes(stmt).into_iter()
            .find(|n| n.rule_name == "expr")
            .ok_or_else(|| CompileError::Malformed(
                "DEF missing body expr".into()))?;

        // ── swap in a fresh emission context for the function body ───────
        let saved_instrs = std::mem::take(&mut self.instrs);
        let saved_source_map = std::mem::take(&mut self.source_map);
        let saved_temp = std::mem::replace(&mut self.temp_counter, 0);
        let saved_param = self.current_fn_param.replace(param.clone());

        // Lower the body and return its value.  The parameter is bound by
        // name (`param`) and the body may reference only it (enforced in
        // `emit_primary`).
        let result = self.emit_expr(body)?;
        self.emit("ret", None, vec![Operand::Var(result)], "i64");

        // ── assemble the function and restore main's context ────────────
        let body_instrs = std::mem::take(&mut self.instrs);
        let body_len = body_instrs.len();
        let mut func = IIRFunction::new(
            name,
            vec![(param, "i64".to_string())],
            "i64",
            body_instrs,
        );
        func.type_status = FunctionTypeStatus::FullyTyped;
        let mut sm = std::mem::take(&mut self.source_map);
        while sm.len() < body_len {
            sm.push(SourceLoc::SYNTHETIC);
        }
        sm.truncate(body_len);
        func.source_map = sm;
        self.functions.push(func);

        self.instrs = saved_instrs;
        self.source_map = saved_source_map;
        self.temp_counter = saved_temp;
        self.current_fn_param = saved_param;
        Ok(())
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

fn extract_line_num(line: &GrammarASTNode) -> Option<i64> {
    for c in &line.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "LINE_NUM" {
                return t.value.trim().parse::<i64>().ok();
            }
        }
    }
    None
}

/// If `var` is a *subscripted* variable `A(I)` (`variable = NAME LPAREN expr
/// RPAREN`), return its index `expr` node; for the scalar form (`NAME`) return
/// `None`.  This is the single place that distinguishes an array access from a
/// plain variable — both the `LET` write path and the expression read path use
/// it (BA3 / enabler **E5**).
fn array_subscript_index(var: &GrammarASTNode) -> Option<&GrammarASTNode> {
    child_nodes(var).into_iter().find(|n| n.rule_name == "expr")
}

/// The array name of a subscripted `variable` node `A(I)` — the leading NAME
/// token.  (`scalar_variable_name` deliberately rejects the subscripted form,
/// so the array paths use this instead.)
fn array_target_name(var: &GrammarASTNode) -> Result<String, CompileError> {
    first_name_token_value(var).ok_or_else(|| {
        CompileError::Malformed("subscripted variable missing NAME token".into())
    })
}

/// The largest array subscript `DIM` will accept.  BASIC arrays are tiny in
/// practice; this cap keeps the bound (a) exactly representable in `f64` (well
/// under 2^53, so the parse→cast round-trip is lossless) and (b) far from
/// `i64::MAX`, so the `+ 1` element-count computation can never overflow.  A
/// bound above this is a clean `Unsupported` error, never a panic.
const MAX_DIM_BOUND: i64 = 16_777_216; // 2^24 elements — generous for BASIC

/// The IIR register holding the `DATA` pool array handle (BA6).  Underscore-
/// and lowercase-bearing, so it can never collide with a BASIC variable (which
/// is an uppercase letter + optional digit, e.g. `A`, `X7`).
const BASIC_DATA_ARRAY: &str = "__basic_data";
/// The IIR register holding the `READ`/`RESTORE` pointer into the pool (BA6).
const BASIC_DATA_PTR: &str = "__basic_data_ptr";

/// The integer bound `n` in a `dim_decl = NAME LPAREN NUMBER RPAREN`.  The
/// grammar pins the bound to a `NUMBER` literal (not an arbitrary expression),
/// so we read it straight from the token rather than emitting code to compute
/// it.
///
/// A `NUMBER` token can spell an arbitrarily large or fractional value, so we
/// validate the parsed `f64` *before* casting: the bare `as i64` cast saturates
/// (e.g. `1e30 as i64` → `i64::MAX`), which would otherwise sail past a naive
/// range check and overflow the later `+ 1`.  We therefore reject non-finite,
/// negative, and out-of-`MAX_DIM_BOUND`-range values up front; only an in-range
/// value reaches the (now lossless) `as i64` cast.  A fractional spelling is
/// truncated toward zero, consistent with how this integer-only V1 treats every
/// other `NUMBER`.
fn dim_decl_bound(decl: &GrammarASTNode) -> Result<i64, CompileError> {
    for c in &decl.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "NUMBER" {
                let raw = t.value.trim();
                let f = raw.parse::<f64>().map_err(|_| CompileError::Malformed(
                    format!("DIM bound `{raw}` is not a number")))?;
                if !f.is_finite() || f < 0.0 || f > MAX_DIM_BOUND as f64 {
                    return Err(CompileError::Unsupported(format!(
                        "DIM bound `{raw}` is out of the supported range \
                         0..={MAX_DIM_BOUND}")));
                }
                // In range and non-negative ⇒ this `as i64` truncates the
                // fractional part without saturation or overflow.
                return Ok(f as i64);
            }
        }
    }
    Err(CompileError::Malformed("DIM decl missing NUMBER bound".into()))
}

fn scalar_variable_name(var: &GrammarASTNode)
    -> Result<String, CompileError>
{
    // `variable = NAME LPAREN expr RPAREN | NAME`.
    // V1 only supports the scalar form.  Array access (3+ children) is
    // deferred until LANG76-based array lowering lands.
    if var.children.len() != 1 {
        return Err(CompileError::Unsupported(
            "array variable access (e.g. A(I)) — deferred to V2".into()));
    }
    for c in &var.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "NAME" {
                return Ok(t.value.clone());
            }
        }
    }
    Err(CompileError::Malformed(
        "variable node missing NAME token".into()))
}

fn first_name_token_value(node: &GrammarASTNode) -> Option<String> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "NAME" {
                return Some(t.value.clone());
            }
        }
    }
    None
}

/// The first `USER_FN` token value (`fns`, `fna`, …) directly under `node`.
/// Used for both the name in a `DEF FNx(…)` declaration and at a `FNx(…)`
/// call site.  `USER_FN` is a distinct lexer token (regex `/fn[a-z]/`), so it
/// never collides with a plain `NAME` parameter.
fn user_fn_token_value(node: &GrammarASTNode) -> Option<String> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "USER_FN" {
                return Some(t.value.clone());
            }
        }
    }
    None
}

fn first_number_token(node: &GrammarASTNode) -> Option<i64> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "NUMBER" {
                // BASIC line numbers / GOTO/GOSUB targets are always integers.
                return t.value.trim().parse::<f64>().ok().map(|f| f as i64);
            }
        }
    }
    None
}

fn extract_if_target(stmt: &GrammarASTNode) -> Result<i64, CompileError> {
    // `IF` expr relop expr `THEN` NUMBER — the NUMBER comes *after* THEN.
    // Since both LINE_NUM and NUMBER tokens can appear in an IF chain
    // (no — LINE_NUM only on the line itself), the first NUMBER token
    // after the THEN keyword is the target.
    let mut saw_then = false;
    for c in &stmt.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "KEYWORD" && t.value == "THEN" {
                saw_then = true;
            } else if saw_then && t.effective_type_name() == "NUMBER" {
                return t.value.trim().parse::<f64>().ok().map(|f| f as i64)
                    .ok_or_else(|| CompileError::Malformed(
                        format!("IF THEN target not a number: {}", t.value)));
            }
        }
    }
    Err(CompileError::Malformed("IF missing THEN <number>".into()))
}

fn extract_relop_op(relop: &GrammarASTNode) -> Result<&'static str, CompileError> {
    // `relop` is a wrapper around exactly one of EQ / LT / GT / LE / GE / NE.
    for c in &relop.children {
        if let ASTNodeOrToken::Token(t) = c {
            return Ok(match t.value.as_str() {
                "="  => "cmp_eq",
                "<"  => "cmp_lt",
                ">"  => "cmp_gt",
                "<=" => "cmp_le",
                ">=" => "cmp_ge",
                "<>" => "cmp_ne",
                other => return Err(CompileError::Malformed(
                    format!("unknown relop `{other}`"))),
            });
        }
    }
    Err(CompileError::Malformed("relop has no token child".into()))
}

fn binary_op_name(op: &str) -> Option<&'static str> {
    match op {
        "+" => Some("add"),
        "-" => Some("sub"),
        "*" => Some("mul"),
        "/" => Some("div"),
        _ => None,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn compile(src: &str) -> Result<IIRModule, CompileError> {
        compile_source(src, "test")
    }

    /// The simplest possible BASIC program — `10 END` — should produce
    /// a module with one function `main` whose body is `label, const 0,
    /// ret 0`.
    #[test]
    fn compiles_minimal_end() {
        let m = compile("10 END\n").expect("ok");
        assert_eq!(m.functions.len(), 1);
        let main = &m.functions[0];
        assert_eq!(main.name, "main");
        assert_eq!(main.return_type, "i64");
        let ops: Vec<&str> = main.instructions.iter()
            .map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"label"));
        assert!(ops.contains(&"const"));
        assert!(ops.contains(&"ret"));
    }

    /// `LET A = 42` followed by `END` should leave variable A holding 42.
    #[test]
    fn compiles_let_then_end() {
        let m = compile("10 LET A = 42\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        // mov_i64 A = _t0 (where _t0 was const 42)
        let mov = body.iter().find(|i| i.op == "mov")
            .expect("LET produces a mov");
        assert_eq!(mov.dest.as_deref(), Some("A"));
        // and a const 42 somewhere.
        assert!(body.iter().any(|i|
            i.op == "const" && matches!(i.srcs.first(), Some(Operand::Int(42)))));
    }

    /// `PRINT 42` lowers to `const 42 → t; call_builtin "print_i64", t`.
    #[test]
    fn compiles_print_integer() {
        let m = compile("10 PRINT 42\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let call = body.iter().find(|i|
            i.op == "call_builtin" && i.srcs.first().and_then(|s| match s {
                Operand::Var(n) => Some(n.as_str()), _ => None,
            }) == Some("print_i64")
        );
        assert!(call.is_some(), "expected call_builtin print_i64 in {body:?}");
    }

    /// `GOTO 99` emits a `jmp` to label `line_99`.
    #[test]
    fn compiles_goto() {
        let m = compile("10 GOTO 99\n99 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let jmp = body.iter().find(|i| i.op == "jmp");
        assert!(jmp.is_some());
        let target = jmp.unwrap().srcs.first().and_then(|s| match s {
            Operand::Var(n) => Some(n.clone()), _ => None,
        });
        assert_eq!(target, Some("line_99".to_string()));
    }

    /// `IF A > 5 THEN 100` emits `cmp_gt`, then `jmp_if_true cond, line_100`.
    #[test]
    fn compiles_if_then() {
        let src = "10 LET A = 7\n20 IF A > 5 THEN 100\n30 END\n100 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "cmp_gt"));
        let jit = body.iter().find(|i| i.op == "jmp_if_true").expect("jmp_if_true");
        assert_eq!(jit.srcs.get(1).and_then(|s| match s {
            Operand::Var(n) => Some(n.clone()), _ => None,
        }), Some("line_100".to_string()));
    }

    /// `FOR I = 1 TO 3 / NEXT I` emits the loop scaffold: const, mov,
    /// label test, cmp_le, jmp_if_false, … add, mov, jmp, label end.
    #[test]
    fn compiles_for_next() {
        let src = "10 FOR I = 1 TO 3\n20 NEXT I\n30 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        let ops: Vec<&str> = body.iter().map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"cmp_le"), "got: {ops:?}");
        assert!(ops.contains(&"jmp_if_false"));
        // Need both the for_0_test label and the for_0_end label.
        let labels: Vec<&str> = body.iter()
            .filter(|i| i.op == "label")
            .filter_map(|i| match i.srcs.first() {
                Some(Operand::Var(n)) => Some(n.as_str()), _ => None,
            }).collect();
        assert!(labels.iter().any(|l| l.starts_with("for_") && l.ends_with("_test")),
                "missing for_*_test label in {labels:?}");
        assert!(labels.iter().any(|l| l.starts_with("for_") && l.ends_with("_end")),
                "missing for_*_end label in {labels:?}");
    }

    /// `REM` is a no-op — generates only the line label.
    #[test]
    fn compiles_rem_as_noop() {
        let m = compile("10 REM HELLO WORLD\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        // Line 10 emits label line_10 then nothing else for REM.
        let between: Vec<&str> = body.iter()
            .skip_while(|i| i.op != "label")
            .take_while(|i| i.op != "ret")
            .map(|i| i.op.as_str()).collect();
        // Should be: label, label, const, (ret comes from END).
        // We expect 3 instructions before ret: line_10 label, line_20 label, const 0.
        assert!(between.contains(&"label"));
    }

    /// `INPUT X` emits `call_builtin "input_i64" -> X`.
    #[test]
    fn compiles_input() {
        let m = compile("10 INPUT X\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let call = body.iter().find(|i|
            i.op == "call_builtin" && i.dest.as_deref() == Some("X"));
        assert!(call.is_some(), "expected call_builtin -> X");
        let helper = call.unwrap().srcs.first().and_then(|s| match s {
            Operand::Var(n) => Some(n.as_str()), _ => None,
        });
        assert_eq!(helper, Some("input_i64"));
    }

    /// String literals in PRINT are deferred → `Unsupported`.
    #[test]
    fn rejects_print_string() {
        let err = compile("10 PRINT \"HI\"\n20 END\n").unwrap_err();
        match err {
            CompileError::Unsupported(_) => {}
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    /// GOSUB is deferred to V2 — emits `UnsupportedStatement`.
    #[test]
    fn rejects_gosub_as_unsupported_statement() {
        let err = compile("10 GOSUB 100\n100 END\n").unwrap_err();
        match err {
            CompileError::UnsupportedStatement(name) => assert_eq!(name, "GOSUB"),
            other => panic!("expected UnsupportedStatement(GOSUB), got {other:?}"),
        }
    }

    // ── DEF FN — user-defined single-line functions (BA5) ────────────

    /// `DEF FNS(X) = X * X` lowers to a sibling `IIRFunction` named `fns`
    /// (one `i64` parameter `X`, body `mul X X` then `ret`), pushed after
    /// `main`.  The `DEF` line itself emits nothing runtime into `main`.
    #[test]
    fn compiles_def_fn_into_sibling_function() {
        let m = compile("10 DEF FNS(X) = X * X\n20 PRINT FNS(7)\n30 END\n")
            .expect("ok");
        // main + one sibling function.
        assert_eq!(m.functions.len(), 2, "expected main + fns");
        let f = m.functions.iter().find(|f| f.name == "FNS")
            .expect("sibling function `FNS`");
        assert_eq!(f.return_type, "i64");
        assert_eq!(f.params, vec![("X".to_string(), "i64".to_string())]);
        let ops: Vec<&str> = f.instructions.iter().map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"mul"), "fns body should multiply: {ops:?}");
        assert_eq!(ops.last(), Some(&"ret"), "fns body ends with ret: {ops:?}");
        // The call site in `main` emits a `call` naming `fns`.
        let main = &m.functions[0];
        let call = main.instructions.iter().find(|i|
            i.op == "call"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "FNS"))
            .expect("main calls fns");
        // `call dest = fns, <arg>` — callee + one argument slot.
        assert_eq!(call.srcs.len(), 2, "call passes one argument");
    }

    /// A function may be *called before* its `DEF` line appears (BASIC
    /// permits forward use) — the pre-pass registers the name first.
    #[test]
    fn compiles_forward_referenced_def_fn() {
        let m = compile("10 PRINT FNS(7)\n20 DEF FNS(X) = X * X\n30 END\n")
            .expect("forward reference should compile");
        assert!(m.functions.iter().any(|f| f.name == "FNS"));
        assert!(m.functions[0].instructions.iter().any(|i|
            i.op == "call"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "FNS")));
    }

    /// Calling a function that was never `DEF`-ined is a clean
    /// `Unsupported` error, not a malformed-AST panic.
    #[test]
    fn rejects_call_to_undefined_function() {
        let err = compile("10 PRINT FNQ(1)\n20 END\n").unwrap_err();
        match err {
            CompileError::Unsupported(msg) => assert!(
                msg.contains("undefined function"),
                "message should name the cause: {msg}"),
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    /// A `DEF` body may reference only its own parameter — touching any
    /// other variable (a `main`-scope global) is rejected, because a
    /// function can't read `main`'s registers on the code-gen backends
    /// without the host global table they reject (enabler E6).
    #[test]
    fn rejects_global_reference_in_def_body() {
        let err = compile(
            "10 LET A = 5\n20 DEF FNS(X) = X + A\n30 PRINT FNS(7)\n40 END\n")
            .unwrap_err();
        match err {
            CompileError::Unsupported(msg) => assert!(
                msg.contains("only its") || msg.contains("parameter"),
                "message should explain the scope rule: {msg}"),
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    /// The source-map lockstep invariant holds for the *sibling* function
    /// too (one entry per instruction), not just `main`.
    #[test]
    fn def_fn_source_map_lockstep() {
        let m = compile("10 DEF FNS(X) = X * X\n20 PRINT FNS(7)\n30 END\n")
            .expect("ok");
        for f in &m.functions {
            assert_eq!(f.source_map.len(), f.instructions.len(),
                "function {} source_map/instruction mismatch", f.name);
        }
    }

    // ── Source-map invariants (BASIC05 — debugger prerequisite) ──────

    /// Every function's `source_map` must have exactly one entry per
    /// instruction.  Without this lockstep invariant the debugger's
    /// sidecar cannot map a paused IIR PC back to a source line.
    #[test]
    fn source_map_lockstep_with_instructions() {
        let m = compile("10 LET A = 42\n20 PRINT A\n30 END\n").expect("ok");
        for f in &m.functions {
            assert_eq!(
                f.source_map.len(),
                f.instructions.len(),
                "fn {} source_map ({}) must be lockstep with instructions ({})",
                f.name, f.source_map.len(), f.instructions.len(),
            );
        }
    }

    /// The compiler should thread real source positions through the
    /// emitted IIR, not just `SYNTHETIC` (line=0, col=0).  Without
    /// real positions, line-based breakpoints cannot resolve.
    ///
    /// We construct a small program with each statement on its own
    /// line and assert that at least one instruction is tagged with
    /// each non-empty source line.
    #[test]
    fn source_map_carries_real_line_numbers() {
        let src = "10 LET A = 30\n\
                   20 LET B = 12\n\
                   30 PRINT A\n\
                   40 END\n";
        let m = compile(src).expect("ok");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        let lines_seen: std::collections::BTreeSet<u32> = main.source_map.iter()
            .filter(|l| **l != SourceLoc::SYNTHETIC)
            .map(|l| l.line)
            .collect();
        // Each BASIC source line should appear at least once in the
        // source_map (since each emits at least the `label line_N`
        // instruction plus a statement body).
        assert!(lines_seen.contains(&1),
            "expected line 1 (LET A) to appear in source_map; got: {lines_seen:?}");
        assert!(lines_seen.contains(&2),
            "expected line 2 (LET B) to appear in source_map; got: {lines_seen:?}");
        assert!(lines_seen.contains(&3),
            "expected line 3 (PRINT A) to appear in source_map; got: {lines_seen:?}");
    }

    /// A complete small program: LET, arithmetic, PRINT, END.
    /// Just exercises that the whole pipeline compiles a multi-line source
    /// without crashing.
    #[test]
    fn compiles_arithmetic_program() {
        let src = "10 LET A = 30\n\
                   20 LET B = 12\n\
                   30 LET C = A + B\n\
                   40 PRINT C\n\
                   50 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        // Should have at least one `add` and one `call_builtin print_i64`.
        assert!(body.iter().any(|i| i.op == "add"));
        assert!(body.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first().and_then(|s| match s {
                Operand::Var(n) => Some(n.as_str()), _ => None,
            }) == Some("print_i64")));
    }

    // -----------------------------------------------------------------------
    // BA6 — READ / DATA / RESTORE (data pool over E5 arrays)
    // -----------------------------------------------------------------------

    /// `DATA` + `READ` lowers to: an `alloc_array` for the pool, `array_set`s
    /// filling it, an `array_get` per `READ`, and the pointer advance (`add`).
    #[test]
    fn read_data_lowers_to_pool_array_and_get() {
        let m = compile("10 DATA 7, 8\n20 READ X\n30 PRINT X\n40 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "alloc_array"),
            "DATA must materialise a pool array; got {:?}",
            body.iter().map(|i| &i.op).collect::<Vec<_>>());
        // Two DATA values ⇒ two array_set fills.
        assert_eq!(body.iter().filter(|i| i.op == "array_set").count(), 2,
            "two DATA values should produce two array_set fills");
        // READ reads through the pointer (array_get) and advances it (add).
        assert!(body.iter().any(|i| i.op == "array_get"), "READ must array_get");
        assert!(body.iter().any(|i| i.op == "add"
            && i.dest.as_deref() == Some("__basic_data_ptr")),
            "READ must advance __basic_data_ptr with an add");
    }

    /// `RESTORE` resets the pointer to 0 with a `mov` of a `const 0`.
    #[test]
    fn restore_resets_pointer_to_zero() {
        let m = compile("10 DATA 1\n20 READ X\n30 RESTORE\n40 READ Y\n50 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        // The RESTORE `mov __basic_data_ptr = <const 0 reg>` exists; find a mov
        // into the pointer whose source const is 0 (the init also does this, so
        // there are at least two — one init + one RESTORE).
        let ptr_movs = body.iter().filter(|i| i.op == "mov"
            && i.dest.as_deref() == Some("__basic_data_ptr")).count();
        assert!(ptr_movs >= 2,
            "expected the init mov plus a RESTORE mov into the pointer; got {ptr_movs}");
    }

    /// `READ`/`RESTORE` with no `DATA` in the program is a clean error, not a
    /// miscompile that reads an uninitialised pointer.
    #[test]
    fn read_without_data_errors() {
        let err = compile("10 READ X\n20 END\n").unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)),
            "READ with no DATA should be Unsupported, got {err:?}");
    }

    /// A non-integer `DATA` value is rejected (real DATA is a follow-up).
    #[test]
    fn non_integer_data_errors() {
        let err = compile("10 DATA 3.5\n20 READ X\n30 END\n").unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)),
            "non-integer DATA should be Unsupported, got {err:?}");
    }

    // -----------------------------------------------------------------------
    // BA3 — DIM arrays (enabler E5)
    // -----------------------------------------------------------------------

    fn var_name(op: Option<&Operand>) -> Option<&str> {
        match op {
            Some(Operand::Var(n)) => Some(n.as_str()),
            _ => None,
        }
    }

    /// `DIM A(5)` lowers to `alloc_array A = <len>` where the length is the
    /// **inclusive** element count `5 + 1 = 6` (BASIC arrays are 0-based:
    /// `A(0)..A(5)`).
    #[test]
    fn compiles_dim_to_alloc_array() {
        let m = compile("10 DIM A(5)\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let alloc = body.iter().find(|i| i.op == "alloc_array")
            .expect("DIM produces an alloc_array");
        assert_eq!(alloc.dest.as_deref(), Some("A"));
        assert_eq!(alloc.type_hint, "array<i64>");
        // Its length operand is a register that was `const 6`.
        let len_reg = var_name(alloc.srcs.first()).expect("alloc_array len reg");
        assert!(body.iter().any(|i|
            i.op == "const" && i.dest.as_deref() == Some(len_reg)
                && matches!(i.srcs.first(), Some(Operand::Int(6)))),
            "expected `const 6` feeding the alloc_array length");
    }

    /// `DIM A(3), B(2)` declares two arrays in one statement → two
    /// `alloc_array`s with lengths 4 and 3.
    #[test]
    fn compiles_multi_dim_to_two_alloc_arrays() {
        let m = compile("10 DIM A(3), B(2)\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let allocs: Vec<_> = body.iter().filter(|i| i.op == "alloc_array").collect();
        assert_eq!(allocs.len(), 2);
        assert_eq!(allocs[0].dest.as_deref(), Some("A"));
        assert_eq!(allocs[1].dest.as_deref(), Some("B"));
    }

    /// `LET A(2) = 7` lowers to `array_set A, idx, val` with the subscript
    /// used **directly** as the 0-based index (no lower-bound subtraction).
    #[test]
    fn compiles_array_assignment_to_array_set() {
        let m = compile("10 DIM A(5)\n20 LET A(2) = 7\n30 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let set = body.iter().find(|i| i.op == "array_set")
            .expect("LET A(i)=e produces an array_set");
        assert_eq!(set.type_hint, "i64");
        assert_eq!(var_name(set.srcs.first()), Some("A"));
        assert_eq!(set.srcs.len(), 3, "array_set takes handle, index, value");
        // The index register was `const 2`, used as-is (BASIC is 0-based).
        let idx_reg = var_name(set.srcs.get(1)).expect("index reg");
        assert!(body.iter().any(|i|
            i.op == "const" && i.dest.as_deref() == Some(idx_reg)
                && matches!(i.srcs.first(), Some(Operand::Int(2)))));
    }

    /// `LET X = A(2)` reads an element → `array_get A, idx → dest`.
    #[test]
    fn compiles_array_read_to_array_get() {
        let m = compile("10 DIM A(5)\n20 LET X = A(2)\n30 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let get = body.iter().find(|i| i.op == "array_get")
            .expect("reading A(i) produces an array_get");
        assert_eq!(get.type_hint, "i64");
        assert_eq!(var_name(get.srcs.first()), Some("A"));
        assert!(get.dest.is_some(), "array_get writes a destination register");
    }

    /// Storing into an array that was never `DIM`med is a clean error, not a
    /// miscompile against an undefined handle register.
    #[test]
    fn undeclared_array_write_is_unsupported() {
        let err = compile("10 LET A(2) = 7\n20 END\n").unwrap_err();
        match err {
            CompileError::Unsupported(msg) => assert!(msg.contains("DIM")),
            other => panic!("expected Unsupported(DIM…), got {other:?}"),
        }
    }

    /// A `DIM` bound far larger than `MAX_DIM_BOUND` (here spelled in
    /// scientific notation, which the bare `as i64` cast would *saturate* to
    /// `i64::MAX` and then overflow on the `+ 1`) must be a clean `Unsupported`
    /// error — never a panic or a wrapped/garbage length.
    #[test]
    fn oversized_dim_bound_is_unsupported_not_a_panic() {
        let err = compile("10 DIM A(1E30)\n20 END\n").unwrap_err();
        match err {
            CompileError::Unsupported(msg) => assert!(msg.contains("range")),
            other => panic!("expected Unsupported(range…), got {other:?}"),
        }
    }

    /// Reading an array that was never `DIM`med is likewise an error.
    #[test]
    fn undeclared_array_read_is_unsupported() {
        let err = compile("10 LET X = A(2)\n20 END\n").unwrap_err();
        match err {
            CompileError::Unsupported(msg) => assert!(msg.contains("DIM")),
            other => panic!("expected Unsupported(DIM…), got {other:?}"),
        }
    }

    /// End-to-end shape: fill an array in a loop and sum it back — the canonical
    /// E5 program.  Confirms the alloc/set/get ops all appear and the subscript
    /// expression (a variable `I`) flows through as the index.
    #[test]
    fn compiles_array_fill_and_sum_program() {
        let src = "10 DIM A(3)\n\
                   20 FOR I = 0 TO 3\n\
                   30 LET A(I) = I\n\
                   40 NEXT I\n\
                   50 LET S = 0\n\
                   60 FOR I = 0 TO 3\n\
                   70 LET S = S + A(I)\n\
                   80 NEXT I\n\
                   90 PRINT S\n\
                   99 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "alloc_array"));
        assert!(body.iter().any(|i| i.op == "array_set" && var_name(i.srcs.first()) == Some("A")));
        assert!(body.iter().any(|i| i.op == "array_get" && var_name(i.srcs.first()) == Some("A")));
    }
}
