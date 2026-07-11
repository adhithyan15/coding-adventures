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
//! BA7 scalar BASIC uses the historical Dartmouth numeric model: scalar
//! literals, numeric variables, arithmetic, `DEF FN`, `FOR`, `IF`, `PRINT`,
//! `DATA`, and arrays lower as `f64`.  Integer `i64` remains at structural
//! boundaries: line numbers, `DIM` bounds, array subscripts, DATA read pointers,
//! and GOSUB return stacks.  String literals and literal-backed string variables
//! in `PRINT` lower through the shared E4 string ops.
//!
//! | Statement     | Lowering |
//! |---------------|----------|
//! | `LET A = expr` | `<eval expr → t>; mov A = t` (`f64` scalar values; explicit `i64` boundaries) |
//! | `PRINT a; "x", c` | numeric items call `__basic_print_int`/`__basic_print_real`, string literals lower to `str_const` + `print_str`, with `;`=tight / `,`=space separators and a trailing `putchar(10)` newline (BA2/BA4/BA7) |
//! | `INPUT X`      | numeric: `call_builtin "input_i64" -> X`; string `INPUT A$`: `call_builtin "input_str" -> t; mov __basic_str_A = t` (runtime string, E4-dyn) |
//! | `IF cond THEN m` | `<eval cond → c>; jmp_if_true c, "line_m"` |
//! | `GOTO m`       | `jmp "line_m"` |
//! | `FOR I = a TO b STEP s` / `NEXT I` | classic counter loop with `for_<n>_test` / `for_<n>_end` labels |
//! | `END`          | `const_i64 0 -> r; ret r` |
//! | `REM …`        | no-op |
//! | `GOSUB n`      | push call-site id on the `array<i64>` return stack, `jmp line_n`, drop `gosub_ret_<id>` (BA1 / E7) |
//! | `RETURN`       | pop the id, computed-`goto` (`cmp_eq`+`jmp_if_true`) to its `gosub_ret_<id>` (BA1 / E7) |
//! | `READ` / `DATA` / `RESTORE` | real `DATA` pool over `array<f64>` (BA6 / BA7) |
//! | `DIM A(n)`     | `alloc_array A = <n+1>` (`array<f64>`, 0-based, inclusive) — BA3 / BA7 |
//! | `LET A(i)=e`   | `<eval i → x>; <eval e → v>; array_set A, x, v` (BA3) |
//! | `A(i)` (rvalue) | `<eval i → x>; array_get A, x -> t` (BA3) |
//! | `DIM A$(n)`    | `alloc_array A$ = <n+1>` (`array<str>`, E4-dyn string handles) — E4d-BA-arr |
//! | `LET A$(i)=s`  | `<eval i → x>; <eval string s → h>; array_set A$, x, h` (`str`) — E4d-BA-arr |
//! | `A$(i)` (rvalue) | `<eval i → x>; array_get A$, x -> t` (`str`) — E4d-BA-arr |
//! | `STOP`         | same as `END` for V1 |
//! | `DEF FNx(P)=e` | sibling `IIRFunction` + `call` (BA5); `FNx(arg)` → `call` |
//!
//! ## Strings
//!
//! `PRINT "HELLO"` lowers to the shared LANG-FULL E4 string ops
//! (`str_const` + `print_str`) and runs on every matrix column.  String
//! variables (`A$`) can be assigned from literals and printed through the same
//! E4 path.  `IF A$ = "Y" THEN n` lowers to `str_eq`.  `INPUT A$` reads a whole
//! line from the host as a **runtime string** (`call_builtin "input_str"` — the
//! `str` sibling of numeric `INPUT`'s `input_i64`), a value the compiler cannot
//! fold.  **String arrays** (`DIM A$(n)`; `A$(i) = …`; `PRINT A$(i)`) reuse the
//! E5 aggregate substrate with a `str` element type (`array<str>`), so each
//! element holds an E4-dyn runtime string handle — enabler **E4-dyn**, work item
//! **E4d-BA-arr**.  String `READ`/`DATA` (numeric `DATA` only today) remains a
//! follow-up.
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
    /// string variables or a not-yet-enabled backend-specific feature).
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
    // BA2/BA7: if any `PRINT` rendered a value, append the synthetic
    // numeric helpers (`__basic_print_int` / `__basic_print_uint` /
    // `__basic_print_real`) so the `call`s emitted by `emit_print` resolve.
    // Appended before the
    // user-defined functions purely for readability; order doesn't matter
    // because every `call` resolves the callee by name.
    if comp.needs_print_helpers {
        for func in print_helper_functions() {
            module.functions.push(func);
        }
    }
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
    /// Names declared as arrays via `DIM` (BA3 / BA-DIM-2D / enabler **E5**).
    /// Each name maps to its **row-major strides**, one per declared dimension.
    /// We use the BASIC variable name itself as the IIR register holding the
    /// array *handle* (an array `A` and a scalar `A` are different variables in
    /// Dartmouth BASIC, and tiny programs never collide them).  Membership tells
    /// the `LET` and expression paths whether a subscripted `A(I)`/`A(I,J)` is a
    /// real array access (→ `array_set`/`array_get`) or an undeclared use.
    ///
    /// The stride vector's length is the array's dimensionality, so it also
    /// enforces the subscript count.  For `DIM A(M,N)` the sizes are `(M+1,N+1)`
    /// (0-based inclusive) and the strides are `[N+1, 1]`: the flat index of
    /// `A(i,j)` is `i*(N+1) + j`.  A 1-D `DIM A(N)` stores `[1]`, so `A(i)` is
    /// the flat index `i` directly — unchanged from BA3.
    arrays: std::collections::HashMap<String, Vec<i64>>,
    /// The subset of [`arrays`](Self::arrays) whose element type is `str`
    /// (declared with a `$`-suffixed name, `DIM A$(n)`) — enabler **E4-dyn**,
    /// work item **E4d-BA-arr**.  A string array is *also* recorded in `arrays`
    /// (its row-major strides drive the same flat-index folding numeric arrays
    /// use), so membership here is the single bit that decides whether a
    /// subscripted `A$(i)` lowers to a `str`-typed `array_get`/`array_set`
    /// (reusing the E5 aggregate substrate to hold E4-dyn runtime string
    /// handles) rather than the numeric `f64` path.  A numeric read of a name
    /// in this set is a clean type error, mirroring the scalar `$`/non-`$`
    /// split.
    string_arrays: std::collections::HashSet<String>,
    /// Scalar value types learned while lowering `main`.  BA7 makes BASIC
    /// scalar values real (`f64`); the few integer slots left are true
    /// structural boundaries such as indexes, read pointers, and return PCs.
    scalar_types: std::collections::HashMap<String, BasicScalarType>,
    /// The `DATA` pool (BA6): every numeric literal from every `DATA`
    /// statement, gathered in line-number order by a pre-pass.  `READ`
    /// consumes these sequentially through a run-time pointer; `RESTORE`
    /// rewinds the pointer.  Because the BASIC program is a single `main`
    /// function (no `GOSUB` yet), the pool is materialised once at the top of
    /// `main` as an `array<f64>` ([[E5]] arrays, with BA7 real elements) plus
    /// an `i64` pointer register — no module global is needed.
    data_pool: Vec<f64>,
    /// Set once any `PRINT` lowers a numeric item (BA2/BA7).  When true,
    /// `compile_program` appends the synthetic digit-printing helper functions
    /// (`__basic_print_int` / `__basic_print_uint` / `__basic_print_real`) after `main`
    /// so the `call`s emitted by [`emit_print`] resolve.  We only emit the
    /// helpers when they're actually used — a program with no `PRINT` (or
    /// only bare `PRINT`s) carries no dead functions.
    needs_print_helpers: bool,
    /// Number of `GOSUB` statements in the whole program, counted by a
    /// pre-pass (BA1 / enabler E7).  When > 0, `main` gets a return-address
    /// stack (an `array<i64>` + the `__basic_gosub_sp` pointer) materialised
    /// at the top, and every `RETURN` dispatches over `0..gosub_count` return
    /// sites.  Counting up front lets a `RETURN` that appears *before* some of
    /// the `GOSUB`s it might return to (legal in BASIC's flat program) still
    /// emit the complete dispatch chain.
    gosub_count: usize,
    /// Sequential id of the next `GOSUB` lowered, 0-based.  Emission walks
    /// lines in source order, so the id handed out here matches the pre-pass
    /// counting order — `GOSUB` #k pushes `k` and its return label is
    /// `gosub_ret_k`.
    gosub_next_id: usize,
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
            arrays: std::collections::HashMap::new(),
            string_arrays: std::collections::HashSet::new(),
            scalar_types: std::collections::HashMap::new(),
            data_pool: Vec::new(),
            needs_print_helpers: false,
            gosub_count: 0,
            gosub_next_id: 0,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // limit_slot kept for future use (e.g. cmp_ge on negative STEP)
struct ForState {
    var: String,
    limit_slot: String,
    step_slot: String,
    ty: BasicScalarType,
    test_label: String,
    end_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BasicScalarType {
    Int,
    Real,
}

impl BasicScalarType {
    fn iir(self) -> &'static str {
        match self {
            Self::Int => "i64",
            Self::Real => "f64",
        }
    }
}

#[derive(Debug, Clone)]
struct ExprValue {
    slot: String,
    ty: BasicScalarType,
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

    fn emit_str_const_to(&mut self, dest: &str, text: String) {
        self.emit("str_const", Some(dest), vec![Operand::Str(text)], "str");
    }

    /// Update the "currently compiling" source position.  Subsequent
    /// [`emit`] calls tag their instructions with this position via
    /// the `source_map` field.
    fn set_loc(&self, loc: SourceLoc) {
        self.current_loc.set(loc);
    }

    fn coerce_value(&mut self, value: ExprValue, target: BasicScalarType) -> ExprValue {
        if value.ty == target {
            return value;
        }
        let dest = self.fresh_temp();
        let op = match (value.ty, target) {
            (BasicScalarType::Int, BasicScalarType::Real) => "int_to_real",
            (BasicScalarType::Real, BasicScalarType::Int) => "real_to_int_trunc",
            _ => unreachable!("checked equal types above"),
        };
        self.emit(op, Some(&dest), vec![Operand::Var(value.slot)], target.iir());
        ExprValue { slot: dest, ty: target }
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
        // Materialise the pool once at the top of `main`: an `array<f64>` of
        // the literals plus the `__basic_data_ptr` register initialised to 0.
        self.emit_data_pool_init();

        // Pre-pass — count `GOSUB` statements (BA1 / enabler E7).  The count is
        // needed before lowering so every `RETURN` (which may appear earlier in
        // the flat program than some of the `GOSUB`s it returns to) can emit a
        // dispatch chain covering all return sites; it also tells us whether to
        // materialise the return stack at all.
        self.gosub_count = self.count_gosubs(ast);
        if self.gosub_count > 0 {
            self.emit_gosub_stack_init();
        }

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
        if !self.instrs.last().is_some_and(|i| i.op.starts_with("ret")) {
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
            "gosub_stmt"   => self.emit_gosub(stmt),
            "return_stmt"  => self.emit_return(),
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
        let subs = array_subscript_indices(var_node);
        if !subs.is_empty() {
            let name = array_target_name(var_node)?;
            if !self.arrays.contains_key(&name) {
                return Err(CompileError::Unsupported(format!(
                    "assignment to `{name}(...)` but `{name}` was never DIMmed \
                     — declare it with `DIM {name}(n)` first")));
            }
            // `array_set handle, flat_idx, value` — the flat index folds the
            // (possibly multi-dimensional) subscripts through the row-major
            // strides recorded at `DIM` (BA-DIM-2D).  1-D arrays fold to the bare
            // subscript, so BA3 semantics are unchanged.
            let flat = self.emit_flat_index(&name, &subs)?;
            if self.string_arrays.contains(&name) {
                // `A$(i) = <string expr>` — E4d-BA-arr.  The RHS lowers through
                // the shared E4 string-expression path (literal / `$`-variable /
                // `+` concat / another `A$(j)` element read) to a runtime `str`
                // handle, which `array_set` stores into the `array<str>` slot.
                // A numeric RHS is a clean type error (no silent coercion).
                let Some(val) = self.emit_basic_string_expr(expr_node)? else {
                    return Err(CompileError::Unsupported(format!(
                        "string array element `{name}(...)` assignment needs a \
                         string RHS (literal, string variable, or `+` concat)")));
                };
                self.emit("array_set", None,
                    vec![Operand::Var(basic_array_handle(&name)),
                         Operand::Var(flat), Operand::Var(val)],
                    "str");
                return Ok(());
            }
            let val = self.emit_expr(expr_node)?;
            let val = self.coerce_value(val, BasicScalarType::Real);
            self.emit("array_set", None,
                vec![Operand::Var(name), Operand::Var(flat), Operand::Var(val.slot)],
                "f64");
            return Ok(());
        }

        let var_name = scalar_variable_name(var_node)?;
        if is_basic_string_name(&var_name) {
            let slot = basic_string_slot(&var_name);
            if self.emit_basic_string_expr_to(expr_node, Some(&slot))?.is_none() {
                return Err(CompileError::Unsupported(format!(
                    "string variable `{var_name}` assignment currently supports a string literal or `+` concatenation RHS")));
            }
            return Ok(());
        }
        let val = self.emit_expr(expr_node)?;
        let target_ty = self.scalar_types.get(&var_name).copied()
            .unwrap_or(BasicScalarType::Real);
        let target_ty = if val.ty == BasicScalarType::Real {
            BasicScalarType::Real
        } else {
            target_ty
        };
        let val = self.coerce_value(val, target_ty);
        // Scalar BASIC values are real in BA7; structural boundaries stay i64.
        self.emit("mov", Some(&var_name),
                  vec![Operand::Var(val.slot)], val.ty.iir());
        self.scalar_types.insert(var_name, val.ty);
        Ok(())
    }

    /// Emit the flat 0-based index for a subscripted array reference `A(i)` or
    /// `A(i,j)` (BA3 / BA-DIM-2D).  `subs` are the subscript expressions in
    /// source order; the array's row-major strides come from the `arrays` table
    /// recorded at `DIM`.  Returns the register holding the flat `i64` index,
    /// ready for `array_get`/`array_set`.
    ///
    /// `flat = Σ_d subscript[d] * stride[d]`.  BASIC subscripts are already
    /// 0-based (`DIM A(N)` gives `A(0)..A(N)`), so — unlike ALGOL's `[lo:hi]` —
    /// no lower-bound subtraction is needed.  The innermost dimension has
    /// `stride == 1`, so its term is the bare subscript with no multiply
    /// emitted; a 1-D array therefore lowers to exactly the same IIR as before
    /// BA-DIM-2D (`array_get A, i`).
    fn emit_flat_index(&mut self, name: &str, subs: &[&GrammarASTNode])
        -> Result<String, CompileError>
    {
        let strides = self.arrays.get(name)
            .expect("caller checked the array is DIMmed")
            .clone();
        if subs.len() != strides.len() {
            return Err(CompileError::Unsupported(format!(
                "`{name}` was DIMmed with {} dimension(s) but {} subscript(s) given",
                strides.len(), subs.len())));
        }
        let mut flat: Option<String> = None;
        for (sub, stride) in subs.iter().zip(&strides) {
            let idx = self.emit_expr(sub)?;
            let idx = self.coerce_value(idx, BasicScalarType::Int);
            // contrib = subscript * stride  (stride == 1 ⇒ just the subscript).
            let contrib = if *stride == 1 {
                idx.slot
            } else {
                let s = self.fresh_temp();
                self.emit("const", Some(&s), vec![Operand::Int(*stride)], "i64");
                let prod = self.fresh_temp();
                self.emit("mul", Some(&prod),
                    vec![Operand::Var(idx.slot), Operand::Var(s)], "i64");
                prod
            };
            flat = Some(match flat {
                None => contrib,
                Some(acc) => {
                    let sum = self.fresh_temp();
                    self.emit("add", Some(&sum),
                        vec![Operand::Var(acc), Operand::Var(contrib)], "i64");
                    sum
                }
            });
        }
        Ok(flat.expect("a DIMmed array always has at least one dimension"))
    }

    /// Lower `DIM A(n) [, B(m) …]` to one `alloc_array` per declared name
    /// (BA3 / BA-DIM-2D / enabler **E5**).
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
            // `dim_decl = NAME LPAREN NUMBER { COMMA NUMBER } RPAREN` —
            // one bound per dimension (BA-DIM-2D generalises BA3's single bound).
            let name = first_name_token_value(decl).ok_or_else(|| {
                CompileError::Malformed("DIM decl missing array name".into())
            })?;
            let bounds = dim_decl_bounds(decl)?;
            // Per-dimension size = max subscript + 1 (0-based inclusive).  Each
            // bound was already range-checked (`0..=MAX_DIM_BOUND`) so the `+ 1`
            // and the running product below stay panic-free via `checked_*`.
            let mut sizes: Vec<i64> = Vec::with_capacity(bounds.len());
            for max_sub in &bounds {
                if *max_sub < 0 {
                    return Err(CompileError::Unsupported(format!(
                        "DIM {name}({max_sub}) — array bound must be non-negative")));
                }
                sizes.push(max_sub.checked_add(1).ok_or_else(|| {
                    CompileError::Unsupported(format!(
                        "DIM {name}({max_sub}) — array bound too large"))
                })?);
            }
            // Total element count = product of the per-dimension sizes.  A 2×3
            // array is 6 flat elements; overflow (absurd for BASIC, but we stay
            // panic-free) is a clean `Unsupported`.
            let mut count: i64 = 1;
            for s in &sizes {
                count = count.checked_mul(*s).ok_or_else(|| {
                    CompileError::Unsupported(format!(
                        "DIM {name}(...) — total array size too large"))
                })?;
            }
            // Row-major strides: stride[last] = 1, stride[d] = product of all
            // later dimension sizes.  For `DIM A(M,N)` (sizes M+1, N+1) this is
            // `[N+1, 1]`, so `A(i,j)` → flat `i*(N+1) + j`.
            let mut strides: Vec<i64> = vec![1; sizes.len()];
            for d in (0..sizes.len().saturating_sub(1)).rev() {
                strides[d] = strides[d + 1].checked_mul(sizes[d + 1]).ok_or_else(|| {
                    CompileError::Unsupported(format!(
                        "DIM {name}(...) — stride overflow"))
                })?;
            }
            let len = self.fresh_temp();
            self.emit("const", Some(&len),
                vec![Operand::Int(count)], "i64");
            // A `$`-named array holds E4-dyn runtime string handles
            // (`array<str>`, work item E4d-BA-arr); every other array holds
            // BA7 reals (`array<f64>`).  Both ride the *same* E5 length-
            // prefixed aggregate — only the element type differs — so all of
            // the count/stride math above is shared.  On the static backends a
            // `str` element is an 8-byte handle to the `[i64 len][bytes]` heap
            // block E4-dyn already lays down; on the managed backends it is a
            // native `String[]` slot; on the VM/JIT a tagged `Value::Str`.
            let elem_ty = if is_basic_string_name(&name) {
                self.string_arrays.insert(name.clone());
                "array<str>"
            } else {
                "array<f64>"
            };
            self.emit("alloc_array", Some(&basic_array_handle(&name)),
                vec![Operand::Var(len)], elem_ty);
            self.arrays.insert(name, strides);
        }
        Ok(())
    }

    /// BA6/BA7 pre-pass: append a `DATA` statement's numeric literals to
    /// [`data_pool`].  Called once per line before any statement is lowered,
    /// so the pool ends up in line-number (source) order.
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
                if !f.is_finite() {
                    return Err(CompileError::Unsupported(format!(
                        "non-finite DATA value `{raw}`")));
                }
                self.data_pool.push(f);
            }
        }
        Ok(())
    }

    /// BA6: materialise the gathered `DATA` pool at the top of `main`.  Emits
    /// nothing when there is no `DATA`.  Otherwise allocates an `array<f64>`
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
            vec![Operand::Var(len.clone())], "array<f64>");
        // Fill the pool.  `self.data_pool` is cloned to a local first so the
        // immutable borrow doesn't clash with the `&mut self` emits.
        let values: Vec<f64> = self.data_pool.clone();
        for (i, value) in values.into_iter().enumerate() {
            let idx = self.fresh_temp();
            self.emit("const", Some(&idx), vec![Operand::Int(i as i64)], "i64");
            let val = self.fresh_temp();
            self.emit("const", Some(&val), vec![Operand::Float(value)], "f64");
            self.emit("array_set", None,
                vec![Operand::Var(BASIC_DATA_ARRAY.into()),
                     Operand::Var(idx), Operand::Var(val)], "f64");
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
                     Operand::Var(BASIC_DATA_PTR.into())], "f64");
            // Store it into the target — array element or scalar.
            let subs = array_subscript_indices(var_node);
            if !subs.is_empty() {
                let name = array_target_name(var_node)?;
                if !self.arrays.contains_key(&name) {
                    return Err(CompileError::Unsupported(format!(
                        "READ into `{name}(...)` but `{name}` was never DIMmed")));
                }
                if is_basic_string_name(&name) {
                    return Err(CompileError::Unsupported(format!(
                        "READ into string array `{name}(...)` — DATA is numeric today")));
                }
                let flat = self.emit_flat_index(&name, &subs)?;
                self.emit("array_set", None,
                    vec![Operand::Var(name), Operand::Var(flat),
                         Operand::Var(value)], "f64");
            } else {
                let target = numeric_scalar_variable_name(var_node)?;
                let value = ExprValue { slot: value, ty: BasicScalarType::Real };
                let target_ty = self.scalar_types.get(&target).copied()
                    .unwrap_or(BasicScalarType::Real);
                let value = self.coerce_value(value, target_ty);
                self.emit("mov", Some(&target),
                    vec![Operand::Var(value.slot)], value.ty.iir());
                self.scalar_types.insert(target, value.ty);
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

    /// Emit `putchar(byte)` — a `const` feeding the universal `putchar`
    /// builtin (the same one Brainfuck uses, so it lowers on all 7 backends).
    /// This is BA2's atom of output: every character BASIC prints — digits,
    /// the minus sign, separator spaces, the line-ending newline — goes out
    /// one `putchar` at a time, which is what lets several `PRINT` items share
    /// a single line.
    fn emit_putchar(&mut self, byte: i64) {
        let t = self.fresh_temp();
        self.emit("const", Some(&t), vec![Operand::Int(byte)], "i64");
        self.emit("call_builtin", None,
            vec![Operand::Var("putchar".into()), Operand::Var(t)], "void");
    }

    /// BA2 — `PRINT` with multiple items and `;` / `,` separators on one line.
    ///
    /// ```text
    ///   10 PRINT 4; 2      ⇒  42        ( ';' joins tightly )
    ///   20 PRINT 4, 2      ⇒  4 2       ( ',' inserts a space )
    ///   30 PRINT 7;        ⇒  7         ( trailing sep ⇒ no newline )
    /// ```
    ///
    /// Why a *character-level* model.  The old lowering emitted one
    /// `call_builtin "print_i64"` per item, and `print_i64` prints the number
    /// **followed by a newline** — so `PRINT 4; 2` wrongly landed `4` and `2`
    /// on separate lines.  Same-line printing requires separating "print the
    /// value" from "end the line", i.e. printing digit by digit with
    /// `putchar` and emitting the newline ourselves.  The digits come from the
    /// synthetic recursive helper `__basic_print_int` (see
    /// [`print_helper_functions`]); here we sequence items, separators, and
    /// the trailing newline.
    ///
    /// Separator semantics (BA2): `;` concatenates with nothing between;
    /// `,` inserts a single space.  Historical Dartmouth BASIC tabs `,` to
    /// the next 14-column *print zone*, which needs a run-time column counter
    /// — deferred to a later item; a single space is the well-defined BA2
    /// approximation.  A **trailing** separator (`PRINT X;` or `PRINT X,`)
    /// suppresses the line-ending newline, exactly as the manual specifies.
    fn emit_print(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        let Some(list) = child_nodes(stmt).into_iter()
            .find(|n| n.rule_name == "print_list")
        else {
            // Bare `PRINT` — emits a blank line (a lone newline).
            self.emit_putchar(b'\n' as i64);
            return Ok(());
        };

        // Walk the `print_list` children in source order.  Both `print_item`
        // and `print_sep` are rule nodes; their interleaving is the layout.
        // `pending_sep` carries the separator seen *before* the next item so
        // we can insert its spacing between items (not before the first one);
        // if it is still `Some` after the loop, the list ended on a separator
        // and the newline is suppressed.
        let mut pending_sep: Option<char> = None;
        for child in child_nodes(list) {
            match child.rule_name.as_str() {
                "print_sep" => pending_sep = Some(print_sep_char(child)),
                "print_item" => {
                    if pending_sep.take() == Some(',') {
                        self.emit_putchar(b' ' as i64);
                    }
                    let inner = child_nodes(child).into_iter().next();
                    match inner {
                        Some(expr_node) if expr_node.rule_name == "expr" => {
                            if let Some(slot) = self.emit_basic_string_expr(expr_node)? {
                                self.emit("print_str", None, vec![Operand::Var(slot)], "void");
                                continue;
                            }
                            let v = self.emit_expr(expr_node)?;
                            let dest = self.fresh_temp();
                            // Discardable result: the helper returns a dummy 0
                            // (its work is the `putchar` side effects).
                            let helper = match v.ty {
                                BasicScalarType::Int => "__basic_print_int",
                                BasicScalarType::Real => "__basic_print_real",
                            };
                            self.emit("call", Some(&dest),
                                vec![Operand::Var(helper.into()),
                                     Operand::Var(v.slot)],
                                "i64");
                            self.needs_print_helpers = true;
                        }
                        _ => {
                            if let Some(text) = string_token_value(child) {
                                let s = self.fresh_temp();
                                self.emit_str_const_to(&s, text);
                                self.emit("print_str", None,
                                    vec![Operand::Var(s)], "void");
                            } else {
                                return Err(CompileError::Malformed(
                                    "PRINT item was neither an expression nor a STRING".into()));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // A trailing separator (pending_sep still set) suppresses the newline;
        // otherwise PRINT ends its line.
        if pending_sep.is_none() {
            self.emit_putchar(b'\n' as i64);
        }
        Ok(())
    }

    fn emit_input(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `INPUT` variable { COMMA variable }
        // Each variable reads one line from the host.  A numeric variable parses
        // that line as a number (`input_i64`); a `$`-suffixed *string* variable
        // keeps the whole line as a genuinely runtime string (`input_str`).
        // V1 only handles plain NAMEs (no array elements) — `scalar_variable_name`
        // rejects a subscripted `A(I)`.
        for v in child_nodes(stmt).into_iter()
            .filter(|n| n.rule_name == "variable")
        {
            let name = scalar_variable_name(v)?;
            if is_basic_string_name(&name) {
                self.emit_input_string(&name);
            } else {
                self.emit_input_numeric(name);
            }
        }
        Ok(())
    }

    /// `INPUT X` (numeric): read a line, parse it as an integer via the
    /// `input_i64` builtin, then coerce to `X`'s tracked scalar type and store.
    fn emit_input_numeric(&mut self, name: String) {
        let input = self.fresh_temp();
        self.emit("call_builtin", Some(&input),
            vec![Operand::Var("input_i64".into())],
            "i64");
        let input = ExprValue { slot: input, ty: BasicScalarType::Int };
        let target_ty = self.scalar_types.get(&name).copied()
            .unwrap_or(BasicScalarType::Real);
        let input = self.coerce_value(input, target_ty);
        self.emit("mov", Some(&name), vec![Operand::Var(input.slot)], input.ty.iir());
        self.scalar_types.insert(name, input.ty);
    }

    /// `INPUT A$` (string, E4-dyn): the host reads a whole line and hands back a
    /// **runtime string** — a value the compiler cannot fold, unlike every prior
    /// BA4 string cell where the literal was known at compile time.  The
    /// `input_str` builtin returns a `str`; a `mov` copies it into the
    /// deterministic string slot `__basic_str_<stem>` (see [`basic_string_slot`])
    /// so a later `PRINT A$` / `IF A$ = …` resolves the same slot through the
    /// shared E4 `print_str` / `str_eq` ops.  This is the BASIC sibling of the
    /// ALGOL string-procedure result and the E4-dyn foothold's branch-selected
    /// string: the observable output depends on stdin, not on a folded constant.
    fn emit_input_string(&mut self, name: &str) {
        let input = self.fresh_temp();
        self.emit("call_builtin", Some(&input),
            vec![Operand::Var("input_str".into())],
            "str");
        let slot = basic_string_slot(name);
        self.emit("mov", Some(&slot), vec![Operand::Var(input)], "str");
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

        let lhs_str = self.emit_basic_string_expr(exprs[0])?;
        let rhs_str = self.emit_basic_string_expr(exprs[1])?;
        if lhs_str.is_some() || rhs_str.is_some() {
            let (Some(lhs), Some(rhs)) = (lhs_str, rhs_str) else {
                return Err(CompileError::Unsupported(
                    "mixed string/numeric IF comparison".into()));
            };
            let target_line = extract_if_target(stmt)?;
            match cmp_op {
                "cmp_eq" | "cmp_ne" => {
                    let branch_op = if cmp_op == "cmp_eq" {
                        "jmp_if_true"
                    } else {
                        "jmp_if_false"
                    };
                    let cond = self.fresh_temp();
                    self.emit("str_eq", Some(&cond),
                        vec![Operand::Var(lhs), Operand::Var(rhs)], "i64");
                    self.emit(branch_op, None,
                        vec![Operand::Var(cond), Operand::Var(format!("line_{target_line}"))],
                        "void");
                }
                "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
                    let ordering = self.fresh_temp();
                    self.emit("str_cmp", Some(&ordering),
                        vec![Operand::Var(lhs), Operand::Var(rhs)], "i64");
                    let zero = self.fresh_temp();
                    self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
                    let cond = self.fresh_temp();
                    self.emit(cmp_op, Some(&cond),
                        vec![Operand::Var(ordering), Operand::Var(zero)], "i64");
                    self.emit("jmp_if_true", None,
                        vec![Operand::Var(cond), Operand::Var(format!("line_{target_line}"))],
                        "void");
                }
                _ => {
                    return Err(CompileError::Unsupported(
                        "unsupported string IF comparison".into()))
                }
            }
            return Ok(());
        }

        let lhs = self.emit_expr(exprs[0])?;
        let rhs = self.emit_expr(exprs[1])?;
        let cmp_ty = if lhs.ty == BasicScalarType::Real || rhs.ty == BasicScalarType::Real {
            BasicScalarType::Real
        } else {
            BasicScalarType::Int
        };
        let lhs = self.coerce_value(lhs, cmp_ty);
        let rhs = self.coerce_value(rhs, cmp_ty);
        let cond = self.fresh_temp();
        // The `type_hint` on a comparison is the OPERAND width, not the (always
        // boolean) result — the IIR-to-* backends size the machine compare from
        // it (`i1 sgt` truncates to a 1-bit compare, the LANG-FULL BA0 bug).
        // BASIC's scalars materialise as `i64`, matching Nib/Oct/ALGOL.
        self.emit(cmp_op, Some(&cond),
            vec![Operand::Var(lhs.slot), Operand::Var(rhs.slot)],
            cmp_ty.iir());

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

    // -- GOSUB / RETURN (BA1, enabler E7) ----------------------------------
    //
    // BASIC's `GOSUB`/`RETURN` is *unstructured*: the program is one flat list
    // of line-numbered statements in `main`, and the same `RETURN` resumes at
    // the dynamically most-recent `GOSUB` (see `code/specs/lang-full-e7-
    // subroutine-return-stack.md`).  Plain `call`/`ret` can't express that, but
    // it needs NO new backend op: model it *inside* `main` as a runtime
    // return-address stack (an E5 `array<i64>`) plus a computed `goto` — the
    // exact AL5 switch chain (`cmp_eq` + `jmp_if_true`), which already runs on
    // every backend, just like E5 arrays do.

    /// Count every `gosub_stmt` in the program (pre-pass).
    fn count_gosubs(&self, ast: &GrammarASTNode) -> usize {
        let mut n = 0;
        for child in &ast.children {
            let ASTNodeOrToken::Node(line) = child else { continue };
            if line.rule_name != "line" { continue; }
            let Some(stmt) = child_nodes(line).into_iter()
                .find(|s| s.rule_name == "statement") else { continue };
            if let Some(inner) = child_nodes(stmt).into_iter().next() {
                if inner.rule_name == "gosub_stmt" { n += 1; }
            }
        }
        n
    }

    /// Materialise the return-address stack at the top of `main` (only when the
    /// program uses `GOSUB`): a fixed-capacity `array<i64>` plus the
    /// `__basic_gosub_sp` pointer seeded to 0.  Mirrors the BA6 `DATA`-pool
    /// init — one `const` + `alloc_array` + a pointer `mov`.
    fn emit_gosub_stack_init(&mut self) {
        let cap = self.fresh_temp();
        self.emit("const", Some(&cap),
            vec![Operand::Int(BASIC_GOSUB_STACK_DEPTH)], "i64");
        self.emit("alloc_array", Some(BASIC_GOSUB_STACK),
            vec![Operand::Var(cap)], "array<i64>");
        let zero = self.fresh_temp();
        self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
        self.emit("mov", Some(BASIC_GOSUB_SP), vec![Operand::Var(zero)], "i64");
    }

    /// `GOSUB n` — push this call site's id, jump to `line_n`, and drop a
    /// `gosub_ret_<id>` label so the matching `RETURN` can resume here.
    ///
    /// ```text
    ///   array_set __basic_gosub_stack, __basic_gosub_sp, <id>
    ///   __basic_gosub_sp := __basic_gosub_sp + 1
    ///   jmp line_n
    ///   label gosub_ret_<id>
    /// ```
    fn emit_gosub(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        let target = first_number_token(stmt)
            .ok_or_else(|| CompileError::Malformed("GOSUB missing target".into()))?;
        let id = self.gosub_next_id;
        self.gosub_next_id += 1;

        // push the id: stack[sp] = id
        let id_reg = self.fresh_temp();
        self.emit("const", Some(&id_reg), vec![Operand::Int(id as i64)], "i64");
        self.emit("array_set", None,
            vec![Operand::Var(BASIC_GOSUB_STACK.into()),
                 Operand::Var(BASIC_GOSUB_SP.into()), Operand::Var(id_reg)],
            "i64");
        // sp := sp + 1
        let one = self.fresh_temp();
        self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
        let new_sp = self.fresh_temp();
        self.emit("add", Some(&new_sp),
            vec![Operand::Var(BASIC_GOSUB_SP.into()), Operand::Var(one)], "i64");
        self.emit("mov", Some(BASIC_GOSUB_SP), vec![Operand::Var(new_sp)], "i64");
        // jump into the subroutine, then land the resume label
        self.emit("jmp", None,
            vec![Operand::Var(format!("line_{target}"))], "void");
        self.emit("label", None,
            vec![Operand::Var(format!("gosub_ret_{id}"))], "void");
        Ok(())
    }

    /// `RETURN` — pop the most-recent return id and computed-`goto` to its
    /// `gosub_ret_<id>` label.  The dispatch is the AL5 switch chain over every
    /// `GOSUB` site `0..gosub_count` (counted by the pre-pass).
    ///
    /// ```text
    ///   __basic_gosub_sp := __basic_gosub_sp - 1
    ///   r := array_get __basic_gosub_stack, __basic_gosub_sp
    ///   if r == 0 : jmp gosub_ret_0
    ///   …
    ///   if r == K : jmp gosub_ret_K
    /// ```
    fn emit_return(&mut self) -> Result<(), CompileError> {
        if self.gosub_count == 0 {
            return Err(CompileError::Unsupported(
                "RETURN with no GOSUB anywhere in the program".into()));
        }
        // sp := sp - 1
        let one = self.fresh_temp();
        self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
        let new_sp = self.fresh_temp();
        self.emit("sub", Some(&new_sp),
            vec![Operand::Var(BASIC_GOSUB_SP.into()), Operand::Var(one)], "i64");
        self.emit("mov", Some(BASIC_GOSUB_SP), vec![Operand::Var(new_sp)], "i64");
        // r := stack[sp]   (the popped return id)
        let r = self.fresh_temp();
        self.emit("array_get", Some(&r),
            vec![Operand::Var(BASIC_GOSUB_STACK.into()),
                 Operand::Var(BASIC_GOSUB_SP.into())], "i64");
        // computed goto: for each site id, `if r == id jmp gosub_ret_id`.
        for id in 0..self.gosub_count {
            let k = self.fresh_temp();
            self.emit("const", Some(&k), vec![Operand::Int(id as i64)], "i64");
            let matched = self.fresh_temp();
            self.emit("cmp_eq", Some(&matched),
                vec![Operand::Var(r.clone()), Operand::Var(k)], "i64");
            self.emit("jmp_if_true", None,
                vec![Operand::Var(matched),
                     Operand::Var(format!("gosub_ret_{id}"))], "void");
        }
        Ok(())
    }

    fn emit_for(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `FOR` NAME `=` expr `TO` expr [ `STEP` expr ]
        let var = first_name_token_value(stmt)
            .ok_or_else(|| CompileError::Malformed("FOR missing NAME".into()))?;
        if is_basic_string_name(&var) {
            return Err(CompileError::Unsupported(format!(
                "FOR variable `{var}` must be numeric")));
        }
        let exprs: Vec<&GrammarASTNode> = child_nodes(stmt).into_iter()
            .filter(|n| n.rule_name == "expr").collect();
        if exprs.len() < 2 {
            return Err(CompileError::Malformed(
                "FOR needs at least 2 exprs (start, end)".into()));
        }
        let start_v = self.emit_expr(exprs[0])?;
        let limit_v = self.emit_expr(exprs[1])?;
        let mut loop_ty = if start_v.ty == BasicScalarType::Real
            || limit_v.ty == BasicScalarType::Real
            || self.scalar_types.get(&var).copied() == Some(BasicScalarType::Real)
        {
            BasicScalarType::Real
        } else {
            BasicScalarType::Int
        };
        let step_v = if let Some(step_expr) = exprs.get(2) {
            let step = self.emit_expr(step_expr)?;
            if step.ty == BasicScalarType::Real {
                loop_ty = BasicScalarType::Real;
            }
            step
        } else {
            // STEP defaults to 1.
            let t = self.fresh_temp();
            match loop_ty {
                BasicScalarType::Int => {
                    self.emit("const", Some(&t), vec![Operand::Int(1)], "i64");
                }
                BasicScalarType::Real => {
                    self.emit("const", Some(&t), vec![Operand::Float(1.0)], "f64");
                }
            }
            ExprValue { slot: t, ty: loop_ty }
        };
        let start_v = self.coerce_value(start_v, loop_ty);
        let limit_v = self.coerce_value(limit_v, loop_ty);
        let step_v = self.coerce_value(step_v, loop_ty);

        // Stash limit and step in named slots so NEXT can read them later
        // (they're computed once at FOR entry, not re-evaluated each pass).
        let id = self.for_counter;
        self.for_counter += 1;
        let limit_slot = format!("_for_{id}_limit");
        let step_slot  = format!("_for_{id}_step");
        let test_label = format!("for_{id}_test");
        let end_label  = format!("for_{id}_end");

        self.emit("mov", Some(&var), vec![Operand::Var(start_v.slot)], loop_ty.iir());
        self.emit("mov", Some(&limit_slot), vec![Operand::Var(limit_v.slot)], loop_ty.iir());
        self.emit("mov", Some(&step_slot),  vec![Operand::Var(step_v.slot)],  loop_ty.iir());
        self.scalar_types.insert(var.clone(), loop_ty);

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
            loop_ty.iir());
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
            ty: loop_ty,
            test_label,
            end_label,
        });
        Ok(())
    }

    fn emit_next(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        let var = first_name_token_value(stmt)
            .ok_or_else(|| CompileError::Malformed("NEXT missing NAME".into()))?;
        if is_basic_string_name(&var) {
            return Err(CompileError::Unsupported(format!(
                "NEXT variable `{var}` must be numeric")));
        }
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
            top.ty.iir());
        self.emit("mov", Some(&top.var),
            vec![Operand::Var(new_val)], top.ty.iir());
        self.scalar_types.insert(top.var.clone(), top.ty);
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
        -> Result<ExprValue, CompileError>
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

    fn emit_basic_string_expr(&mut self, node: &GrammarASTNode)
        -> Result<Option<String>, CompileError>
    {
        self.emit_basic_string_expr_to(node, None)
    }

    fn emit_basic_string_expr_to(
        &mut self,
        node: &GrammarASTNode,
        target: Option<&str>,
    ) -> Result<Option<String>, CompileError> {
        if let Some(text) = expr_string_literal(node) {
            let dest = target
                .map(str::to_string)
                .unwrap_or_else(|| self.fresh_temp());
            self.emit_str_const_to(&dest, text);
            return Ok(Some(dest));
        }
        // `A$(i)` / `A$(i,j)` — an E4-dyn string-array element read (E4d-BA-arr).
        // The scalar `expr_string_variable_name` path deliberately skips
        // subscripted variables; a subscripted `$`-name that was DIMmed as a
        // string array lowers to a `str`-typed `array_get` at the flat row-major
        // index, producing a runtime string handle the surrounding `+` / PRINT /
        // `=` path then consumes exactly like any other runtime string.  The
        // `array_get` writes straight into `target` when one is supplied (it is
        // a fresh-value-producing load, not an aliasing `mov`), mirroring how the
        // `+`-concat path writes its final `str_concat` into `target`.
        if let Some(var) = expr_plain_variable(node) {
            let subs = array_subscript_indices(var);
            if !subs.is_empty() {
                let name = array_target_name(var)?;
                if self.string_arrays.contains(&name) {
                    let flat = self.emit_flat_index(&name, &subs)?;
                    let dest = target
                        .map(str::to_string)
                        .unwrap_or_else(|| self.fresh_temp());
                    self.emit("array_get", Some(&dest),
                        vec![Operand::Var(basic_array_handle(&name)),
                             Operand::Var(flat)], "str");
                    return Ok(Some(dest));
                }
            }
        }
        if let Some(name) = expr_string_variable_name(node)? {
            if let Some(target) = target {
                let src = basic_string_slot(&name);
                if src == target {
                    return Ok(Some(src));
                }
                let empty = self.fresh_temp();
                self.emit_str_const_to(&empty, String::new());
                self.emit("str_concat", Some(target),
                    vec![Operand::Var(src), Operand::Var(empty)], "str");
                return Ok(Some(target.to_string()));
            }
            return Ok(Some(basic_string_slot(&name)));
        }
        if !matches!(node.rule_name.as_str(), "expr" | "term") || node.children.len() < 3 {
            return Ok(None);
        }

        let mut operands: Vec<Option<String>> = Vec::new();
        let mut ops: Vec<String> = Vec::new();
        let mut iter = node.children.iter();
        let Some(first) = iter.next() else { return Ok(None); };
        let ASTNodeOrToken::Node(first_node) = first else { return Ok(None); };
        operands.push(self.emit_basic_string_expr(first_node)?);

        loop {
            let Some(op) = iter.next() else { break; };
            let ASTNodeOrToken::Token(op) = op else {
                return Err(CompileError::Malformed(format!(
                    "{}: expected operator token", node.rule_name)));
            };
            let Some(rhs) = iter.next() else {
                return Err(CompileError::Malformed(format!(
                    "{}: dangling operator", node.rule_name)));
            };
            let ASTNodeOrToken::Node(rhs_node) = rhs else {
                return Err(CompileError::Malformed(format!(
                    "{}: expected rhs expression", node.rule_name)));
            };
            ops.push(op.value.clone());
            operands.push(self.emit_basic_string_expr(rhs_node)?);
        }

        if operands.iter().all(Option::is_none) {
            return Ok(None);
        }
        if operands.iter().any(Option::is_none) {
            return Err(CompileError::Unsupported(
                "mixed string/numeric expressions in BASIC string concatenation".into()));
        }
        if ops.iter().any(|op| op != "+") {
            return Err(CompileError::Unsupported(
                "BASIC string expressions currently support `+` concatenation only".into()));
        }

        let mut acc = operands.remove(0).expect("checked string operand");
        let last = operands.len().saturating_sub(1);
        for (idx, rhs) in operands.into_iter().enumerate() {
            let dest = if idx == last {
                target
                    .map(str::to_string)
                    .unwrap_or_else(|| self.fresh_temp())
            } else {
                self.fresh_temp()
            };
            self.emit("str_concat", Some(&dest),
                vec![Operand::Var(acc), Operand::Var(rhs.expect("checked string operand"))],
                "str");
            acc = dest;
        }
        Ok(Some(acc))
    }

    fn emit_left_assoc_chain(&mut self, node: &GrammarASTNode)
        -> Result<ExprValue, CompileError>
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
            let result_ty = if acc.ty == BasicScalarType::Real || rhs.ty == BasicScalarType::Real {
                BasicScalarType::Real
            } else {
                BasicScalarType::Int
            };
            let acc_v = self.coerce_value(acc, result_ty);
            let rhs_v = self.coerce_value(rhs, result_ty);
            self.emit(cir_op, Some(&dest),
                vec![Operand::Var(acc_v.slot), Operand::Var(rhs_v.slot)],
                result_ty.iir());
            acc = ExprValue { slot: dest, ty: result_ty };
        }
        Ok(acc)
    }

    fn emit_power(&mut self, node: &GrammarASTNode)
        -> Result<ExprValue, CompileError>
    {
        // `power = unary [ CARET power ]` — right-associative.  BA-^ supports
        // the backend-neutral subset where the exponent is a small
        // nonnegative integer-valued literal; that lowers to repeated f64
        // multiplication, avoiding a cross-backend math runtime.
        let kids = node.children.iter().collect::<Vec<_>>();
        if kids.len() == 1 {
            // Pass through to the single `unary` child.
            if let ASTNodeOrToken::Node(n) = kids[0] {
                return self.emit_expr(n);
            }
        }
        if kids.len() == 3
            && matches!(kids[1], ASTNodeOrToken::Token(t) if t.value == "^")
        {
            let base_node = match kids[0] {
                ASTNodeOrToken::Node(n) => n,
                _ => return Err(CompileError::Malformed(
                    "power lhs is not a node".into())),
            };
            let exponent_node = match kids[2] {
                ASTNodeOrToken::Node(n) => n,
                _ => return Err(CompileError::Malformed(
                    "power rhs is not a node".into())),
            };
            // Fast path: literal small nonneg integer exponent → repeated f64 mul.
            if let Some(exponent) = literal_integer_exponent(exponent_node)? {
                let base = self.emit_expr(base_node)?;
                let base = self.coerce_value(base, BasicScalarType::Real);
                if exponent == 0 {
                    let dest = self.fresh_temp();
                    self.emit("const", Some(&dest), vec![Operand::Float(1.0)], "f64");
                    return Ok(ExprValue { slot: dest, ty: BasicScalarType::Real });
                }
                let base_slot = base.slot.clone();
                let mut acc = base.slot;
                for _ in 1..exponent {
                    let dest = self.fresh_temp();
                    self.emit("mul", Some(&dest),
                        vec![Operand::Var(acc), Operand::Var(base_slot.clone())], "f64");
                    acc = dest;
                }
                return Ok(ExprValue { slot: acc, ty: BasicScalarType::Real });
            }
            // General case: runtime pow(base, exp) via the f64_pow IIR op.
            let base = self.emit_expr(base_node)?;
            let base = self.coerce_value(base, BasicScalarType::Real);
            let exp_val = self.emit_expr(exponent_node)?;
            let exp_val = self.coerce_value(exp_val, BasicScalarType::Real);
            let dest = self.fresh_temp();
            self.emit("f64_pow", Some(&dest),
                vec![Operand::Var(base.slot), Operand::Var(exp_val.slot)], "f64");
            return Ok(ExprValue { slot: dest, ty: BasicScalarType::Real });
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
        -> Result<ExprValue, CompileError>
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
        self.emit("neg", Some(&dest), vec![Operand::Var(v.slot)], v.ty.iir());
        Ok(ExprValue { slot: dest, ty: v.ty })
    }

    fn emit_primary(&mut self, node: &GrammarASTNode)
        -> Result<ExprValue, CompileError>
    {
        // primary = NUMBER | BUILTIN_FN(expr) | USER_FN(expr) | variable | (expr)
        //
        // V1 supports NUMBER, `variable`, `USER_FN(expr)`, and built-in functions.
        // E3 (reals) is complete, so SQR/INT/ABS/SGN are now lowered inline.
        for c in &node.children {
            match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NUMBER" => {
                    let raw = t.value.trim();
                    let dest = self.fresh_temp();
                    let v = raw.parse::<f64>().map_err(|_| CompileError::Malformed(
                        format!("NUMBER literal `{raw}` is not a real value")))?;
                    if !v.is_finite() {
                        return Err(CompileError::Unsupported(format!(
                            "non-finite real literal `{raw}`")));
                    }
                    self.emit("const", Some(&dest),
                        vec![Operand::Float(v)], "f64");
                    return Ok(ExprValue { slot: dest, ty: BasicScalarType::Real });
                }
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "STRING" => {
                    return Err(CompileError::Unsupported(
                        format!("string literal `{}` in numeric expression", t.value)));
                }
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "USER_FN" => {
                    // `USER_FN LPAREN expr RPAREN` — a call to a user-defined
                    // function (BA5).  Lower to the same IIR `call` convention
                    // ALGOL's value procedures use: `call dest = callee, arg`.
                    return self.emit_user_fn_call(&t.value, node);
                }
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "BUILTIN_FN" => {
                    return self.emit_builtin_fn(&t.value.to_lowercase(), node);
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "variable" => {
                    // `A(I)` / `A(I,J)` reads an array element (BA3 / BA-DIM-2D /
                    // E5) → `array_get` at the flat row-major index.
                    let subs = array_subscript_indices(n);
                    if !subs.is_empty() {
                        let name = array_target_name(n)?;
                        if !self.arrays.contains_key(&name) {
                            return Err(CompileError::Unsupported(format!(
                                "`{name}(...)` is read but `{name}` was never \
                                 DIMmed — declare it with `DIM {name}(n)` first")));
                        }
                        // A `$`-named (string) array read reaching the numeric
                        // path is a type error — string arrays live only in
                        // string context (PRINT, `+` concat, string `=`), where
                        // `emit_basic_string_expr_to` handles the `str` array_get.
                        if self.string_arrays.contains(&name) {
                            return Err(CompileError::Unsupported(format!(
                                "string array element `{name}(...)` used in a \
                                 numeric expression")));
                        }
                        let flat = self.emit_flat_index(&name, &subs)?;
                        let dest = self.fresh_temp();
                        self.emit("array_get", Some(&dest),
                            vec![Operand::Var(name), Operand::Var(flat)], "f64");
                        return Ok(ExprValue { slot: dest, ty: BasicScalarType::Real });
                    }
                    let name = numeric_scalar_variable_name(n)?;
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
                        return Ok(ExprValue { slot: name, ty: BasicScalarType::Real });
                    }
                    let ty = self.scalar_types.get(&name).copied()
                        .unwrap_or(BasicScalarType::Real);
                    return Ok(ExprValue { slot: name, ty });
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
    /// `f64` return hint — the calling convention every backend understands
    /// (`srcs[0]` names the callee, the rest are argument slots).  This is
    /// the BASIC counterpart of ALGOL's value-procedure calls (AL3), which
    /// already run on native/LLVM/WASM/JVM/CLR/VM/JIT.
    fn emit_user_fn_call(&mut self, name: &str, node: &GrammarASTNode)
        -> Result<ExprValue, CompileError>
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
        let arg = self.coerce_value(arg, BasicScalarType::Real);
        let dest = self.fresh_temp();
        self.emit("call", Some(&dest),
            vec![Operand::Var(name.to_string()), Operand::Var(arg.slot)], "f64");
        Ok(ExprValue { slot: dest, ty: BasicScalarType::Real })
    }

    /// Lower a Dartmouth BASIC built-in math function call.
    ///
    /// `name` is the lower-cased `BUILTIN_FN` token value (`sqr`, `int`,
    /// `abs`, `sgn`, …); `node` is the enclosing `primary`.  The argument is
    /// the single `expr` child of that primary (same shape as `USER_FN`).
    ///
    /// Implemented now (no new backend ops needed — all reuse E3/E8 IIR ops):
    ///
    /// | Function | Lowers to            | Notes                              |
    /// |----------|----------------------|------------------------------------|
    /// | `SQR(X)` | `f64_sqrt`           | hardware sqrt on all 7 backends    |
    /// | `INT(X)` | `real_to_int_floor` → `int_to_real` | floor, result is f64 |
    /// | `ABS(X)` | inline if/neg/jmp    | store-per-branch, no phi          |
    /// | `SGN(X)` | inline 3-way if/jmp  | -1.0 / 0.0 / 1.0 per BA7 model   |
    /// | `ATN(X)` | `f64_atan`           | arctan via libm/Math.atan etc.     |
    /// | `TAN(X)` | `f64_tan`            | tangent via libm/Math.tan etc.     |
    ///
    /// `TAN` and `ATN` now also map to `f64_tan`/`f64_atan` IIR ops (AL8-arctan
    /// infrastructure, same pattern as SIN/COS/LOG/EXP from BA-trig).
    /// `RND` still needs a cross-backend RNG and is rejected with a clear error.
    fn emit_builtin_fn(&mut self, name: &str, node: &GrammarASTNode)
        -> Result<ExprValue, CompileError>
    {
        let arg_node = child_nodes(node).into_iter()
            .find(|n| n.rule_name == "expr")
            .ok_or_else(|| CompileError::Malformed(format!(
                "built-in function `{name}` missing argument expr")))?;
        let arg = self.emit_expr(arg_node)?;
        let arg = self.coerce_value(arg, BasicScalarType::Real);

        match name {
            // SQR(X) = √X — reuse the f64_sqrt IIR op added for ALGOL sqrt.
            "sqr" => {
                let dest = self.fresh_temp();
                self.emit("f64_sqrt", Some(&dest),
                    vec![Operand::Var(arg.slot)], "f64");
                Ok(ExprValue { slot: dest, ty: BasicScalarType::Real })
            }

            // INT(X) = ⌊X⌋ — floor toward −∞, result is a real per BA7.
            // real_to_int_floor + int_to_real: both are E8 ops present on
            // every backend.  (BASIC INT differs from ALGOL entier only in
            // that it returns a float, not an integer — same value.)
            "int" => {
                let floored = self.fresh_temp();
                self.emit("real_to_int_floor", Some(&floored),
                    vec![Operand::Var(arg.slot)], "i64");
                let dest = self.fresh_temp();
                self.emit("int_to_real", Some(&dest),
                    vec![Operand::Var(floored)], "f64");
                Ok(ExprValue { slot: dest, ty: BasicScalarType::Real })
            }

            // ABS(X) = |X| — inline conditional: if X < 0 then −X else X.
            // The store-per-branch discipline (one `mov` per path, no phi)
            // is the same pattern ALGOL abs uses (AL8).
            "abs" => {
                let id = self.temp_counter;
                self.temp_counter += 1;
                let else_lbl = format!("_abs_else_{id}");
                let end_lbl  = format!("_abs_end_{id}");

                let zero = self.fresh_temp();
                self.emit("const", Some(&zero), vec![Operand::Float(0.0)], "f64");
                let cond = self.fresh_temp();
                self.emit("cmp_lt", Some(&cond),
                    vec![Operand::Var(arg.slot.clone()), Operand::Var(zero)], "f64");
                let dest = self.fresh_temp();
                // then: X < 0 → negate
                self.emit("jmp_if_false", None,
                    vec![Operand::Var(cond), Operand::Var(else_lbl.clone())], "void");
                let neg = self.fresh_temp();
                self.emit("neg", Some(&neg), vec![Operand::Var(arg.slot.clone())], "f64");
                self.emit("mov", Some(&dest), vec![Operand::Var(neg)], "f64");
                self.emit("jmp", None, vec![Operand::Var(end_lbl.clone())], "void");
                // else: X >= 0 → keep
                self.emit("label", None, vec![Operand::Var(else_lbl)], "void");
                self.emit("mov", Some(&dest), vec![Operand::Var(arg.slot)], "f64");
                self.emit("label", None, vec![Operand::Var(end_lbl)], "void");
                Ok(ExprValue { slot: dest, ty: BasicScalarType::Real })
            }

            // SGN(X) = sign of X: 1.0 if X > 0, −1.0 if X < 0, 0.0 if X = 0.
            // Result is f64 (the BA7 value model has no separate integer type).
            "sgn" => {
                let id = self.temp_counter;
                self.temp_counter += 1;
                let neg_lbl  = format!("_sgn_neg_{id}");
                let zero_lbl = format!("_sgn_zero_{id}");
                let end_lbl  = format!("_sgn_end_{id}");

                let dest = self.fresh_temp();
                let z = self.fresh_temp();
                self.emit("const", Some(&z), vec![Operand::Float(0.0)], "f64");

                // X > 0 → 1.0
                let gt = self.fresh_temp();
                self.emit("cmp_gt", Some(&gt),
                    vec![Operand::Var(arg.slot.clone()), Operand::Var(z.clone())], "f64");
                self.emit("jmp_if_false", None,
                    vec![Operand::Var(gt), Operand::Var(neg_lbl.clone())], "void");
                self.emit("const", Some(&dest), vec![Operand::Float(1.0)], "f64");
                self.emit("jmp", None, vec![Operand::Var(end_lbl.clone())], "void");

                // X < 0 → −1.0
                self.emit("label", None, vec![Operand::Var(neg_lbl)], "void");
                let lt = self.fresh_temp();
                self.emit("cmp_lt", Some(&lt),
                    vec![Operand::Var(arg.slot), Operand::Var(z)], "f64");
                self.emit("jmp_if_false", None,
                    vec![Operand::Var(lt), Operand::Var(zero_lbl.clone())], "void");
                self.emit("const", Some(&dest), vec![Operand::Float(-1.0)], "f64");
                self.emit("jmp", None, vec![Operand::Var(end_lbl.clone())], "void");

                // X = 0 → 0.0
                self.emit("label", None, vec![Operand::Var(zero_lbl)], "void");
                self.emit("const", Some(&dest), vec![Operand::Float(0.0)], "f64");
                self.emit("label", None, vec![Operand::Var(end_lbl)], "void");
                Ok(ExprValue { slot: dest, ty: BasicScalarType::Real })
            }

            // ATN(X) = arctan(X) — the AL8-arctan `f64_atan` IIR op, same
            // pattern as SIN/COS from BA-trig.  ATN is the standard
            // Dartmouth BASIC name for arctangent (§5 of the manual).
            "atn" => {
                let dest = self.fresh_temp();
                self.emit("f64_atan", Some(&dest),
                    vec![Operand::Var(arg.slot)], "f64");
                Ok(ExprValue { slot: dest, ty: BasicScalarType::Real })
            }

            // TAN(X) = tan(X) — the AL8-arctan `f64_tan` IIR op.
            "tan" => {
                let dest = self.fresh_temp();
                self.emit("f64_tan", Some(&dest),
                    vec![Operand::Var(arg.slot)], "f64");
                Ok(ExprValue { slot: dest, ty: BasicScalarType::Real })
            }

            _ => Err(CompileError::Unsupported(format!(
                "built-in function `{}` not yet implemented \
                 (needs cross-backend math support)",
                name.to_uppercase())))
        }
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
    ///   10 DEF FNS(X) = X * X      ⇒   fn fns(X: f64) -> f64 { ret X * X }
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
        let result = self.coerce_value(result, BasicScalarType::Real);
        self.emit("ret", None, vec![Operand::Var(result.slot)], "f64");

        // ── assemble the function and restore main's context ────────────
        let body_instrs = std::mem::take(&mut self.instrs);
        let body_len = body_instrs.len();
        let mut func = IIRFunction::new(
            name,
            vec![(param, "f64".to_string())],
            "f64",
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

/// The subscript `expr` nodes of a variable, in source order.  For a scalar
/// `NAME` this is empty; for `A(I)` it is one element; for `A(I,J)` (BA-DIM-2D)
/// it is two.  This is the single place that distinguishes an array access from
/// a plain variable — the `LET` write, `READ`, and expression read paths all use
/// it (BA3 / BA-DIM-2D / enabler **E5**).  `variable = NAME LPAREN expr
/// { COMMA expr } RPAREN | NAME`, so every direct `expr` child is a subscript.
fn array_subscript_indices(var: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    child_nodes(var).into_iter().filter(|n| n.rule_name == "expr").collect()
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
/// The `GOSUB` return-address stack (BA1 / enabler E7): an `array<i64>` holding
/// the id of each pending `GOSUB`'s return site, LIFO.
const BASIC_GOSUB_STACK: &str = "__basic_gosub_stack";
/// The stack pointer into [`BASIC_GOSUB_STACK`] — index of the next free slot.
const BASIC_GOSUB_SP: &str = "__basic_gosub_sp";
/// Fixed capacity of the `GOSUB` stack.  Dartmouth BASIC programs nest only a
/// few levels deep; pushing past this traps via the bounds-checked `array_set`
/// (the faithful "GOSUB nesting too deep" runtime error).
const BASIC_GOSUB_STACK_DEPTH: i64 = 64;

const BASIC_DATA_ARRAY: &str = "__basic_data";
/// The IIR register holding the `READ`/`RESTORE` pointer into the pool (BA6).
const BASIC_DATA_PTR: &str = "__basic_data_ptr";

/// Largest exponent the frontend will unroll for `base ^ <literal>`.
/// This is a deliberately small no-runtime-helper slice; larger/general
/// exponents should use a real cross-backend math helper later.
const MAX_LITERAL_EXPONENT: u32 = 64;

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
/// truncated toward zero because the DIM bound remains an integer structural
/// boundary even though BASIC values are otherwise real in BA7.
fn dim_decl_bounds(decl: &GrammarASTNode) -> Result<Vec<i64>, CompileError> {
    let mut bounds = Vec::new();
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
                bounds.push(f as i64);
            }
        }
    }
    if bounds.is_empty() {
        return Err(CompileError::Malformed("DIM decl missing NUMBER bound".into()));
    }
    Ok(bounds)
}

fn literal_integer_exponent(node: &GrammarASTNode) -> Result<Option<u32>, CompileError> {
    let mut number: Option<&str> = None;
    let mut unsupported_shape = false;

    fn visit<'a>(node: &'a GrammarASTNode, number: &mut Option<&'a str>,
                 unsupported_shape: &mut bool)
    {
        for c in &node.children {
            match c {
                ASTNodeOrToken::Node(n) => visit(n, number, unsupported_shape),
                ASTNodeOrToken::Token(t) => match t.effective_type_name() {
                    "NUMBER" => {
                        if number.replace(t.value.trim()).is_some() {
                            *unsupported_shape = true;
                        }
                    }
                    "LPAREN" | "RPAREN" => {}
                    _ => *unsupported_shape = true,
                },
            }
        }
    }

    visit(node, &mut number, &mut unsupported_shape);
    if unsupported_shape {
        return Ok(None);
    }
    let Some(raw) = number else {
        return Ok(None);
    };
    let value = raw.parse::<f64>().map_err(|_| CompileError::Malformed(
        format!("exponent literal `{raw}` is not a real value")))?;
    if !value.is_finite()
        || value < 0.0
        || value.fract() != 0.0
        || value > MAX_LITERAL_EXPONENT as f64
    {
        // Not a small nonnegative integer literal — caller falls through to f64_pow.
        return Ok(None);
    }
    Ok(Some(value as u32))
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

fn numeric_scalar_variable_name(var: &GrammarASTNode)
    -> Result<String, CompileError>
{
    let name = scalar_variable_name(var)?;
    if is_basic_string_name(&name) {
        return Err(CompileError::Unsupported(format!(
            "string variable `{name}` is only supported in literal assignment, PRINT, and IF string comparisons")));
    }
    Ok(name)
}

fn is_basic_string_name(name: &str) -> bool {
    name.ends_with('$')
}

fn basic_string_slot(name: &str) -> String {
    let stem = name.strip_suffix('$').unwrap_or(name);
    format!("__basic_str_{stem}")
}

/// The IIR register that holds an array's aggregate handle.
///
/// A numeric array uses its bare BASIC name (`A` → register `A`).  A **string**
/// array (`A$`) cannot: `$` is not a portable IIR register-name character (the
/// scalar string path sanitises the same way via [`basic_string_slot`]), and
/// BASIC lets a numeric array `A` and a string array `A$` coexist as distinct
/// variables — so the string array gets its own prefixed, `$`-free handle
/// register that can never collide with the numeric `A`.  Keyed lookups
/// (`arrays`, `string_arrays`, `emit_flat_index`) still use the original BASIC
/// name; only the *emitted register operand* is sanitised.
fn basic_array_handle(name: &str) -> String {
    if let Some(stem) = name.strip_suffix('$') {
        format!("__basic_strarr_{stem}")
    } else {
        name.to_string()
    }
}

fn single_child_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    if node.children.len() != 1 {
        return None;
    }
    match node.children.first()? {
        ASTNodeOrToken::Node(n) => Some(n),
        ASTNodeOrToken::Token(_) => None,
    }
}

fn expr_string_literal(node: &GrammarASTNode) -> Option<String> {
    if node.rule_name == "primary" {
        return string_token_value(node);
    }
    single_child_node(node).and_then(expr_string_literal)
}

fn expr_plain_variable(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    if node.rule_name == "primary" {
        return child_nodes(node).into_iter().find(|n| n.rule_name == "variable");
    }
    single_child_node(node).and_then(expr_plain_variable)
}

fn expr_string_variable_name(node: &GrammarASTNode)
    -> Result<Option<String>, CompileError>
{
    let Some(var) = expr_plain_variable(node) else {
        return Ok(None);
    };
    if !array_subscript_indices(var).is_empty() {
        return Ok(None);
    }
    let name = scalar_variable_name(var)?;
    if is_basic_string_name(&name) {
        Ok(Some(name))
    } else {
        Ok(None)
    }
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

fn string_token_value(node: &GrammarASTNode) -> Option<String> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "STRING" {
                return Some(unquote_basic_string(&t.value));
            }
        }
    }
    None
}

fn unquote_basic_string(raw: &str) -> String {
    let s = raw.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
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

/// The separator character carried by a `print_sep` node — `,` or `;`.
/// (`print_sep = COMMA | SEMICOLON`, so the node has exactly one token
/// child.)  Falls back to `;` (the tight, space-free join) if the token is
/// somehow missing, so a malformed parse never injects stray spaces.
fn print_sep_char(node: &GrammarASTNode) -> char {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            match t.value.trim() {
                "," => return ',',
                ";" => return ';',
                _ => {}
            }
        }
    }
    ';'
}

/// Synthetic helper functions BA2/BA7 `PRINT` lowering calls to render numeric
/// values one character at a time (so several items can share a line).
/// They are appended to the module after `main` only when a program actually
/// `PRINT`s a value (`Compiler::needs_print_helpers`).
///
/// They use **only** ops every backend already runs — `const`, `cmp_*`,
/// `div`/`mul`/`sub`/`add`, `call` (the ALGOL value-procedure ABI, AL3),
/// `jmp`/`label`, and the universal `putchar` builtin (shared with Brainfuck)
/// — so BA2 needs **zero** backend changes and runs on all seven targets.
///
/// ```text
///   fn __basic_print_uint(n):              # n >= 0
///       if n >= 10:
///           __basic_print_uint(n / 10)     # high-order digits first…
///       putchar('0' + n - (n / 10) * 10)   # …then this digit (the last)
///
///   fn __basic_print_int(n):
///       if n < 0:
///           putchar('-'); __basic_print_uint(0 - n)
///       else:
///           __basic_print_uint(n)
///
///   fn __basic_print_real(x):
///       if x < 0: putchar('-'); x = 0.0 - x
///       ip = real_to_int_trunc(x)
///       frac = x - int_to_real(ip)
///       if frac == 0: __basic_print_uint(ip)
///       else: [__basic_print_uint(ip) if ip > 0]; putchar('.'); print frac digits
/// ```
///
/// The recursion is what gets the digits out in left-to-right order with no
/// reversal buffer: the deepest call prints the most-significant digit first.
/// (`0 - n` for the sign overflows only at `i64::MIN`, a value no BA2 program
/// can express; a saturating negate is a later refinement.)
fn print_helper_functions() -> Vec<IIRFunction> {
    fn var(s: &str) -> Operand { Operand::Var(s.to_string()) }
    let mk = |op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str| {
        IIRInstr::new(op, dest.map(|s| s.to_string()), srcs, ty)
    };

    // __basic_print_uint(n) — unsigned magnitude, recursive.
    let uint_body = vec![
        mk("const", Some("ten"), vec![Operand::Int(10)], "i64"),
        // n >= 10 ?  (operand width i64, like every BASIC compare — a "bool"
        // hint would make the backends emit a 1-bit compare, the BA0 bug.)
        mk("cmp_ge", Some("hi"), vec![var("n"), var("ten")], "i64"),
        mk("jmp_if_false", None, vec![var("hi"), var("uint_tail")], "void"),
        mk("div", Some("hq"), vec![var("n"), var("ten")], "i64"),
        mk("call", Some("_r"),
            vec![var("__basic_print_uint"), var("hq")], "i64"),
        mk("label", None, vec![var("uint_tail")], "void"),
        // last digit = n - (n / 10) * 10
        mk("div", Some("q"), vec![var("n"), var("ten")], "i64"),
        mk("mul", Some("qt"), vec![var("q"), var("ten")], "i64"),
        mk("sub", Some("rem"), vec![var("n"), var("qt")], "i64"),
        mk("const", Some("c0"), vec![Operand::Int(b'0' as i64)], "i64"),
        mk("add", Some("digit"), vec![var("c0"), var("rem")], "i64"),
        mk("call_builtin", None, vec![var("putchar"), var("digit")], "void"),
        mk("const", Some("z"), vec![Operand::Int(0)], "i64"),
        mk("ret", None, vec![var("z")], "i64"),
    ];

    // __basic_print_int(n) — sign dispatch over the magnitude helper.
    let int_body = vec![
        mk("const", Some("zero"), vec![Operand::Int(0)], "i64"),
        mk("cmp_lt", Some("neg"), vec![var("n"), var("zero")], "i64"),
        mk("jmp_if_false", None, vec![var("neg"), var("int_nonneg")], "void"),
        mk("const", Some("minus"), vec![Operand::Int(b'-' as i64)], "i64"),
        mk("call_builtin", None, vec![var("putchar"), var("minus")], "void"),
        mk("sub", Some("mag"), vec![var("zero"), var("n")], "i64"),
        mk("call", Some("_rn"),
            vec![var("__basic_print_uint"), var("mag")], "i64"),
        mk("jmp", None, vec![var("int_done")], "void"),
        mk("label", None, vec![var("int_nonneg")], "void"),
        mk("call", Some("_rp"),
            vec![var("__basic_print_uint"), var("n")], "i64"),
        mk("label", None, vec![var("int_done")], "void"),
        mk("const", Some("z2"), vec![Operand::Int(0)], "i64"),
        mk("ret", None, vec![var("z2")], "i64"),
    ];

    // __basic_print_zeros(count) — emit exactly `count` ASCII zeroes.
    let zeros_body = vec![
        mk("const", Some("zero"), vec![Operand::Int(0)], "i64"),
        mk("const", Some("one"), vec![Operand::Int(1)], "i64"),
        mk("const", Some("c0"), vec![Operand::Int(b'0' as i64)], "i64"),
        mk("label", None, vec![var("zeros_loop")], "void"),
        mk("cmp_le", Some("done"), vec![var("count"), var("zero")], "i64"),
        mk("jmp_if_true", None, vec![var("done"), var("zeros_done")], "void"),
        mk("call_builtin", None, vec![var("putchar"), var("c0")], "void"),
        mk("sub", Some("count"), vec![var("count"), var("one")], "i64"),
        mk("jmp", None, vec![var("zeros_loop")], "void"),
        mk("label", None, vec![var("zeros_done")], "void"),
        mk("const", Some("z"), vec![Operand::Int(0)], "i64"),
        mk("ret", None, vec![var("z")], "i64"),
    ];

    // __basic_print_fixed_mag(mag, places, scale, skip_zero) — print a positive
    // fixed-decimal magnitude with six-significant-digit round-half-up already
    // encoded in (`places`, `scale`). `skip_zero` omits the leading zero for
    // values like `.25`; trailing fractional zeroes are trimmed.
    let fixed_body = vec![
        mk("const", Some("zero_i"), vec![Operand::Int(0)], "i64"),
        mk("const", Some("one_i"), vec![Operand::Int(1)], "i64"),
        mk("const", Some("ten_i"), vec![Operand::Int(10)], "i64"),
        mk("const", Some("half_r"), vec![Operand::Float(0.5)], "f64"),
        mk("int_to_real", Some("scale_r"), vec![var("scale")], "f64"),
        mk("mul", Some("scaled"), vec![var("mag"), var("scale_r")], "f64"),
        mk("add", Some("rounded"), vec![var("scaled"), var("half_r")], "f64"),
        mk("real_to_int_trunc", Some("n"), vec![var("rounded")], "i64"),
        mk("div", Some("ip"), vec![var("n"), var("scale")], "i64"),
        mk("mul", Some("ip_scaled"), vec![var("ip"), var("scale")], "i64"),
        mk("sub", Some("frac"), vec![var("n"), var("ip_scaled")], "i64"),
        mk("cmp_eq", Some("no_frac"), vec![var("frac"), var("zero_i")], "i64"),
        mk("jmp_if_false", None, vec![var("no_frac"), var("fixed_fraction")], "void"),
        mk("call", Some("_whole"),
            vec![var("__basic_print_uint"), var("ip")], "i64"),
        mk("jmp", None, vec![var("fixed_done")], "void"),
        mk("label", None, vec![var("fixed_fraction")], "void"),
        mk("label", None, vec![var("trim_loop")], "void"),
        mk("div", Some("q"), vec![var("frac"), var("ten_i")], "i64"),
        mk("mul", Some("qt"), vec![var("q"), var("ten_i")], "i64"),
        mk("sub", Some("rem"), vec![var("frac"), var("qt")], "i64"),
        mk("cmp_ne", Some("rem_nonzero"), vec![var("rem"), var("zero_i")], "i64"),
        mk("jmp_if_true", None, vec![var("rem_nonzero"), var("trim_done")], "void"),
        mk("add", Some("frac"), vec![var("q"), var("zero_i")], "i64"),
        mk("sub", Some("places"), vec![var("places"), var("one_i")], "i64"),
        mk("jmp", None, vec![var("trim_loop")], "void"),
        mk("label", None, vec![var("trim_done")], "void"),
        mk("cmp_eq", Some("skip_one"), vec![var("skip_zero"), var("one_i")], "i64"),
        mk("jmp_if_false", None, vec![var("skip_one"), var("print_ip")], "void"),
        mk("cmp_eq", Some("ip_zero"), vec![var("ip"), var("zero_i")], "i64"),
        mk("jmp_if_true", None, vec![var("ip_zero"), var("after_ip")], "void"),
        mk("label", None, vec![var("print_ip")], "void"),
        mk("call", Some("_ip"), vec![var("__basic_print_uint"), var("ip")], "i64"),
        mk("label", None, vec![var("after_ip")], "void"),
        mk("const", Some("dot"), vec![Operand::Int(b'.' as i64)], "i64"),
        mk("call_builtin", None, vec![var("putchar"), var("dot")], "void"),
        mk("const", Some("digits"), vec![Operand::Int(1)], "i64"),
        mk("add", Some("tmp"), vec![var("frac"), var("zero_i")], "i64"),
        mk("label", None, vec![var("digit_count_loop")], "void"),
        mk("cmp_ge", Some("many"), vec![var("tmp"), var("ten_i")], "i64"),
        mk("jmp_if_false", None, vec![var("many"), var("digit_count_done")], "void"),
        mk("div", Some("tmp"), vec![var("tmp"), var("ten_i")], "i64"),
        mk("add", Some("digits"), vec![var("digits"), var("one_i")], "i64"),
        mk("jmp", None, vec![var("digit_count_loop")], "void"),
        mk("label", None, vec![var("digit_count_done")], "void"),
        mk("sub", Some("zeros"), vec![var("places"), var("digits")], "i64"),
        mk("call", Some("_zeros"),
            vec![var("__basic_print_zeros"), var("zeros")], "i64"),
        mk("call", Some("_frac"),
            vec![var("__basic_print_uint"), var("frac")], "i64"),
        mk("label", None, vec![var("fixed_done")], "void"),
        mk("const", Some("z"), vec![Operand::Int(0)], "i64"),
        mk("ret", None, vec![var("z")], "i64"),
    ];

    // __basic_print_real_e(mag) — scientific notation with a six-significant
    // digit mantissa and a signed, at-least-two-digit exponent (`E+08`).
    let real_e_body = vec![
        mk("const", Some("zero_i"), vec![Operand::Int(0)], "i64"),
        mk("const", Some("one_i"), vec![Operand::Int(1)], "i64"),
        mk("const", Some("ten_i"), vec![Operand::Int(10)], "i64"),
        mk("const", Some("zero_r"), vec![Operand::Float(0.0)], "f64"),
        mk("const", Some("one_r"), vec![Operand::Float(1.0)], "f64"),
        mk("const", Some("ten_r"), vec![Operand::Float(10.0)], "f64"),
        mk("const", Some("carry_r"), vec![Operand::Float(9.999995)], "f64"),
        mk("const", Some("places"), vec![Operand::Int(5)], "i64"),
        mk("const", Some("scale"), vec![Operand::Int(100_000)], "i64"),
        mk("const", Some("skip_zero"), vec![Operand::Int(0)], "i64"),
        mk("const", Some("exp"), vec![Operand::Int(0)], "i64"),
        mk("add", Some("m"), vec![var("mag"), var("zero_r")], "f64"),
        mk("label", None, vec![var("e_high_loop")], "void"),
        mk("cmp_ge", Some("too_high"), vec![var("m"), var("ten_r")], "f64"),
        mk("jmp_if_false", None, vec![var("too_high"), var("e_low_loop")], "void"),
        mk("div", Some("m"), vec![var("m"), var("ten_r")], "f64"),
        mk("add", Some("exp"), vec![var("exp"), var("one_i")], "i64"),
        mk("jmp", None, vec![var("e_high_loop")], "void"),
        mk("label", None, vec![var("e_low_loop")], "void"),
        mk("cmp_lt", Some("too_low"), vec![var("m"), var("one_r")], "f64"),
        mk("jmp_if_false", None, vec![var("too_low"), var("e_normalized")], "void"),
        mk("mul", Some("m"), vec![var("m"), var("ten_r")], "f64"),
        mk("sub", Some("exp"), vec![var("exp"), var("one_i")], "i64"),
        mk("jmp", None, vec![var("e_low_loop")], "void"),
        mk("label", None, vec![var("e_normalized")], "void"),
        mk("cmp_ge", Some("carry"), vec![var("m"), var("carry_r")], "f64"),
        mk("jmp_if_false", None, vec![var("carry"), var("e_print_mantissa")], "void"),
        mk("add", Some("m"), vec![var("one_r"), var("zero_r")], "f64"),
        mk("add", Some("exp"), vec![var("exp"), var("one_i")], "i64"),
        mk("label", None, vec![var("e_print_mantissa")], "void"),
        mk("call", Some("_mant"),
            vec![var("__basic_print_fixed_mag"), var("m"), var("places"),
                 var("scale"), var("skip_zero")], "i64"),
        mk("const", Some("e_ch"), vec![Operand::Int(b'E' as i64)], "i64"),
        mk("call_builtin", None, vec![var("putchar"), var("e_ch")], "void"),
        mk("cmp_lt", Some("exp_neg"), vec![var("exp"), var("zero_i")], "i64"),
        mk("jmp_if_false", None, vec![var("exp_neg"), var("e_exp_pos")], "void"),
        mk("const", Some("minus"), vec![Operand::Int(b'-' as i64)], "i64"),
        mk("call_builtin", None, vec![var("putchar"), var("minus")], "void"),
        mk("sub", Some("exp_mag"), vec![var("zero_i"), var("exp")], "i64"),
        mk("jmp", None, vec![var("e_exp_sign_done")], "void"),
        mk("label", None, vec![var("e_exp_pos")], "void"),
        mk("const", Some("plus"), vec![Operand::Int(b'+' as i64)], "i64"),
        mk("call_builtin", None, vec![var("putchar"), var("plus")], "void"),
        mk("add", Some("exp_mag"), vec![var("exp"), var("zero_i")], "i64"),
        mk("label", None, vec![var("e_exp_sign_done")], "void"),
        mk("cmp_lt", Some("one_digit"), vec![var("exp_mag"), var("ten_i")], "i64"),
        mk("jmp_if_false", None, vec![var("one_digit"), var("e_exp_digits")], "void"),
        mk("call", Some("_pad"),
            vec![var("__basic_print_zeros"), var("one_i")], "i64"),
        mk("label", None, vec![var("e_exp_digits")], "void"),
        mk("call", Some("_exp"),
            vec![var("__basic_print_uint"), var("exp_mag")], "i64"),
        mk("const", Some("z"), vec![Operand::Int(0)], "i64"),
        mk("ret", None, vec![var("z")], "i64"),
    ];

    // __basic_print_real(x) — BA7 real output. The heavy formatting work lives
    // in small helpers so the direct AArch64 backend stays under its frame-size
    // limit while still supporting six significant digits and E notation.
    let real_body = vec![
        mk("const", Some("zero_i"), vec![Operand::Int(0)], "i64"),
        mk("const", Some("zero_r"), vec![Operand::Float(0.0)], "f64"),
        mk("const", Some("one_i"), vec![Operand::Int(1)], "i64"),
        mk("const", Some("one_tenth_r"), vec![Operand::Float(0.1)], "f64"),
        mk("const", Some("one_r"), vec![Operand::Float(1.0)], "f64"),
        mk("const", Some("ten_r"), vec![Operand::Float(10.0)], "f64"),
        mk("const", Some("hundred_r"), vec![Operand::Float(100.0)], "f64"),
        mk("const", Some("thousand_r"), vec![Operand::Float(1_000.0)], "f64"),
        mk("const", Some("ten_thousand_r"), vec![Operand::Float(10_000.0)], "f64"),
        mk("const", Some("hundred_thousand_r"), vec![Operand::Float(100_000.0)], "f64"),
        mk("const", Some("e_high_r"), vec![Operand::Float(999_999.5)], "f64"),
        mk("cmp_lt", Some("neg"), vec![var("x"), var("zero_r")], "f64"),
        mk("jmp_if_false", None, vec![var("neg"), var("real_nonneg")], "void"),
        mk("const", Some("minus"), vec![Operand::Int(b'-' as i64)], "i64"),
        mk("call_builtin", None, vec![var("putchar"), var("minus")], "void"),
        mk("sub", Some("mag"), vec![var("zero_r"), var("x")], "f64"),
        mk("jmp", None, vec![var("real_abs_done")], "void"),
        mk("label", None, vec![var("real_nonneg")], "void"),
        mk("add", Some("mag"), vec![var("x"), var("zero_r")], "f64"),
        mk("label", None, vec![var("real_abs_done")], "void"),
        mk("cmp_eq", Some("is_zero"), vec![var("mag"), var("zero_r")], "f64"),
        mk("jmp_if_false", None, vec![var("is_zero"), var("real_nonzero")], "void"),
        mk("call", Some("_zero"),
            vec![var("__basic_print_uint"), var("zero_i")], "i64"),
        mk("jmp", None, vec![var("real_done")], "void"),
        mk("label", None, vec![var("real_nonzero")], "void"),
        mk("cmp_lt", Some("e_low"), vec![var("mag"), var("one_tenth_r")], "f64"),
        mk("jmp_if_true", None, vec![var("e_low"), var("real_e")], "void"),
        mk("cmp_ge", Some("e_high"), vec![var("mag"), var("e_high_r")], "f64"),
        mk("jmp_if_true", None, vec![var("e_high"), var("real_e")], "void"),
        mk("const", Some("places"), vec![Operand::Int(6)], "i64"),
        mk("const", Some("scale"), vec![Operand::Int(1_000_000)], "i64"),
        mk("const", Some("skip_zero"), vec![Operand::Int(1)], "i64"),
        mk("cmp_ge", Some("ge1"), vec![var("mag"), var("one_r")], "f64"),
        mk("jmp_if_false", None, vec![var("ge1"), var("real_print_fixed")], "void"),
        mk("const", Some("places"), vec![Operand::Int(5)], "i64"),
        mk("const", Some("scale"), vec![Operand::Int(100_000)], "i64"),
        mk("const", Some("skip_zero"), vec![Operand::Int(0)], "i64"),
        mk("cmp_ge", Some("ge10"), vec![var("mag"), var("ten_r")], "f64"),
        mk("jmp_if_false", None, vec![var("ge10"), var("real_print_fixed")], "void"),
        mk("const", Some("places"), vec![Operand::Int(4)], "i64"),
        mk("const", Some("scale"), vec![Operand::Int(10_000)], "i64"),
        mk("cmp_ge", Some("ge100"), vec![var("mag"), var("hundred_r")], "f64"),
        mk("jmp_if_false", None, vec![var("ge100"), var("real_print_fixed")], "void"),
        mk("const", Some("places"), vec![Operand::Int(3)], "i64"),
        mk("const", Some("scale"), vec![Operand::Int(1_000)], "i64"),
        mk("cmp_ge", Some("ge1000"), vec![var("mag"), var("thousand_r")], "f64"),
        mk("jmp_if_false", None, vec![var("ge1000"), var("real_print_fixed")], "void"),
        mk("const", Some("places"), vec![Operand::Int(2)], "i64"),
        mk("const", Some("scale"), vec![Operand::Int(100)], "i64"),
        mk("cmp_ge", Some("ge10000"), vec![var("mag"), var("ten_thousand_r")], "f64"),
        mk("jmp_if_false", None, vec![var("ge10000"), var("real_print_fixed")], "void"),
        mk("const", Some("places"), vec![Operand::Int(1)], "i64"),
        mk("const", Some("scale"), vec![Operand::Int(10)], "i64"),
        mk("cmp_ge", Some("ge100000"), vec![var("mag"), var("hundred_thousand_r")], "f64"),
        mk("jmp_if_false", None, vec![var("ge100000"), var("real_print_fixed")], "void"),
        mk("const", Some("places"), vec![Operand::Int(0)], "i64"),
        mk("const", Some("scale"), vec![Operand::Int(1)], "i64"),
        mk("label", None, vec![var("real_print_fixed")], "void"),
        mk("call", Some("_fixed"),
            vec![var("__basic_print_fixed_mag"), var("mag"), var("places"),
                 var("scale"), var("skip_zero")], "i64"),
        mk("jmp", None, vec![var("real_done")], "void"),
        mk("label", None, vec![var("real_e")], "void"),
        mk("call", Some("_e"),
            vec![var("__basic_print_real_e"), var("mag")], "i64"),
        mk("label", None, vec![var("real_done")], "void"),
        mk("const", Some("z"), vec![Operand::Int(0)], "i64"),
        mk("ret", None, vec![var("z")], "i64"),
    ];

    let mut funcs = Vec::new();
    for (name, params, body) in [("__basic_print_uint", vec![("n".to_string(), "i64".to_string())], uint_body),
        ("__basic_print_int", vec![("n".to_string(), "i64".to_string())], int_body),
        ("__basic_print_zeros", vec![("count".to_string(), "i64".to_string())], zeros_body),
        ("__basic_print_fixed_mag",
         vec![("mag".to_string(), "f64".to_string()),
              ("places".to_string(), "i64".to_string()),
              ("scale".to_string(), "i64".to_string()),
              ("skip_zero".to_string(), "i64".to_string())],
         fixed_body),
        ("__basic_print_real_e", vec![("mag".to_string(), "f64".to_string())], real_e_body),
        ("__basic_print_real", vec![("x".to_string(), "f64".to_string())], real_body)] {
        let len = body.len();
        let mut f = IIRFunction::new(
            name,
            params,
            "i64",
            body,
        );
        // Every op carries a concrete (non-"any") hint, so — like `main` and
        // the `DEF FN` siblings — the function is genuinely fully typed.
        f.type_status = FunctionTypeStatus::FullyTyped;
        let mut sm = Vec::with_capacity(len);
        for _ in 0..len { sm.push(SourceLoc::SYNTHETIC); }
        f.source_map = sm;
        funcs.push(f);
    }
    funcs
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

    /// `LET A = 42` followed by `END` should leave scalar A holding 42.0.
    #[test]
    fn compiles_let_then_end() {
        let m = compile("10 LET A = 42\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        // BA7: scalar BASIC numbers are f64, so A gets a real slot.
        let mov = body.iter().find(|i| i.op == "mov")
            .expect("LET produces a mov");
        assert_eq!(mov.dest.as_deref(), Some("A"));
        assert_eq!(mov.type_hint, "f64");
        // and a real const 42.0 somewhere.
        assert!(body.iter().any(|i|
            i.op == "const" && matches!(i.srcs.first(), Some(Operand::Float(v)) if (*v - 42.0).abs() < f64::EPSILON)));
    }

    /// Returns the callee name of the first `call`/`call_builtin` whose first
    /// source operand matches `name`, anywhere in `body`.
    fn calls_named(body: &[IIRInstr], name: &str) -> bool {
        body.iter().any(|i|
            (i.op == "call" || i.op == "call_builtin")
            && i.srcs.first().and_then(|s| match s {
                Operand::Var(n) => Some(n.as_str()), _ => None,
            }) == Some(name))
    }

    /// BA7: `PRINT 42` lowers through `__basic_print_real` (which delegates to
    /// BA2's digit helper for whole-valued output) — not the old `print_i64`.
    #[test]
    fn compiles_print_integer() {
        let m = compile("10 PRINT 42\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(calls_named(body, "__basic_print_real"),
            "expected call __basic_print_real in {body:?}");
        // The old builtin must be gone — same-line printing replaced it.
        assert!(!calls_named(body, "print_i64"),
            "print_i64 should no longer be emitted");
        // And the helper functions must be present in the module.
        assert!(m.functions.iter().any(|f| f.name == "__basic_print_int"));
        assert!(m.functions.iter().any(|f| f.name == "__basic_print_uint"));
    }

    /// BA7-1: decimal/exponent literals enter the staged `f64` value path.
    #[test]
    fn compiles_decimal_literal_as_f64_const() {
        let m = compile("10 LET A = 6.0\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "const"
            && i.type_hint == "f64"
            && matches!(i.srcs.first(), Some(Operand::Float(v)) if (*v - 6.0).abs() < f64::EPSILON)),
            "expected f64 const 6.0 in {body:?}");
        assert!(body.iter().any(|i| i.op == "mov"
            && i.dest.as_deref() == Some("A")
            && i.type_hint == "f64"),
            "LET A = 6.0 should store A as f64");
    }

    /// BA7-1: real arithmetic stays on the `f64` track and whole-valued PRINT
    /// delegates through `__basic_print_real`.
    #[test]
    fn float_arithmetic_uses_f64_and_print_real() {
        let m = compile("10 PRINT 6.0 * 7.0\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "mul" && i.type_hint == "f64"),
            "6.0 * 7.0 should lower to f64 mul: {body:?}");
        assert!(calls_named(body, "__basic_print_real"),
            "real PRINT should call __basic_print_real");
        assert!(m.functions.iter().any(|f| f.name == "__basic_print_real"
            && f.params == vec![("x".to_string(), "f64".to_string())]),
            "module should include f64 real print helper");
    }

    /// BA7-1b: integer-spelled scalar literals are real values too, so mixed
    /// spellings compute directly on the f64 track.
    #[test]
    fn integer_spelled_scalar_arithmetic_is_f64() {
        let m = compile("10 PRINT 40 + 2.0\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "add" && i.type_hint == "f64"),
            "mixed arithmetic result should be f64");
        assert!(calls_named(body, "__basic_print_real"));
    }

    /// BA-^: small integer-valued literal exponents lower to repeated f64
    /// multiplication, avoiding a cross-backend math runtime.
    #[test]
    fn literal_power_lowers_to_repeated_f64_mul() {
        let m = compile("10 PRINT 6 ^ 2 + 6\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "mul" && i.type_hint == "f64"),
            "`6 ^ 2` should lower to f64 repeated multiplication: {body:?}");
        assert!(calls_named(body, "__basic_print_real"));
    }

    /// BA-^: a variable exponent falls through to the two-argument f64_pow IIR op,
    /// which every backend lowers to libm pow().
    #[test]
    fn variable_power_exponent_uses_f64_pow() {
        let m = compile("10 LET X = 2\n20 PRINT 6 ^ X\n30 END\n").expect("variable exponent should compile via f64_pow");
        let body = &m.functions[0].instructions;
        assert!(
            body.iter().any(|i| i.op == "f64_pow"),
            "`6 ^ X` should lower to f64_pow IIR op: {body:?}"
        );
    }

    /// BA2: a program with no value-printing `PRINT` carries no helper
    /// functions (they're emitted lazily, only when used).
    #[test]
    fn no_print_no_helpers() {
        let m = compile("10 LET A = 1\n20 END\n").expect("ok");
        assert!(!m.functions.iter().any(|f| f.name == "__basic_print_int"),
            "helpers must not be emitted when nothing prints");
    }

    /// BA2/BA7: `PRINT 4; 2` (semicolon) joins tightly — two numeric helper
    /// calls, no separator space, and a single trailing newline.
    #[test]
    fn print_semicolon_joins_tight() {
        let m = compile("10 PRINT 4; 2\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let prints = body.iter().filter(|i| i.op == "call"
            && i.srcs.first().and_then(|s| match s {
                Operand::Var(n) => Some(n.as_str()), _ => None,
            }) == Some("__basic_print_real")).count();
        assert_eq!(prints, 2, "two items ⇒ two helper calls");
        // No space (32) const between items for ';'.
        assert!(!body.iter().any(|i| i.op == "const"
            && matches!(i.srcs.first(), Some(Operand::Int(32)))),
            "';' must not insert a space");
        // Exactly one trailing newline (10).
        assert_eq!(body.iter().filter(|i| i.op == "const"
            && matches!(i.srcs.first(), Some(Operand::Int(10)))).count(), 1,
            "one trailing newline");
    }

    /// BA2: `PRINT 4, 2` (comma) inserts a space (const 32) between items.
    #[test]
    fn print_comma_inserts_space() {
        let m = compile("10 PRINT 4, 2\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "const"
            && matches!(i.srcs.first(), Some(Operand::Int(32)))),
            "',' must insert a space (const 32)");
    }

    /// BA2: a trailing separator (`PRINT 7;`) suppresses the final newline.
    #[test]
    fn print_trailing_sep_suppresses_newline() {
        let m = compile("10 PRINT 7;\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(!body.iter().any(|i| i.op == "const"
            && matches!(i.srcs.first(), Some(Operand::Int(10)))),
            "trailing ';' must suppress the newline");
    }

    /// BA2: bare `PRINT` emits a lone newline (a blank line).
    #[test]
    fn bare_print_emits_newline() {
        let m = compile("10 PRINT\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "const"
            && matches!(i.srcs.first(), Some(Operand::Int(10)))),
            "bare PRINT must emit a newline");
        // No value printed ⇒ no helper calls.
        assert!(!calls_named(body, "__basic_print_int"));
    }

    /// BA2: the magnitude helper recurses on itself (multi-digit support).
    #[test]
    fn print_uint_helper_recurses() {
        let m = compile("10 PRINT 123\n20 END\n").expect("ok");
        let uint = m.functions.iter()
            .find(|f| f.name == "__basic_print_uint").expect("helper present");
        assert!(calls_named(&uint.instructions, "__basic_print_uint"),
            "__basic_print_uint must call itself");
        // Sign helper must dispatch to the magnitude helper.
        let int = m.functions.iter()
            .find(|f| f.name == "__basic_print_int").expect("helper present");
        assert!(calls_named(&int.instructions, "__basic_print_uint"));
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

    /// `INPUT X` emits `call_builtin "input_i64"` then stores into `X`.
    #[test]
    fn compiles_input() {
        let m = compile("10 INPUT X\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let call = body.iter().find(|i|
            i.op == "call_builtin"
                && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "input_i64"));
        assert!(call.is_some(), "expected call_builtin input_i64");
        let helper = call.unwrap().srcs.first().and_then(|s| match s {
            Operand::Var(n) => Some(n.as_str()), _ => None,
        });
        assert_eq!(helper, Some("input_i64"));
        let dest = call.unwrap().dest.as_deref().expect("input has destination temp");
        let widened = body.iter().find(|i|
            i.op == "int_to_real"
                && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == dest))
            .and_then(|i| i.dest.as_deref())
            .expect("BA7 scalar INPUT widens the integer host input to f64");
        assert!(body.iter().any(|i| i.op == "mov"
            && i.dest.as_deref() == Some("X")
            && i.type_hint == "f64"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == widened)),
            "expected widened input temp to move into X");
    }

    /// E4-dyn: `INPUT A$` reads a whole line as a *runtime string* via the
    /// `input_str` builtin (`str`-typed), then `mov`s it into the deterministic
    /// string slot so a later `PRINT A$` resolves the same slot.  No literal
    /// folding happens — the value is unknown until run time.
    #[test]
    fn compiles_input_string() {
        let m = compile("10 INPUT A$\n20 PRINT A$\n30 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        // The builtin call returns a `str` and names `input_str` (not `input_i64`).
        let call = body.iter().find(|i| i.op == "call_builtin"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "input_str"))
            .expect("expected call_builtin input_str");
        assert_eq!(call.type_hint, "str", "string INPUT reads a str");
        let temp = call.dest.as_deref().expect("input_str has a destination temp");
        // The runtime string moves into the `$`-variable's string slot at `str`.
        let slot = basic_string_slot("A$");
        assert!(body.iter().any(|i| i.op == "mov"
            && i.dest.as_deref() == Some(slot.as_str())
            && i.type_hint == "str"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == temp)),
            "expected input_str temp to move into the string slot");
        // No `$` ever reaches a backend-facing register.
        assert!(body.iter().all(|i| i.dest.as_deref() != Some("A$")),
            "backend-facing registers must not contain `$`");
        // `PRINT A$` reads that same slot through the shared E4 `print_str` op.
        assert!(body.iter().any(|i| i.op == "print_str"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if *n == slot)),
            "expected PRINT A$ to print the runtime string slot");
    }

    /// String literals in PRINT lower through shared E4 string ops.
    #[test]
    fn compiles_print_string() {
        let m = compile("10 PRINT \"HI\"\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "str_const"
            && i.dest.as_deref().is_some()
            && i.type_hint == "str"
            && matches!(i.srcs.first(), Some(Operand::Str(s)) if s == "HI")),
            "expected str_const for PRINT literal");
        let str_dest = body.iter().find(|i| i.op == "str_const")
            .and_then(|i| i.dest.as_deref())
            .expect("string literal has a destination")
            .to_string();
        assert!(body.iter().any(|i| i.op == "print_str"
            && i.type_hint == "void"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == &str_dest)),
            "expected print_str to consume the string temp");
        assert!(body.iter().any(|i| i.op == "call_builtin"
            && matches!(i.srcs.as_slice(), [Operand::Var(name), Operand::Var(_)] if name == "putchar")),
            "PRINT should still emit the trailing newline via putchar");
    }

    #[test]
    fn compiles_string_variable_assignment_and_print() {
        let m = compile("10 LET A$ = \"HI\"\n20 PRINT A$\n30 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "str_const"
            && i.dest.as_deref() == Some("__basic_str_A")
            && i.type_hint == "str"
            && matches!(i.srcs.first(), Some(Operand::Str(s)) if s == "HI")),
            "LET A$ should materialize the string literal directly into a safe slot");
        assert!(body.iter().any(|i| i.op == "print_str"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "__basic_str_A")),
            "PRINT A$ should write the string slot through E4 print_str");
        assert!(body.iter().all(|i| i.dest.as_deref() != Some("A$")),
            "backend-facing registers must not contain `$`");
    }

    #[test]
    fn compiles_string_variable_literal_reassignment() {
        let m = compile("10 LET A$ = \"NO\"\n20 LET A$ = \"OK\"\n30 PRINT A$\n40 END\n")
            .expect("ok");
        let body = &m.functions[0].instructions;
        let assigned: Vec<&str> = body.iter()
            .filter(|i| i.op == "str_const"
                && i.dest.as_deref() == Some("__basic_str_A"))
            .filter_map(|i| match i.srcs.first() {
                Some(Operand::Str(s)) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            assigned,
            vec!["NO", "OK"],
            "each literal assignment should rematerialize the same safe string slot"
        );
        assert!(body.iter().any(|i| i.op == "print_str"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "__basic_str_A")),
            "PRINT A$ should consume the reassigned string slot");
    }

    #[test]
    fn compiles_string_literal_concat_assignment_and_print() {
        let m = compile("10 LET A$ = \"O\" + \"K\"\n20 PRINT A$\n30 END\n")
            .expect("ok");
        let body = &m.functions[0].instructions;
        let literal_slots: Vec<&str> = body.iter()
            .filter(|i| i.op == "str_const")
            .filter_map(|i| i.dest.as_deref())
            .collect();
        assert_eq!(literal_slots.len(), 2, "concat should materialize both literals");
        assert!(body.iter().any(|i| i.op == "str_concat"
            && i.dest.as_deref() == Some("__basic_str_A")
            && matches!(i.srcs.as_slice(), [
                Operand::Var(left),
                Operand::Var(right)
            ] if literal_slots.contains(&left.as_str()) && literal_slots.contains(&right.as_str()))),
            "literal concat should land directly in the safe string slot");
        assert!(body.iter().any(|i| i.op == "print_str"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "__basic_str_A")),
            "PRINT A$ should consume the concatenated string slot");
    }

    #[test]
    fn compiles_string_variable_copy_assignment_and_print() {
        let m = compile("10 LET A$ = \"OK\"\n20 LET B$ = A$\n30 PRINT B$\n40 END\n")
            .expect("ok");
        let body = &m.functions[0].instructions;
        let empty_slot = body.iter()
            .find(|i| i.op == "str_const"
                && matches!(i.srcs.first(), Some(Operand::Str(s)) if s.is_empty()))
            .and_then(|i| i.dest.as_deref())
            .expect("copy should materialize an empty string concat operand");
        assert!(body.iter().any(|i| i.op == "str_concat"
            && i.dest.as_deref() == Some("__basic_str_B")
            && matches!(i.srcs.as_slice(), [
                Operand::Var(left),
                Operand::Var(right)
            ] if left == "__basic_str_A" && right == empty_slot)),
            "B$ = A$ should copy through E4 str_concat with an empty suffix");
        assert!(body.iter().any(|i| i.op == "print_str"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "__basic_str_B")),
            "PRINT B$ should consume the copied string slot");
    }

    #[test]
    fn compiles_string_concat_print_expression() {
        let m = compile("10 LET A$ = \"O\"\n20 PRINT A$ + \"K\"\n30 END\n")
            .expect("ok");
        let body = &m.functions[0].instructions;
        let concat_dest = body.iter()
            .find(|i| i.op == "str_concat"
                && matches!(i.srcs.as_slice(), [
                    Operand::Var(left),
                    Operand::Var(_right)
                ] if left == "__basic_str_A"))
            .and_then(|i| i.dest.as_deref())
            .expect("PRINT A$ + literal should lower through str_concat");
        assert!(body.iter().any(|i| i.op == "print_str"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == concat_dest)),
            "PRINT should consume the temporary string expression result");
    }

    #[test]
    fn compiles_string_variable_concat_print_expression() {
        let m = compile("10 LET A$ = \"O\"\n20 LET B$ = \"K\"\n30 PRINT A$ + B$\n40 END\n")
            .expect("ok");
        let body = &m.functions[0].instructions;
        let concat_dest = body.iter()
            .find(|i| i.op == "str_concat"
                && matches!(i.srcs.as_slice(), [
                    Operand::Var(left),
                    Operand::Var(right)
                ] if left == "__basic_str_A" && right == "__basic_str_B"))
            .and_then(|i| i.dest.as_deref())
            .expect("PRINT A$ + B$ should lower through str_concat");
        assert!(body.iter().any(|i| i.op == "print_str"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == concat_dest)),
            "PRINT should consume the variable-variable concat result directly");
    }

    #[test]
    fn compiles_string_concat_if_expression_equality() {
        let src = "10 LET A$ = \"O\"\n\
                   20 IF A$ + \"K\" = \"OK\" THEN 50\n\
                   30 PRINT \"BAD\"\n\
                   40 END\n\
                   50 PRINT \"OK\"\n\
                   60 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        let concat_dest = body.iter()
            .find(|i| i.op == "str_concat"
                && matches!(i.srcs.as_slice(), [
                    Operand::Var(left),
                    Operand::Var(_right)
                ] if left == "__basic_str_A"))
            .and_then(|i| i.dest.as_deref())
            .expect("IF A$ + literal should lower through str_concat");
        assert!(body.iter().any(|i| i.op == "str_eq"
            && matches!(i.srcs.as_slice(), [
                Operand::Var(left),
                Operand::Var(_right)
            ] if left == concat_dest)),
            "string expression IF should compare the temporary concat result");
        assert!(body.iter().any(|i| i.op == "jmp_if_true"),
            "string expression equality should branch on true");
    }

    #[test]
    fn compiles_string_variable_concat_if_expression_equality() {
        let src = "10 LET A$ = \"O\"\n\
                   20 LET B$ = \"K\"\n\
                   30 IF A$ + B$ = \"OK\" THEN 60\n\
                   40 PRINT \"BAD\"\n\
                   50 END\n\
                   60 PRINT \"OK\"\n\
                   70 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        let concat_dest = body.iter()
            .find(|i| i.op == "str_concat"
                && matches!(i.srcs.as_slice(), [
                    Operand::Var(left),
                    Operand::Var(right)
                ] if left == "__basic_str_A" && right == "__basic_str_B"))
            .and_then(|i| i.dest.as_deref())
            .expect("IF A$ + B$ should lower through str_concat");
        assert!(body.iter().any(|i| i.op == "str_eq"
            && matches!(i.srcs.as_slice(), [
                Operand::Var(left),
                Operand::Var(_right)
            ] if left == concat_dest)),
            "string expression equality should compare the concat temp");
        assert!(body.iter().any(|i| i.op == "jmp_if_true"),
            "string expression equality should branch when str_eq is true");
    }

    #[test]
    fn compiles_string_variable_concat_if_expression_inequality() {
        let src = "10 LET A$ = \"O\"\n\
                   20 LET B$ = \"K\"\n\
                   30 IF A$ + B$ <> \"NO\" THEN 60\n\
                   40 PRINT \"BAD\"\n\
                   50 END\n\
                   60 PRINT \"OK\"\n\
                   70 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        let concat_dest = body.iter()
            .find(|i| i.op == "str_concat"
                && matches!(i.srcs.as_slice(), [
                    Operand::Var(left),
                    Operand::Var(right)
                ] if left == "__basic_str_A" && right == "__basic_str_B"))
            .and_then(|i| i.dest.as_deref())
            .expect("IF A$ + B$ should lower through str_concat");
        assert!(body.iter().any(|i| i.op == "str_eq"
            && matches!(i.srcs.as_slice(), [
                Operand::Var(left),
                Operand::Var(_right)
            ] if left == concat_dest)),
            "string expression inequality should compare the concat temp");
        assert!(body.iter().any(|i| i.op == "jmp_if_false"),
            "string expression inequality should branch when str_eq is false");
    }

    #[test]
    fn compiles_string_variable_concat_assignment_and_print() {
        let m = compile("10 LET A$ = \"O\"\n20 LET B$ = A$ + \"K\"\n30 PRINT B$\n40 END\n")
            .expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "str_concat"
            && i.dest.as_deref() == Some("__basic_str_B")
            && matches!(i.srcs.as_slice(), [
                Operand::Var(left),
                Operand::Var(_right)
            ] if left == "__basic_str_A")),
            "B$ = A$ + literal should store the concat directly in B$");
        assert!(body.iter().any(|i| i.op == "print_str"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "__basic_str_B")),
            "PRINT B$ should consume the assigned concat result");
    }

    #[test]
    fn compiles_chained_string_variable_concat_assignment_and_print() {
        let m = compile("10 LET A$ = \"A\"\n20 LET B$ = A$ + \"B\" + \"C\"\n30 PRINT B$\n40 END\n")
            .expect("ok");
        let body = &m.functions[0].instructions;
        let concat_positions: Vec<usize> = body.iter()
            .enumerate()
            .filter_map(|(idx, i)| (i.op == "str_concat").then_some(idx))
            .collect();
        assert_eq!(
            concat_positions.len(),
            2,
            "three string operands should lower to two str_concat ops"
        );
        let first_dest = body[concat_positions[0]]
            .dest
            .as_deref()
            .expect("first concat has a temp destination");
        assert!(matches!(body[concat_positions[0]].srcs.as_slice(), [
            Operand::Var(left),
            Operand::Var(_right)
        ] if left == "__basic_str_A"));
        assert!(matches!(body[concat_positions[1]].srcs.as_slice(), [
            Operand::Var(left),
            Operand::Var(_right)
        ] if left == first_dest));
        assert_eq!(
            body[concat_positions[1]].dest.as_deref(),
            Some("__basic_str_B"),
            "the final concat in a target assignment should land directly in B$"
        );
        assert!(body.iter().any(|i| i.op == "print_str"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "__basic_str_B")),
            "PRINT B$ should consume the chained concat result");
    }

    #[test]
    fn compiles_multi_item_string_print_with_semicolon() {
        let m = compile("10 LET A$ = \"O\"\n20 LET B$ = \"K\"\n30 PRINT A$; B$\n40 END\n")
            .expect("ok");
        let body = &m.functions[0].instructions;
        let print_a = body.iter()
            .position(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == "__basic_str_A")
            })
            .expect("PRINT should emit print_str for A$");
        let print_b = body.iter()
            .position(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == "__basic_str_B")
            })
            .expect("PRINT should emit print_str for B$");
        assert!(print_a < print_b, "PRINT A$; B$ should preserve item order");
        assert!(
            !body.iter().any(|i| i.op == "call"
                && i.srcs.first().and_then(|o| o.as_str_lit()) == Some("__basic_print_real")),
            "string-only PRINT should not call numeric formatting helpers"
        );
    }

    #[test]
    fn compiles_multi_item_string_print_with_comma_separator() {
        let m = compile("10 LET A$ = \"O\"\n20 LET B$ = \"K\"\n30 PRINT A$, B$\n40 END\n")
            .expect("ok");
        let body = &m.functions[0].instructions;
        let print_a = body.iter()
            .position(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == "__basic_str_A")
            })
            .expect("PRINT should emit print_str for A$");
        let print_b = body.iter()
            .position(|i| {
                i.op == "print_str"
                    && matches!(i.srcs.first(), Some(Operand::Var(slot)) if slot == "__basic_str_B")
            })
            .expect("PRINT should emit print_str for B$");
        let space_call = body.iter()
            .position(|i| {
                i.op == "call_builtin"
                    && matches!(i.srcs.as_slice(), [Operand::Var(name), Operand::Var(arg)]
                        if name == "putchar" && body.iter().any(|c|
                            c.dest.as_deref() == Some(arg.as_str())
                                && matches!(c.srcs.first(), Some(Operand::Int(32)))))
            })
            .expect("comma separator should emit a single-space putchar");
        assert!(
            print_a < space_call && space_call < print_b,
            "PRINT A$, B$ should emit the comma separator between string items"
        );
    }

    #[test]
    fn compiles_string_variable_if_equality() {
        let src = "10 LET A$ = \"Y\"\n\
                   20 IF A$ = \"Y\" THEN 40\n\
                   30 PRINT \"NO\"\n\
                   40 PRINT A$\n\
                   50 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "str_eq"
            && matches!(i.srcs.as_slice(), [
                Operand::Var(lhs),
                Operand::Var(_rhs)
            ] if lhs == "__basic_str_A")),
            "IF A$ = literal should lower to E4 str_eq");
        assert!(body.iter().any(|i| i.op == "jmp_if_true"),
            "string equality should feed the existing BASIC branch lowering");
    }

    #[test]
    fn compiles_string_variable_if_copied_slot_equality() {
        let src = "10 LET A$ = \"OK\"\n\
                   20 LET B$ = A$\n\
                   30 IF B$ = A$ THEN 60\n\
                   40 PRINT \"BAD\"\n\
                   50 END\n\
                   60 PRINT \"OK\"\n\
                   70 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "str_eq"
            && matches!(i.srcs.as_slice(), [
                Operand::Var(lhs),
                Operand::Var(rhs)
            ] if lhs == "__basic_str_B" && rhs == "__basic_str_A")),
            "IF B$ = A$ should compare two scalar string slots");
        assert!(body.iter().any(|i| i.op == "jmp_if_true"),
            "copied string slot equality should branch on true");
    }

    #[test]
    fn compiles_string_variable_if_inequality() {
        let src = "10 LET A$ = \"N\"\n\
                   20 IF A$ <> \"Y\" THEN 40\n\
                   30 PRINT \"BAD\"\n\
                   40 PRINT \"OK\"\n\
                   50 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "str_eq"
            && matches!(i.srcs.as_slice(), [
                Operand::Var(lhs),
                Operand::Var(_rhs)
            ] if lhs == "__basic_str_A")),
            "IF A$ <> literal should reuse E4 str_eq");
        assert!(body.iter().any(|i| i.op == "jmp_if_false"),
            "string inequality should branch when str_eq is false");
    }

    #[test]
    fn compiles_string_variable_if_ordering() {
        let src = "10 LET A$ = \"ALPHA\"\n\
                   20 IF A$ < \"BETA\" THEN 50\n\
                   30 PRINT \"BAD\"\n\
                   40 END\n\
                   50 IF \"BETA\" > A$ THEN 80\n\
                   60 PRINT \"BAD\"\n\
                   70 END\n\
                   80 PRINT \"OK\"\n\
                   90 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        assert_eq!(
            body.iter().filter(|i| i.op == "str_cmp").count(),
            2,
            "strict string ordering should lower through E4 str_cmp twice"
        );
        assert!(body.iter().any(|i| i.op == "cmp_lt"
            && i.type_hint == "i64"
            && matches!(i.srcs.as_slice(), [
                Operand::Var(ordering),
                Operand::Var(zero)
            ] if body.iter().any(|j| j.dest.as_deref() == Some(ordering.as_str()) && j.op == "str_cmp")
                && body.iter().any(|j| j.dest.as_deref() == Some(zero.as_str())
                    && j.op == "const"
                    && matches!(j.srcs.first(), Some(Operand::Int(0)))))),
            "A$ < literal should compare str_cmp output with zero");
        assert!(body.iter().any(|i| i.op == "cmp_gt" && i.type_hint == "i64"),
            "literal > A$ should use the existing numeric cmp_gt over str_cmp");
        assert!(
            body.iter().filter(|i| i.op == "jmp_if_true").count() >= 2,
            "strict string ordering should branch when the ordering predicate is true"
        );
    }

    #[test]
    fn compiles_string_variable_if_inclusive_ordering() {
        let src = "10 LET A$ = \"BETA\"\n\
                   20 IF A$ <= \"BETA\" THEN 50\n\
                   30 PRINT \"BAD\"\n\
                   40 END\n\
                   50 IF \"BETA\" >= A$ THEN 80\n\
                   60 PRINT \"BAD\"\n\
                   70 END\n\
                   80 PRINT \"OK\"\n\
                   90 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        assert_eq!(
            body.iter().filter(|i| i.op == "str_cmp").count(),
            2,
            "inclusive string ordering should lower through E4 str_cmp twice"
        );
        assert!(body.iter().any(|i| i.op == "cmp_le" && i.type_hint == "i64"),
            "A$ <= literal should compare str_cmp output with zero");
        assert!(body.iter().any(|i| i.op == "cmp_ge" && i.type_hint == "i64"),
            "literal >= A$ should compare str_cmp output with zero");
    }

    #[test]
    fn rejects_string_variable_numeric_expression() {
        let err = compile("10 LET A$ = \"HI\"\n20 PRINT A$ + 1\n30 END\n")
            .unwrap_err();
        match err {
            CompileError::Unsupported(msg) => assert!(msg.contains("mixed string/numeric")),
            other => panic!("expected Unsupported(mixed string/numeric...), got {other:?}"),
        }
    }

    // ── GOSUB / RETURN — unstructured subroutines (BA1, enabler E7) ──────

    /// `GOSUB 100` materialises the return stack, pushes the call-site id,
    /// jumps to `line_100`, and drops a `gosub_ret_0` resume label.
    #[test]
    fn compiles_gosub_pushes_and_jumps() {
        let m = compile("10 GOSUB 100\n20 END\n100 RETURN\n").expect("ok");
        let body = &m.functions[0].instructions;
        let ops: Vec<&str> = body.iter().map(|i| i.op.as_str()).collect();
        // Stack materialised + a push (array_set) + the pointer bump (add).
        assert!(ops.contains(&"alloc_array"), "GOSUB needs a return stack: {ops:?}");
        assert!(ops.contains(&"array_set"), "GOSUB must push a return id");
        // Jump into the subroutine and a resume label after it.
        let labels: Vec<&str> = body.iter().filter(|i| i.op == "label")
            .filter_map(|i| match i.srcs.first() {
                Some(Operand::Var(n)) => Some(n.as_str()), _ => None }).collect();
        assert!(labels.contains(&"gosub_ret_0"),
            "missing gosub_ret_0 resume label in {labels:?}");
        let jmps: Vec<&str> = body.iter().filter(|i| i.op == "jmp")
            .filter_map(|i| match i.srcs.first() {
                Some(Operand::Var(n)) => Some(n.as_str()), _ => None }).collect();
        assert!(jmps.contains(&"line_100"), "GOSUB must jmp to line_100: {jmps:?}");
    }

    /// `RETURN` pops the id and computed-`goto`s over every GOSUB site
    /// (`cmp_eq` + `jmp_if_true gosub_ret_<id>`).
    #[test]
    fn compiles_return_dispatches_over_sites() {
        // Two GOSUB sites ⇒ the RETURN chain has two `cmp_eq`/`jmp_if_true` arms.
        let m = compile(
            "10 GOSUB 100\n20 GOSUB 100\n30 END\n100 RETURN\n").expect("ok");
        let body = &m.functions[0].instructions;
        // RETURN dispatch: a jmp_if_true to each gosub_ret_<id>.
        let targets: Vec<String> = body.iter()
            .filter(|i| i.op == "jmp_if_true")
            .filter_map(|i| match i.srcs.get(1) {
                Some(Operand::Var(n)) => Some(n.clone()), _ => None }).collect();
        assert!(targets.iter().any(|t| t == "gosub_ret_0"));
        assert!(targets.iter().any(|t| t == "gosub_ret_1"),
            "RETURN must dispatch to both sites; got {targets:?}");
        // The pop is an array_get off the stack.
        assert!(body.iter().any(|i| i.op == "array_get"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == BASIC_GOSUB_STACK)),
            "RETURN must array_get the popped id");
    }

    /// A program with no `GOSUB` carries no return stack (lazy materialisation),
    /// and a bare `RETURN` is a clean error rather than a miscompile.
    #[test]
    fn gosub_stack_is_lazy_and_bare_return_errors() {
        let m = compile("10 LET A = 1\n20 END\n").expect("ok");
        let ops: Vec<&str> = m.functions[0].instructions.iter()
            .map(|i| i.op.as_str()).collect();
        // No GOSUB ⇒ no gosub stack array allocated for it. (DATA-less program
        // has no alloc_array at all.)
        assert!(!ops.contains(&"alloc_array"),
            "no GOSUB/DATA ⇒ no array allocation: {ops:?}");

        let err = compile("10 RETURN\n20 END\n").unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)),
            "bare RETURN (no GOSUB) must be a clean error, got {err:?}");
    }

    // ── DEF FN — user-defined single-line functions (BA5) ────────────

    /// `DEF FNS(X) = X * X` lowers to a sibling `IIRFunction` named `fns`
    /// (one `f64` parameter `X`, body `mul X X` then `ret`), pushed after
    /// `main`.  The `DEF` line itself emits nothing runtime into `main`.
    #[test]
    fn compiles_def_fn_into_sibling_function() {
        let m = compile("10 DEF FNS(X) = X * X\n20 PRINT FNS(7)\n30 END\n")
            .expect("ok");
        // `main` is first, with the `FNS` sibling present after it. (The module
        // also carries the two BA2 print helpers because `PRINT` renders a
        // value, so the total count is not asserted here — see the FNS lookup.)
        assert_eq!(m.functions[0].name, "main");
        let f = m.functions.iter().find(|f| f.name == "FNS")
            .expect("sibling function `FNS`");
        assert_eq!(f.return_type, "f64");
        assert_eq!(f.params, vec![("X".to_string(), "f64".to_string())]);
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
        // Should have at least one `add` (the LET) and one `call` to the BA7
        // print helper (scalar PRINT now lowers to `call __basic_print_real`).
        assert!(body.iter().any(|i| i.op == "add"));
        assert!(calls_named(body, "__basic_print_real"));
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

    /// A fractional `DATA` value is accepted into the real-valued pool.
    #[test]
    fn fractional_data_lowers_to_real_pool() {
        let m = compile("10 DATA 3.5\n20 READ X\n30 PRINT X\n40 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i|
            i.op == "alloc_array"
                && i.dest.as_deref() == Some("__basic_data")
                && i.type_hint == "array<f64>"),
            "DATA must materialise an f64 pool");
        assert!(body.iter().any(|i|
            i.op == "const" && i.type_hint == "f64"
                && matches!(i.srcs.first(), Some(Operand::Float(v)) if (*v - 3.5).abs() < f64::EPSILON)),
            "fractional DATA literal should remain f64");
        assert!(body.iter().any(|i|
            i.op == "array_get"
                && i.type_hint == "f64"
                && var_name(i.srcs.first()) == Some("__basic_data")),
            "READ should fetch f64 DATA values");
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
        assert_eq!(alloc.type_hint, "array<f64>");
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

    /// `DIM A(2,3)` (BA-DIM-2D) lowers to a single flat `alloc_array` whose
    /// length is the product of the per-dimension inclusive sizes:
    /// `(2+1) * (3+1) = 12`.
    #[test]
    fn compiles_2d_dim_to_alloc_array() {
        let m = compile("10 DIM A(2,3)\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let alloc = body.iter().find(|i| i.op == "alloc_array")
            .expect("2-D DIM produces one alloc_array");
        assert_eq!(alloc.dest.as_deref(), Some("A"));
        let len_reg = var_name(alloc.srcs.first()).expect("alloc_array len reg");
        assert!(body.iter().any(|i|
            i.op == "const" && i.dest.as_deref() == Some(len_reg)
                && matches!(i.srcs.first(), Some(Operand::Int(12)))),
            "expected `const 12` (= 3*4) feeding the 2-D alloc_array length");
    }

    /// `LET A(1,2) = 7` on a `DIM A(2,3)` array folds the two subscripts into the
    /// row-major flat index `i*(N+1) + j = 1*4 + 2 = 6`.  The lowering therefore
    /// emits a `const 4` stride, a `mul` (i*stride), and an `add` (+ j), feeding a
    /// single `array_set`.
    #[test]
    fn compiles_2d_array_write_uses_stride_mul_and_add() {
        let m = compile("10 DIM A(2,3)\n20 LET A(1,2) = 7\n30 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        // The outer stride is size[1] = N+1 = 4 — emitted as an i64 const.
        assert!(body.iter().any(|i|
            i.op == "const" && i.type_hint == "i64"
                && matches!(i.srcs.first(), Some(Operand::Int(4)))),
            "expected an i64 `const 4` for the row stride (N+1)");
        assert!(body.iter().any(|i| i.op == "mul" && i.type_hint == "i64"),
            "flat index needs a `mul` for i*stride");
        assert!(body.iter().any(|i| i.op == "add" && i.type_hint == "i64"),
            "flat index needs an `add` to combine i*stride + j");
        let set = body.iter().find(|i| i.op == "array_set")
            .expect("LET A(i,j)=e produces one array_set");
        assert_eq!(var_name(set.srcs.first()), Some("A"));
        assert_eq!(set.srcs.len(), 3, "array_set takes handle, flat index, value");
    }

    /// Reading `A(1,2)` from a 2-D array produces a single `array_get` at the
    /// flat index.
    #[test]
    fn compiles_2d_array_read_to_array_get() {
        let m = compile("10 DIM A(2,3)\n20 LET X = A(1,2)\n30 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let get = body.iter().find(|i| i.op == "array_get" && var_name(i.srcs.first()) == Some("A"))
            .expect("reading A(i,j) produces an array_get on A");
        assert_eq!(get.type_hint, "f64");
        assert!(get.dest.is_some());
    }

    /// Giving the wrong number of subscripts for a DIMmed array is a clean
    /// `Unsupported` error (dimension mismatch), not a miscompile.
    #[test]
    fn wrong_subscript_count_is_unsupported() {
        // A(1) has one subscript but A was DIMmed 2-D.
        let err = compile("10 DIM A(2,3)\n20 LET A(1) = 7\n30 END\n").unwrap_err();
        match err {
            CompileError::Unsupported(msg) =>
                assert!(msg.contains("dimension"),
                    "expected a dimension-mismatch message, got {msg:?}"),
            other => panic!("expected Unsupported(dimension…), got {other:?}"),
        }
    }

    /// A 3-D `DIM A(1,1,1)` also works — the strides generalise: sizes
    /// `(2,2,2)`, total `8`.
    #[test]
    fn compiles_3d_dim_to_alloc_array() {
        let m = compile("10 DIM A(1,1,1)\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let alloc = body.iter().find(|i| i.op == "alloc_array")
            .expect("3-D DIM produces one alloc_array");
        let len_reg = var_name(alloc.srcs.first()).expect("alloc_array len reg");
        assert!(body.iter().any(|i|
            i.op == "const" && i.dest.as_deref() == Some(len_reg)
                && matches!(i.srcs.first(), Some(Operand::Int(8)))),
            "expected `const 8` (= 2*2*2) for the 3-D alloc_array length");
    }

    /// `LET A(2) = 7` lowers to `array_set A, idx, val` with the subscript
    /// used **directly** as the 0-based index (no lower-bound subtraction).
    #[test]
    fn compiles_array_assignment_to_array_set() {
        let m = compile("10 DIM A(5)\n20 LET A(2) = 7\n30 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let set = body.iter().find(|i| i.op == "array_set")
            .expect("LET A(i)=e produces an array_set");
        assert_eq!(set.type_hint, "f64");
        assert_eq!(var_name(set.srcs.first()), Some("A"));
        assert_eq!(set.srcs.len(), 3, "array_set takes handle, index, value");
        // BA7 array elements are f64; only the subscript truncates to i64.
        let idx_reg = var_name(set.srcs.get(1)).expect("index reg");
        assert!(body.iter().any(|i|
            i.op == "real_to_int_trunc" && i.dest.as_deref() == Some(idx_reg)),
            "array index should be an explicit real_to_int_trunc result");
        assert!(body.iter().any(|i|
            i.op == "const" && i.type_hint == "f64"
                && matches!(i.srcs.first(), Some(Operand::Float(v)) if (*v - 2.0).abs() < f64::EPSILON)),
            "source subscript literal should be a scalar f64 const");
        assert!(body.iter().any(|i|
            i.op == "const" && i.type_hint == "f64"
                && matches!(i.srcs.first(), Some(Operand::Float(v)) if (*v - 7.0).abs() < f64::EPSILON)),
            "array value literal should stay f64");
    }

    /// `LET X = A(2)` reads an element → `array_get A, idx → dest`.
    #[test]
    fn compiles_array_read_to_array_get() {
        let m = compile("10 DIM A(5)\n20 LET X = A(2)\n30 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let get = body.iter().find(|i| i.op == "array_get")
            .expect("reading A(i) produces an array_get");
        assert_eq!(get.type_hint, "f64");
        assert_eq!(var_name(get.srcs.first()), Some("A"));
        assert!(get.dest.is_some(), "array_get writes a destination register");
    }

    // -- E4d-BA-arr: BASIC string arrays (`DIM A$(n)`) -----------------------
    // A `$`-named array holds E4-dyn runtime string handles: it reuses the E5
    // aggregate substrate with a `str` element type (`array<str>`) rather than
    // the numeric `array<f64>`.  The handle register is sanitised ($-free) so it
    // is a portable IIR name and coexists with a numeric array of the same stem.

    /// `DIM A$(2)` allocates an `array<str>` of 3 handles (0-based inclusive),
    /// under the `$`-free handle register `__basic_strarr_A`.
    #[test]
    fn compiles_string_dim_to_alloc_array_str() {
        let m = compile("10 DIM A$(2)\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let alloc = body.iter().find(|i| i.op == "alloc_array")
            .expect("DIM A$(n) produces one alloc_array");
        assert_eq!(alloc.type_hint, "array<str>",
            "a $-named array holds E4-dyn string handles");
        assert_eq!(alloc.dest.as_deref(), Some("__basic_strarr_A"),
            "the string-array handle register is $-free");
        let len_reg = var_name(alloc.srcs.first()).expect("alloc_array len reg");
        assert!(body.iter().any(|i|
            i.op == "const" && i.dest.as_deref() == Some(len_reg)
                && matches!(i.srcs.first(), Some(Operand::Int(3)))),
            "DIM A$(2) allocates 3 elements (A$(0)..A$(2))");
    }

    /// `LET A$(0) = "HI"` stores a runtime `str` handle into the element via a
    /// `str`-typed `array_set` (handle, flat index, value).
    #[test]
    fn compiles_string_array_assignment_to_array_set_str() {
        let m = compile("10 DIM A$(2)\n20 LET A$(0) = \"HI\"\n30 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let set = body.iter().find(|i| i.op == "array_set")
            .expect("LET A$(i)=s produces an array_set");
        assert_eq!(set.type_hint, "str");
        assert_eq!(var_name(set.srcs.first()), Some("__basic_strarr_A"));
        assert_eq!(set.srcs.len(), 3, "array_set takes handle, index, value");
        let val_reg = var_name(set.srcs.get(2)).expect("value reg");
        assert!(body.iter().any(|i|
            i.op == "str_const" && i.dest.as_deref() == Some(val_reg)),
            "the stored value is a runtime str handle from str_const");
    }

    /// `PRINT A$(1)` reads the element with a `str`-typed `array_get` and prints
    /// it through the shared E4 `print_str` op.
    #[test]
    fn compiles_string_array_read_to_array_get_str() {
        let m = compile("10 DIM A$(2)\n20 LET A$(1) = \"HI\"\n30 PRINT A$(1)\n40 END\n")
            .expect("ok");
        let body = &m.functions[0].instructions;
        let get = body.iter().find(|i| i.op == "array_get"
            && var_name(i.srcs.first()) == Some("__basic_strarr_A"))
            .expect("reading A$(i) produces an array_get on the string handle");
        assert_eq!(get.type_hint, "str");
        let dest = get.dest.as_deref().expect("array_get writes a dest");
        assert!(body.iter().any(|i|
            i.op == "print_str" && var_name(i.srcs.first()) == Some(dest)),
            "PRINT A$(i) feeds the array_get result into print_str");
    }

    /// A string-array element can feed `+` concatenation — proving the read is a
    /// genuine runtime string handle (two `str` array_gets → one `str_concat`),
    /// not a folded literal.
    #[test]
    fn string_array_element_feeds_concat() {
        let m = compile(
            "10 DIM A$(2)\n20 LET A$(0)=\"O\"\n30 LET A$(1)=\"K\"\n\
             40 LET B$ = A$(0) + A$(1)\n50 PRINT B$\n60 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let gets = body.iter()
            .filter(|i| i.op == "array_get" && i.type_hint == "str").count();
        assert!(gets >= 2, "A$(0)+A$(1) reads two string elements, got {gets}");
        assert!(body.iter().any(|i| i.op == "str_concat"),
            "the two element reads feed a str_concat");
    }

    /// Reading a string array in a numeric expression is a clean type error, not
    /// a miscompile into an `f64` array_get.
    #[test]
    fn string_array_in_numeric_context_is_unsupported() {
        let err = compile("10 DIM A$(2)\n20 LET X = A$(0)\n30 END\n").unwrap_err();
        match err {
            CompileError::Unsupported(msg) =>
                assert!(msg.contains("numeric expression"),
                    "expected a numeric-context error, got {msg:?}"),
            other => panic!("expected Unsupported(numeric…), got {other:?}"),
        }
    }

    /// A numeric RHS assigned to a string-array element is a clean type error
    /// (no silent coercion).
    #[test]
    fn numeric_rhs_to_string_array_is_unsupported() {
        let err = compile("10 DIM A$(2)\n20 LET A$(0) = 5\n30 END\n").unwrap_err();
        match err {
            CompileError::Unsupported(msg) =>
                assert!(msg.contains("string RHS"),
                    "expected a string-RHS error, got {msg:?}"),
            other => panic!("expected Unsupported(string RHS…), got {other:?}"),
        }
    }

    /// A numeric array `A` and a string array `A$` coexist as distinct
    /// variables with distinct, non-colliding handle registers.
    #[test]
    fn numeric_and_string_arrays_coexist() {
        let m = compile(
            "10 DIM A(2)\n20 DIM A$(2)\n30 LET A(0)=7\n40 LET A$(0)=\"HI\"\n50 END\n")
            .expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "alloc_array"
            && i.dest.as_deref() == Some("A") && i.type_hint == "array<f64>"),
            "numeric array A keeps its bare f64 handle");
        assert!(body.iter().any(|i| i.op == "alloc_array"
            && i.dest.as_deref() == Some("__basic_strarr_A") && i.type_hint == "array<str>"),
            "string array A$ gets a distinct str handle");
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
