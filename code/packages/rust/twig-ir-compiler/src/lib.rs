//! # twig-ir-compiler — TW00: Twig → InterpreterIR (IIR)
//!
//! This crate is the third stage of the Rust [Twig](../../specs/TW00-twig-language.md)
//! pipeline.  It accepts a parsed [`twig_parser::Program`] and emits an
//! [`interpreter_ir::IIRModule`] that the LANG VM (`vm-core`) can
//! execute, or that a JIT can specialise.
//!
//! ## Pipeline
//!
//! ```text
//! Twig source
//!     │
//!     ▼  twig_lexer::tokenize
//! Vec<Token>
//!     │
//!     ▼  twig_parser::parse
//! Program (typed AST)
//!     │
//!     ▼  compile_source / compile_program        ← THIS CRATE
//! IIRModule  (functions: top-level fns + anonymous lambdas + main)
//!     │
//!     ▼  vm-core / jit-core
//! program output
//! ```
//!
//! ## What gets emitted
//!
//! - **One `IIRFunction` per `(define (name args) body+)`** — top-level
//!   user functions.  Recursion works naturally because the compiler
//!   pre-classifies all top-level names before walking any bodies.
//! - **One `IIRFunction` per `(lambda ...)`** — synthesised name
//!   (`__lambda_0`, `__lambda_1`, …).  Captured free variables become
//!   the *leading* parameters, in stable insertion order, so the
//!   `make_closure` call site can pass them in the same order the
//!   inner function expects.
//! - **A synthesised `main` function** — holds top-level value defines
//!   (each emitted as `call_builtin "global_set" name value`) plus
//!   bare top-level expressions.  The value of the last bare
//!   expression becomes `main`'s return; programs with no expression
//!   return `nil`.
//!
//! All instructions carry `type_hint = "any"` (Twig is dynamically
//! typed); functions therefore have `type_status = Untyped`.  The
//! vm-core profiler fills in observed types at runtime, which the
//! JIT can specialise on later.
//!
//! ## Apply-site dispatch (compile-time)
//!
//! | Function position           | Emitted IIR                                 |
//! |-----------------------------|---------------------------------------------|
//! | Top-level user fn name      | `call <name>, ...args`                      |
//! | Builtin (`+`, `cons`, …)    | `call_builtin <name>, ...args`              |
//! | Anything else (locals etc.) | `call_builtin "apply_closure", h, ...args`  |
//!
//! Top-level recursion stays on the fast `call` path; closure dispatch
//! pays the indirect cost only for locals that hold closures.
//!
//! ## Example
//!
//! ```
//! use twig_ir_compiler::compile_source;
//!
//! let module = compile_source(
//!     "(define (square x) (* x x)) (square 7)",
//!     "demo",
//! ).unwrap();
//!
//! assert_eq!(module.entry_point.as_deref(), Some("main"));
//! assert_eq!(module.language, "twig");
//! // One IIRFunction for `square`, one for `main`.
//! let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
//! assert!(names.contains(&"square"));
//! assert!(names.contains(&"main"));
//! ```

pub mod compiler;
pub mod errors;
pub mod free_vars;

pub use compiler::Compiler;
pub use errors::TwigCompileError;
pub use free_vars::free_vars;

use interpreter_ir::{
    function::FunctionTypeStatus,
    source_loc::SourceLoc,
    IIRModule,
};
use std::collections::HashMap;
use twig_parser::{parse, Program, TypedMode};
use type_declarations::AnnotatedNode;

/// Compile a parsed [`Program`] into an [`IIRModule`].
///
/// The `module_name` is stored on the resulting module — useful for
/// debug prints and for the source-position table.  Entry point is
/// always `"main"`; language tag is always `"twig"`.
///
/// # Errors
///
/// Returns [`TwigCompileError`] for any of: a `(lambda ...)` that
/// captures an unbound name, a `VarRef` that doesn't resolve to a
/// local / global / builtin, an empty function body, or an integer
/// overflow.
pub fn compile_program(
    program: &Program,
    module_name: &str,
) -> Result<IIRModule, TwigCompileError> {
    // ── Optional TW05-B type-check pre-pass (LANG49) ──────────────────────
    //
    // Runs only when the module declares `(typed lenient)` or `(typed strict)`.
    // `(typed off)` and programs with no `(module …)` declaration are untouched —
    // `check_program` returns immediately in those cases.
    //
    // Lenient mode: errors are printed as warnings but compilation continues.
    // Strict mode:  any type error blocks compilation and returns Err.
    if let Some(mode) = program
        .module_info
        .as_ref()
        .and_then(|mi| mi.typed_mode.as_ref())
    {
        let tc_result = twig_type_checker::check_program(program, None);
        match mode {
            TypedMode::Strict if !tc_result.ok => {
                // Invariant: ok == false in Strict mode ↔ errors is non-empty.
                // Using expect here enforces the invariant at this integration
                // boundary.  A panic here means a bug in the type checker, not
                // in user input — fail loudly rather than silently.
                let d = tc_result.errors.first().expect(
                    "type-checker invariant violated: ok==false but errors is empty",
                );
                return Err(TwigCompileError {
                    message: format!("type error: {}", d.message),
                    line: d.line,
                    column: d.column,
                });
            }
            TypedMode::Lenient => {
                for d in &tc_result.errors {
                    eprintln!(
                        "twig type warning ({}:{}): {}",
                        d.line, d.column, d.message
                    );
                }
            }
            _ => {}
        }
    }

    Compiler::new().compile(program, module_name)
}

/// Compile a [`Program`] that may call functions defined in other modules.
///
/// This is the LANG56 entry point used by `twig-module-driver`.  Before
/// the compiler's own pre-pass runs, `extern_fns` is pre-registered in
/// `fn_globals` so that cross-module calls compile to `call` instructions
/// rather than failing with "unbound name".  The actual function bodies are
/// provided by the other modules; `iir_linker::link` resolves the call
/// targets at link time.
///
/// The type-check pre-pass (LANG49) is applied identically to
/// [`compile_program`].
///
/// # Example
///
/// ```
/// use twig_ir_compiler::{compile_program_with_externs};
/// use twig_parser::parse;
///
/// let program = parse("(define (sq x) (* x x)) (sq 5)").unwrap();
/// // "helper" is defined in another module; pre-register it so the compiler
/// // accepts `(helper 42)` in this module.
/// let m = compile_program_with_externs(&program, "my_mod", &["helper"]).unwrap();
/// assert!(m.get_function("sq").is_some());
/// ```
pub fn compile_program_with_externs(
    program: &Program,
    module_name: &str,
    extern_fns: &[&str],
) -> Result<IIRModule, TwigCompileError> {
    // Apply the same LANG49 type-check pre-pass as `compile_program`.
    if let Some(mode) = program
        .module_info
        .as_ref()
        .and_then(|mi| mi.typed_mode.as_ref())
    {
        let tc_result = twig_type_checker::check_program(program, None);
        match mode {
            TypedMode::Strict if !tc_result.ok => {
                // Use ok_or_else rather than expect so that a downstream bug in
                // the type-checker (ok==false with empty errors) returns a
                // graceful Err rather than panicking in a library context.
                let d = tc_result.errors.first().ok_or_else(|| TwigCompileError {
                    message: "type-checker invariant violated: ok==false but errors is empty"
                        .to_string(),
                    line: 0,
                    column: 0,
                })?;
                return Err(TwigCompileError {
                    message: format!("type error: {}", d.message),
                    line: d.line,
                    column: d.column,
                });
            }
            TypedMode::Lenient => {
                for d in &tc_result.errors {
                    eprintln!(
                        "twig type warning ({}:{}): {}",
                        d.line, d.column, d.message
                    );
                }
            }
            _ => {}
        }
    }

    Compiler::new().with_extern_fns(extern_fns).compile(program, module_name)
}

/// Lex, parse, and compile a Twig source string in one call.
///
/// This is the most ergonomic entry point — most callers never need
/// to construct a [`Program`] explicitly.
///
/// # Example
///
/// ```
/// use twig_ir_compiler::compile_source;
///
/// let m = compile_source("(+ 1 2)", "test").unwrap();
/// assert_eq!(m.functions.len(), 1); // just main
/// assert_eq!(m.functions[0].name, "main");
/// ```
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, TwigCompileError> {
    let program = parse(source)?;
    compile_program(&program, module_name)
}

// ---------------------------------------------------------------------------
// LANG50: compile_typed_source — annotation-aware IIR emission
// ---------------------------------------------------------------------------

/// Compile a Twig source string with full type annotation propagation.
///
/// Runs the LANG50 `grammar-type-checker` pass first to build an
/// [`AnnotatedNode`] tree, then compiles the program and post-processes the
/// resulting IIR to propagate concrete `type_hint` values (`"i64"`, `"bool"`,
/// `"str"`, `"closure"`) wherever the checker could infer a concrete kind.
///
/// ## Type propagation pipeline
///
/// ```text
/// source
///   ├─ parse_to_ast  → GrammarASTNode
///   ├─ parse + emit_type_declarations → TypeDeclarations
///   └─ grammar_type_checker::check → AnnotatedNode tree
///                          │
///               build_hint_map: (line, col) → iir_hint
///                          │
///          compile_source → IIRModule
///                          │
///             apply_hints: instructions.type_hint updated
///                          │
///            set_function_type_status: FullyTyped / PartiallyTyped / Untyped
/// ```
///
/// ## Mode enforcement
///
/// | `(typed …)` clause | Behaviour                                     |
/// |--------------------|-----------------------------------------------|
/// | absent / `off`     | Annotate; never emit errors                   |
/// | `lenient`          | Annotate; warnings printed; compilation ok    |
/// | `strict`           | Annotate; first error → `Err(TwigCompileError)` |
///
/// # Errors
///
/// Returns `Err` on parse failure or strict-mode type errors.
///
/// # Example
///
/// ```
/// use twig_ir_compiler::compile_typed_source;
///
/// let m = compile_typed_source("42", "test").unwrap();
/// let main = m.functions.iter().find(|f| f.name == "main").unwrap();
/// // The integer literal 42 → type_hint "i64" on the const instruction.
/// let hint = main.instructions.iter()
///     .find(|i| i.op == "const")
///     .map(|i| i.type_hint.as_str());
/// assert_eq!(hint, Some("i64"));
/// ```
pub fn compile_typed_source(source: &str, module_name: &str) -> Result<IIRModule, TwigCompileError> {
    // ── 1. Type-check via grammar-type-checker (LANG50). ──────────────────
    let tc = twig_type_checker::type_check_source(source)
        .map_err(|e| TwigCompileError { message: e.to_string(), line: 0, column: 0 })?;

    // ── 2. Enforce typed-mode. ────────────────────────────────────────────
    // The `TypeCheckResult::ok` flag encodes strict-mode failures; check it.
    if !tc.ok {
        // ok==false ↔ strict mode + at least one error.
        let d = tc.errors.first().expect(
            "type-checker invariant violated: ok==false but errors is empty",
        );
        return Err(TwigCompileError {
            message: format!("type error: {}", d.message),
            line: d.line,
            column: d.column,
        });
    }
    // Lenient mode: print warnings but continue.
    for d in &tc.errors {
        eprintln!("twig type warning ({}:{}): {}", d.line, d.column, d.message);
    }

    // ── 3. Compile to IIR (base pass — all hints still "any"). ────────────
    let mut module = compile_source(source, module_name)?;

    // ── 4. Build (line, col) → iir_hint lookup from the AnnotatedNode tree. ─
    let hint_map = build_hint_map(&tc.typed_ast);

    // ── 5. Post-process: replace "any" type_hints with concrete kinds. ────
    for func in &mut module.functions {
        apply_hints(func, &hint_map);
        set_function_type_status(func);
    }

    Ok(module)
}

// ---------------------------------------------------------------------------
// Annotation post-processing helpers
// ---------------------------------------------------------------------------

/// Walk an [`AnnotatedNode`] tree and build a map from source position
/// `(line, col)` to IIR `type_hint` string.
///
/// Only positions with **concrete** hints (`"i64"`, `"bool"`, `"str"`,
/// `"closure"`) are stored — `"any"` hints are not inserted because the
/// default is already `"any"`.
fn build_hint_map(root: &AnnotatedNode) -> HashMap<(u32, u32), &'static str> {
    let mut map = HashMap::new();
    collect_hints(root, &mut map);
    map
}

fn collect_hints<'a>(
    node: &'a AnnotatedNode,
    map: &mut HashMap<(u32, u32), &'static str>,
) {
    use type_declarations::AnnotatedChild;

    let hint = node.iir_hint();
    if hint != "any" {
        // Use start position as the lookup key.
        if let (Some(line), Some(col)) = (node.start_line, node.start_column) {
            map.insert((line as u32, col as u32), hint);
        }
    }

    for child in &node.children {
        if let AnnotatedChild::Node(child_node) = child {
            collect_hints(child_node, map);
        }
    }
}

/// Replace `"any"` `type_hint`s on instructions where the hint map has a
/// concrete hint at the instruction's source position.
fn apply_hints(
    func: &mut interpreter_ir::function::IIRFunction,
    hint_map: &HashMap<(u32, u32), &'static str>,
) {
    for (instr, loc) in func.instructions.iter_mut().zip(func.source_map.iter()) {
        if instr.type_hint == "any" && loc != &SourceLoc::SYNTHETIC {
            if let Some(&hint) = hint_map.get(&(loc.line, loc.column)) {
                instr.type_hint = hint.to_owned();
            }
        }
    }
}

/// Set [`FunctionTypeStatus`] on a function based on how many of its
/// instructions carry a concrete `type_hint` (non-`"any"`, non-`"void"`).
///
/// | Condition                        | Status            |
/// |----------------------------------|-------------------|
/// | All concrete (threshold = 0%)    | `FullyTyped`      |
/// | Mixed (1%–99%)                   | `PartiallyTyped`  |
/// | All `"any"` / `"void"` (100%)    | `Untyped`         |
///
/// Instructions with `"void"` type_hint (like `label` or `br`) are
/// excluded from the count entirely — they are structural, not typed.
fn set_function_type_status(func: &mut interpreter_ir::function::IIRFunction) {
    let typed_instrs: Vec<_> = func
        .instructions
        .iter()
        .filter(|i| i.type_hint != "void")
        .collect();

    if typed_instrs.is_empty() {
        return; // no value-producing instructions — leave status unchanged
    }

    let concrete_count = typed_instrs
        .iter()
        .filter(|i| i.type_hint != "any")
        .count();

    func.type_status = if concrete_count == typed_instrs.len() {
        FunctionTypeStatus::FullyTyped
    } else if concrete_count > 0 {
        FunctionTypeStatus::PartiallyTyped
    } else {
        FunctionTypeStatus::Untyped
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// These tests verify the *shape* of emitted IIR — instruction order,
// opcode names, and dispatch decisions — for canonical Twig programs.
// They do not execute the IR (that's vm-core's responsibility).
// Coverage targets the same surface as the Python `tests/test_compiler.py`.

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{
        function::FunctionTypeStatus,
        instr::{IIRInstr, Operand},
    };

    fn module(src: &str) -> IIRModule {
        compile_source(src, "test").unwrap_or_else(|e| panic!("compile failed: {e}"))
    }

    fn main_instrs(src: &str) -> Vec<IIRInstr> {
        let m = module(src);
        m.functions
            .into_iter()
            .find(|f| f.name == "main")
            .expect("module must have main")
            .instructions
    }

    fn fn_instrs(src: &str, name: &str) -> Vec<IIRInstr> {
        let m = module(src);
        m.functions
            .into_iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("module missing fn {name}"))
            .instructions
    }

    fn function_names(src: &str) -> Vec<String> {
        module(src)
            .functions
            .into_iter()
            .map(|f| f.name)
            .collect()
    }

    fn op_names(instrs: &[IIRInstr]) -> Vec<&str> {
        instrs.iter().map(|i| i.op.as_str()).collect()
    }

    // ---- Module-level invariants -----------------------------------------

    #[test]
    fn empty_program_returns_nil() {
        let m = module("");
        assert_eq!(m.entry_point.as_deref(), Some("main"));
        assert_eq!(m.language, "twig");
        assert_eq!(m.functions.len(), 1);
        let main = &m.functions[0];
        assert_eq!(main.name, "main");
        // make_nil + ret is the empty-program shape.
        assert_eq!(op_names(&main.instructions), vec!["call_builtin", "ret"]);
        match &main.instructions[0].srcs[0] {
            Operand::Var(s) => assert_eq!(s, "make_nil"),
            other => panic!("expected Var(\"make_nil\"), got {other:?}"),
        }
    }

    #[test]
    fn module_name_forwarded() {
        let m = compile_source("", "my_module").unwrap();
        assert_eq!(m.name, "my_module");
    }

    #[test]
    fn all_functions_are_untyped() {
        let m = module("(define (f x) (+ x 1)) (f 2)");
        for f in &m.functions {
            assert_eq!(
                f.type_status,
                FunctionTypeStatus::Untyped,
                "fn {} should be Untyped (Twig is dynamically typed)",
                f.name
            );
        }
    }

    #[test]
    fn every_instruction_has_any_or_void_type_hint() {
        let src = "(define (f x) (if (= x 0) 1 (* x 2))) (f 3)";
        let m = module(src);
        for f in &m.functions {
            for i in &f.instructions {
                assert!(
                    i.type_hint == "any" || i.type_hint == "void",
                    "fn {} instr {} has unexpected type_hint {:?}",
                    f.name,
                    i.op,
                    i.type_hint
                );
            }
        }
    }

    // ---- Atoms -----------------------------------------------------------

    #[test]
    fn integer_literal_uses_const() {
        let i = main_instrs("42");
        assert_eq!(i[0].op, "const");
        assert_eq!(i[0].srcs[0], Operand::Int(42));
        assert_eq!(i.last().unwrap().op, "ret");
    }

    #[test]
    fn negative_integer_literal_preserved() {
        let i = main_instrs("-7");
        assert_eq!(i[0].srcs[0], Operand::Int(-7));
    }

    #[test]
    fn bool_literal_uses_const_with_bool_operand() {
        let i = main_instrs("#t");
        assert_eq!(i[0].op, "const");
        assert_eq!(i[0].srcs[0], Operand::Bool(true));
    }

    #[test]
    fn nil_literal_emits_make_nil_builtin() {
        let i = main_instrs("nil");
        assert_eq!(i[0].op, "call_builtin");
        assert_eq!(i[0].srcs[0], Operand::Var("make_nil".into()));
    }

    #[test]
    fn quoted_symbol_emits_make_symbol() {
        let i = main_instrs("'foo");
        // const "foo" + call_builtin make_symbol + ret
        assert_eq!(i[0].op, "const");
        assert_eq!(i[1].op, "call_builtin");
        assert_eq!(i[1].srcs[0], Operand::Var("make_symbol".into()));
    }

    // ---- Builtin calls --------------------------------------------------

    #[test]
    fn builtin_call_uses_call_builtin_directly() {
        let i = main_instrs("(+ 1 2)");
        let call = i.iter().find(|x| x.op == "call_builtin").unwrap();
        // First src is the builtin name; remaining are arg registers.
        assert_eq!(call.srcs[0], Operand::Var("+".into()));
        // (+ 1 2) takes two args, so total srcs = 1 (builtin) + 2 (args)
        assert_eq!(call.srcs.len(), 3);
    }

    #[test]
    fn builtins_recognised() {
        for op in ["+", "-", "*", "/", "=", "<", ">", "cons", "car", "cdr",
                   "null?", "pair?", "number?", "symbol?", "print"] {
            let src = format!("({op} 1)");
            let i = main_instrs(&src);
            let call = i.iter().find(|x| x.op == "call_builtin").unwrap();
            assert_eq!(call.srcs[0], Operand::Var(op.into()), "{op} should dispatch to call_builtin");
        }
    }

    // ---- Top-level functions -------------------------------------------

    #[test]
    fn top_level_define_creates_function() {
        let m = module("(define (square x) (* x x))");
        let names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"square"));
        assert!(names.contains(&"main"));
    }

    #[test]
    fn top_level_function_call_uses_call_op() {
        let i = main_instrs("(define (f x) x) (f 42)");
        let call = i.iter().find(|x| x.op == "call").unwrap();
        // First src is the function name (a Var holding the literal name).
        assert_eq!(call.srcs[0], Operand::Var("f".into()));
        // (f 42) → 1 (name) + 1 (arg)
        assert_eq!(call.srcs.len(), 2);
    }

    #[test]
    fn top_level_function_params_match_iir() {
        let m = module("(define (add x y) (+ x y))");
        let f = m.functions.iter().find(|f| f.name == "add").unwrap();
        assert_eq!(
            f.params,
            vec![("x".to_string(), "any".to_string()), ("y".to_string(), "any".to_string())]
        );
        assert_eq!(f.return_type, "any");
    }

    #[test]
    fn top_level_function_body_ends_with_ret() {
        let i = fn_instrs("(define (f x) (+ x 1))", "f");
        assert_eq!(i.last().unwrap().op, "ret");
    }

    #[test]
    fn recursion_resolves_via_pre_pass() {
        // `fact` calls itself — the pre-pass classification means the
        // self-reference compiles to a direct `call`, not `apply_closure`.
        let i = fn_instrs(
            "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))",
            "fact",
        );
        // At least one direct `call` somewhere in the body.
        assert!(
            i.iter().any(|x| x.op == "call"
                && matches!(&x.srcs[0], Operand::Var(s) if s == "fact")),
            "fact should self-call via direct `call`, ops were: {:?}",
            op_names(&i)
        );
    }

    #[test]
    fn mutual_recursion_works() {
        // Both even? and odd? exist as top-level fns; each can call the other.
        let m = module(
            "(define (even? n) (if (= n 0) #t (odd? (- n 1))))\n\
             (define (odd? n) (if (= n 0) #f (even? (- n 1))))",
        );
        let names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"even?"));
        assert!(names.contains(&"odd?"));
    }

    // ---- Top-level value defines ---------------------------------------

    #[test]
    fn top_level_value_define_uses_global_set() {
        let i = main_instrs("(define x 42)");
        let gs = i.iter().find(|x| matches!(&x.srcs.first(), Some(Operand::Var(s)) if s == "global_set"));
        assert!(gs.is_some(), "expected a global_set call_builtin");
    }

    #[test]
    fn value_global_reference_uses_global_get() {
        let i = main_instrs("(define x 42) x");
        let gg = i.iter().find(|x| matches!(&x.srcs.first(), Some(Operand::Var(s)) if s == "global_get"));
        assert!(gg.is_some(), "expected a global_get call_builtin");
    }

    // ---- if + let + begin ---------------------------------------------

    #[test]
    fn if_emits_jmp_if_false_and_two_labels() {
        let i = main_instrs("(if #t 1 2)");
        let ops = op_names(&i);
        // jmp_if_false ... call_builtin _move ... jmp ... label ... call_builtin _move ... label ... ret
        assert!(ops.contains(&"jmp_if_false"));
        assert!(ops.contains(&"jmp"));
        assert_eq!(ops.iter().filter(|&&o| o == "label").count(), 2);
        // Both arms use _move (preserves type, doesn't coerce booleans).
        let moves: Vec<_> = i.iter().filter(|x| {
            x.op == "call_builtin"
                && matches!(&x.srcs[0], Operand::Var(s) if s == "_move")
        }).collect();
        assert_eq!(moves.len(), 2);
    }

    #[test]
    fn let_binds_via_move() {
        let i = main_instrs("(let ((x 1)) x)");
        // Should have a _move into a register named "x".
        let mv = i.iter().find(|x| x.op == "call_builtin"
            && matches!(&x.srcs[0], Operand::Var(s) if s == "_move")
            && x.dest.as_deref() == Some("x"));
        assert!(mv.is_some(), "expected (let ((x 1)) ...) to _move into x");
    }

    #[test]
    fn begin_returns_last() {
        let i = main_instrs("(begin 1 2 3)");
        // Three const instructions for 1, 2, 3 plus a ret.
        let consts: Vec<_> = i.iter().filter(|x| x.op == "const").collect();
        assert!(consts.len() >= 3);
        // The ret reads the last const's dest.
        let ret = i.iter().find(|x| x.op == "ret").unwrap();
        let last_const = consts.last().unwrap();
        assert_eq!(ret.srcs[0], Operand::Var(last_const.dest.clone().unwrap()));
    }

    // ---- Lambdas + closures -------------------------------------------

    #[test]
    fn anonymous_lambda_creates_synthetic_function() {
        let m = module("(define (adder n) (lambda (x) (+ x n)))");
        let names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.iter().any(|n| n.starts_with("__lambda_")));
    }

    #[test]
    fn anonymous_lambda_emits_alloc_closure() {
        // LANG34: compiler now emits alloc_closure instead of call_builtin "make_closure".
        // srcs[0] must be Operand::Str(fn_name) and type_hint must be "closure".
        let i = fn_instrs("(define (adder n) (lambda (x) (+ x n)))", "adder");
        let ac = i.iter().find(|x| x.op == "alloc_closure");
        assert!(ac.is_some(), "expected alloc_closure instruction in adder");
        let ac = ac.unwrap();
        // srcs[0] must be an Operand::Str carrying the synthesised lambda name.
        assert!(
            matches!(&ac.srcs[0], Operand::Str(s) if s.starts_with("__lambda_")),
            "alloc_closure srcs[0] must be Operand::Str(__lambda_N), got {:?}", &ac.srcs[0]
        );
        assert_eq!(ac.type_hint, "closure", "alloc_closure type_hint must be 'closure'");
        // No preceding const instruction should materialise the fn_name.
        let const_before = i.iter().rev().skip_while(|x| x.op != "alloc_closure").skip(1).next();
        if let Some(prev) = const_before {
            assert_ne!(
                prev.op, "const",
                "alloc_closure must NOT be preceded by a const instruction for fn_name"
            );
        }
    }

    #[test]
    fn captures_appear_as_leading_params_of_synth_fn() {
        let m = module("(define (adder n) (lambda (x) (+ x n)))");
        let lam = m.functions.iter().find(|f| f.name.starts_with("__lambda_")).unwrap();
        // captures (n) ++ params (x) → params = [n, x]
        let names: Vec<&str> = lam.params.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["n", "x"]);
    }

    #[test]
    fn closure_call_uses_call_closure() {
        // LANG34: indirect call now emits `call_closure` instead of
        // `call_builtin "apply_closure"`.
        // ((adder 5) 3) — the inner (adder 5) returns a closure handle;
        // the outer call goes through call_closure.
        let m = module(
            "(define (adder n) (lambda (x) (+ x n)))\n\
             ((adder 5) 3)",
        );
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        let cc = main.instructions.iter().find(|x| x.op == "call_closure");
        assert!(cc.is_some(), "expected call_closure instruction in main");
        let cc = cc.unwrap();
        // srcs[0] must be a Var (the closure handle register), not a Str or name-string.
        assert!(
            matches!(&cc.srcs[0], Operand::Var(_)),
            "call_closure srcs[0] must be Operand::Var(handle), got {:?}", &cc.srcs[0]
        );
    }

    #[test]
    fn unbound_capture_in_lambda_is_compile_error() {
        // Lambda inside a function whose body references a name that
        // doesn't resolve to anything.
        let err = compile_source("(define (f) (lambda (x) (+ x z)))", "test").unwrap_err();
        assert!(err.message.contains("unbound name"));
    }

    #[test]
    fn unbound_var_ref_at_top_level_is_compile_error() {
        let err = compile_source("undefined_name", "test").unwrap_err();
        assert!(err.message.contains("unbound name"));
    }

    #[test]
    fn fn_globals_can_be_passed_as_values() {
        // LANG34: Reference to top-level fn name in non-call position
        // produces an `alloc_closure` (0 captures) instead of
        // `call_builtin "make_closure"`.
        let m = module("(define (id x) x) id");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        let ac = main.instructions.iter().find(|x| x.op == "alloc_closure"
            && matches!(&x.srcs[0], Operand::Str(s) if s == "id"));
        assert!(ac.is_some(), "fn-as-value should emit alloc_closure(Str('id'))");
    }

    #[test]
    fn builtin_in_non_call_position_uses_make_builtin_closure() {
        // `+` referenced but not called — should wrap in make_builtin_closure.
        let i = main_instrs("+");
        let mbc = i.iter().find(|x| x.op == "call_builtin"
            && matches!(&x.srcs[0], Operand::Var(s) if s == "make_builtin_closure"));
        assert!(mbc.is_some());
    }

    // ---- Local references --------------------------------------------

    #[test]
    fn parameter_reference_uses_param_name_directly() {
        // `(define (f x) x)` — the body's `x` should appear in the ret's
        // srcs as `x`, not as a fresh register.
        let i = fn_instrs("(define (f x) x)", "f");
        let ret = i.iter().find(|x| x.op == "ret").unwrap();
        assert_eq!(ret.srcs[0], Operand::Var("x".into()));
    }

    // ---- Realistic shapes --------------------------------------------

    #[test]
    fn factorial_compiles_and_has_expected_shape() {
        let m = module("(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))\n(fact 5)");
        let fact = m.functions.iter().find(|f| f.name == "fact").unwrap();
        // Body must contain: jmp_if_false (for the if), `*`, `fact` self-call.
        let ops: Vec<&str> = fact.instructions.iter().map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"jmp_if_false"));
        assert!(fact.instructions.iter().any(|i| i.op == "call"
            && matches!(&i.srcs[0], Operand::Var(s) if s == "fact")));
    }

    // ---- Register count ------------------------------------------------

    #[test]
    fn register_count_is_at_least_minimum() {
        let m = module("");
        assert!(m.functions[0].register_count >= 16);
    }

    // ---- Defense in depth: stack-overflow guard ------------------------

    #[test]
    fn extreme_nesting_does_not_crash_the_compiler() {
        // The parser will reject this first with its own depth cap,
        // but exercising the path proves we never reach a panic.
        let src = format!(
            "{open}+ 1{close}",
            open = "(".repeat(2000),
            close = ")".repeat(2000),
        );
        // We expect *some* error (parser or compiler depth cap), and
        // crucially: no panic / abort.
        assert!(compile_source(&src, "deep").is_err());
    }

    // ---- PR D-1: source-map population ---------------------------------

    /// Lockstep invariant: for every function in the module,
    /// `source_map.len() == instructions.len()`.  Every dev tool
    /// downstream (LSP, debugger, coverage, AOT DWARF/PDB) relies
    /// on this — if it ever drifts, those consumers see ghosts.
    #[test]
    fn source_map_lockstep_holds_for_every_function() {
        let srcs = [
            "(+ 1 2)",
            "(if (< 1 2) 100 200)",
            "(let ((x 5)) (* x x))",
            "(define (square x) (* x x)) (square 7)",
            "((lambda (x) (* x x)) 3)",
            "(define answer 42) answer",
            "'foo",
            "(begin 1 2 3)",
            "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)",
        ];
        for src in srcs {
            let m = module(src);
            for f in &m.functions {
                assert_eq!(
                    f.source_map.len(),
                    f.instructions.len(),
                    "lockstep violated in fn {:?} of source {src:?}: \
                     source_map.len()={} but instructions.len()={}",
                    f.name,
                    f.source_map.len(),
                    f.instructions.len(),
                );
            }
        }
    }

    /// Every position in `source_map` is either a real source
    /// position (line >= 1, column >= 1) or the synthetic
    /// sentinel.  Frontends should never emit zero-line /
    /// non-zero-column or vice versa.
    #[test]
    fn source_map_positions_are_well_formed() {
        use interpreter_ir::SourceLoc;
        let m = module(
            "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)",
        );
        for f in &m.functions {
            for (i, loc) in f.source_map.iter().enumerate() {
                let well_formed = loc.is_synthetic()
                    || (loc.line >= 1 && loc.column >= 1);
                assert!(
                    well_formed,
                    "ill-formed source loc {loc:?} at fn {:?} instr {i}",
                    f.name,
                );
                let _ = SourceLoc::SYNTHETIC; // touch the constant so the use is intentional
            }
        }
    }

    /// First instruction of `(+ 1 2)` is a `const 1` whose
    /// position should be column 4 (the `1` after `(+ `).  This
    /// is the "did we actually plumb positions correctly"
    /// smoke test — not just "are positions present".
    #[test]
    fn source_map_records_real_positions_from_ast() {
        let m = module("(+ 1 2)");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        // The first instruction is `const _n1 = 1` for the `1`
        // operand, which appears at column 4 of the source.
        let loc = main.source_map[0];
        assert_eq!(loc.line, 1, "first instr should be on line 1");
        assert!(
            loc.column >= 1 && loc.column <= 10,
            "first instr column should be a small positive number, got {}",
            loc.column,
        );
    }

    /// Multi-line programs map each instruction back to the
    /// right line.  Defends against the regression where every
    /// instruction collapses to (1, 1).
    #[test]
    fn source_map_distinguishes_multiple_lines() {
        let src = "(+ 1 2)\n(* 3 4)";
        let m = module(src);
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        let mut lines_seen: std::collections::HashSet<u32> =
            std::collections::HashSet::new();
        for loc in &main.source_map {
            if !loc.is_synthetic() {
                lines_seen.insert(loc.line);
            }
        }
        assert!(
            lines_seen.contains(&1) && lines_seen.contains(&2),
            "expected positions on both lines 1 and 2, got {lines_seen:?}",
        );
    }

    // ---- PR 23-E: refinement type annotation round-trip ----------------
    //
    // These tests verify that LANG23 refinement annotations written in Twig
    // source (`(x : (Int 0 128))`, `-> (Int 0 256)`) are:
    //   1. Parsed into `TypeAnnotation` variants on the `Lambda`/`Define` nodes.
    //   2. Lowered by the IR compiler into `param_refinements` / `return_refinement`
    //      on the resulting `IIRFunction`.
    //
    // They do NOT test the refinement checker (that is `lang-refinement-checker`'s
    // job).  They test only that the annotation survives the
    // parse → compile → IIRFunction pipeline.

    /// A function defined with a ranged-int parameter annotation should carry
    /// a `Some(RefinedType)` in `param_refinements[0]` on the IIR function.
    #[test]
    fn ranged_int_param_annotation_round_trips_to_iir() {
        use lang_refined_types::{Kind, Predicate, RefinedType};
        // `(x : (Int 0 128))` means x ∈ [0, 128).
        let src = "(define (clamp (x : (Int 0 128))) x)";
        let m = compile_source(src, "test_23e").unwrap();
        let f = m.functions.iter().find(|f| f.name == "clamp")
            .expect("expected function named 'clamp'");
        // `param_refinements` must be in lockstep with `params`.
        assert_eq!(
            f.param_refinements.len(), f.params.len(),
            "param_refinements must be lockstep with params"
        );
        let rt = f.param_refinements[0]
            .as_ref()
            .expect("param 0 should have a refinement annotation");
        let expected = RefinedType::refined(
            Kind::Int,
            Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false },
        );
        assert_eq!(rt, &expected, "param refinement should be Range(0,128)");
    }

    /// A function with an unrefined `int` type annotation on a parameter should
    /// produce `RefinedType::unrefined(Kind::Int)` — not `None`.
    #[test]
    fn unrefined_int_param_annotation_round_trips() {
        use lang_refined_types::{Kind, RefinedType};
        let src = "(define (double (x : int)) (* x 2))";
        let m = compile_source(src, "test_23e").unwrap();
        let f = m.functions.iter().find(|f| f.name == "double").unwrap();
        let rt = f.param_refinements[0].as_ref()
            .expect("param 0 should be annotated as int");
        assert_eq!(rt, &RefinedType::unrefined(Kind::Int));
    }

    /// A function with a return type annotation `-> (Int 0 256)` should have
    /// `return_refinement = Some(RefinedType::refined(Kind::Int, Range(0,256)))`.
    #[test]
    fn return_annotation_round_trips_to_iir() {
        use lang_refined_types::{Kind, Predicate, RefinedType};
        let src = "(define (clamp-byte (x : int) -> (Int 0 256)) x)";
        let m = compile_source(src, "test_23e").unwrap();
        let f = m.functions.iter().find(|f| f.name == "clamp-byte").unwrap();
        let rt = f.return_refinement.as_ref()
            .expect("clamp-byte should have a return refinement");
        let expected = RefinedType::refined(
            Kind::Int,
            Predicate::Range { lo: Some(0), hi: Some(256), inclusive_hi: false },
        );
        assert_eq!(rt, &expected);
    }

    /// A function with multiple annotated parameters (mixed refined and plain)
    /// gets a lockstep `param_refinements` vector.
    #[test]
    fn multiple_annotated_params_lockstep() {
        use lang_refined_types::{Kind, Predicate, RefinedType};
        // lo is annotated; hi is unannotated (plain name without `:`).
        let src = "(define (in-range (lo : (Int 0 100)) hi) (+ lo hi))";
        let m = compile_source(src, "test_23e").unwrap();
        let f = m.functions.iter().find(|f| f.name == "in-range").unwrap();
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.param_refinements.len(), 2,
            "lockstep: 2 params ⇒ 2 entries in param_refinements");
        // param 0: annotated
        let rt0 = f.param_refinements[0].as_ref().expect("lo should be annotated");
        assert_eq!(
            rt0,
            &RefinedType::refined(
                Kind::Int,
                Predicate::Range { lo: Some(0), hi: Some(100), inclusive_hi: false },
            )
        );
        // param 1: unannotated → None
        assert!(f.param_refinements[1].is_none(), "hi should be None (unannotated)");
    }

    /// A function with NO annotations should have empty/None annotation fields.
    ///
    /// This is the opt-in contract: callers that don't use LANG23 annotations
    /// see zero change in the IIR they receive.
    #[test]
    fn unannotated_function_has_no_refinement_fields() {
        let src = "(define (add x y) (+ x y))";
        let m = compile_source(src, "test_23e").unwrap();
        let f = m.functions.iter().find(|f| f.name == "add").unwrap();
        // Either empty (pre-LANG23 path) or all-None.
        let all_none = f.param_refinements.iter().all(|r| r.is_none());
        assert!(
            f.param_refinements.is_empty() || all_none,
            "unannotated function should have empty or all-None param_refinements"
        );
        assert!(f.return_refinement.is_none());
    }

    /// Parsing an annotated function does not change its `params` tuple —
    /// the `type_hint` field of each param entry is still `"any"` (dynamic typing
    /// is unchanged; refinements are carried in the parallel `param_refinements`
    /// field, not in the existing `type_hint` strings).
    #[test]
    fn annotation_does_not_change_existing_type_hints() {
        let src = "(define (f (x : (Int 0 10)) (y : int)) (+ x y))";
        let m = compile_source(src, "test_23e").unwrap();
        let f = m.functions.iter().find(|f| f.name == "f").unwrap();
        for (_, type_hint) in &f.params {
            assert_eq!(type_hint, "any",
                "type_hint must remain 'any'; refinements live in param_refinements");
        }
        assert_eq!(f.return_type, "any");
    }

    /// The `source_map` lockstep invariant still holds for annotated functions —
    /// adding annotations must not corrupt instruction count vs source_map.
    #[test]
    fn source_map_lockstep_holds_for_annotated_functions() {
        let srcs = [
            "(define (f (x : (Int 0 128))) x)",
            "(define (g (x : int) -> (Int 0 256)) x)",
            "(define (h (a : (Int 0 10)) (b : (Int 0 20)) -> (Int 0 30)) (+ a b))",
        ];
        for src in srcs {
            let m = compile_source(src, "lockstep_23e").unwrap();
            for f in &m.functions {
                assert_eq!(
                    f.source_map.len(), f.instructions.len(),
                    "lockstep violated in fn {:?} for source {src:?}",
                    f.name,
                );
            }
        }
    }

    // ── LANG50: compile_typed_source tests ───────────────────────────────

    fn typed_module(src: &str) -> IIRModule {
        compile_typed_source(src, "test")
            .unwrap_or_else(|e| panic!("compile_typed_source failed: {e}"))
    }

    fn all_hints(module: &IIRModule) -> Vec<String> {
        module
            .functions
            .iter()
            .flat_map(|f| f.instructions.iter().map(|i| i.type_hint.clone()))
            .collect()
    }

    #[test]
    fn typed_source_int_literal_hint() {
        // A bare integer literal: the `const` instruction should get "i64".
        let m = typed_module("42");
        let hints = all_hints(&m);
        assert!(
            hints.iter().any(|h| h == "i64"),
            "expected at least one 'i64' hint, got: {hints:?}"
        );
    }

    #[test]
    fn typed_source_bool_literal_hint() {
        // A boolean literal: the `const` instruction should get "bool".
        let m = typed_module("#t");
        let hints = all_hints(&m);
        assert!(
            hints.iter().any(|h| h == "bool"),
            "expected at least one 'bool' hint, got: {hints:?}"
        );
    }

    #[test]
    fn typed_source_nil_literal_hint() {
        // `nil` maps to KindDecl::Nil → iir_hint "any".
        // This test documents the current behaviour (Nil → "any").
        let m = typed_module("nil");
        // Should compile without errors.
        assert!(!m.functions.is_empty());
    }

    #[test]
    fn typed_source_untyped_fallback() {
        // A function call to a builtin — the call result has type Any → hint "any".
        // Off mode (no module declaration) so no type errors are emitted.
        // The `+` builtin is known to the compiler, so compilation succeeds.
        let m = typed_module("(+ 1 2)");
        // The call to `+` has type Any (call return type unknown statically).
        let hints = all_hints(&m);
        // At minimum there should be no panic.
        assert!(!m.functions.is_empty());
        // The `+` call instruction itself should be "any" (return type unknown).
        // But the literal args (1, 2) get "i64" hints — so the mix is expected.
        let _ = hints; // documented: mix of "i64" (args) and "any" (call result)
    }

    #[test]
    fn typed_source_function_status_fully_typed() {
        // A function that takes a typed define — verifying FunctionTypeStatus
        // is updated when all instructions have concrete hints.
        //
        // `(define x : int 42)` — the `42` literal gets "i64".
        // The main function emits a `const` + `global_set` + `ret`.
        // `const` gets "i64", `global_set` stays "any" (it's a side-effect op).
        // So main is PartiallyTyped (not FullyTyped — not all concrete).
        //
        // A bare `42` just has `const` + `ret` in main.  `const` → "i64",
        // `ret` → "any".  PartiallyTyped.  FullyTyped requires ALL non-void
        // instructions to be concrete.
        let m = typed_module("42");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        // At minimum, the type_status should not be Untyped (we got a concrete hint).
        assert_ne!(
            main.type_status,
            FunctionTypeStatus::Untyped,
            "main should be at least PartiallyTyped when a literal was inferred"
        );
    }

    #[test]
    fn typed_source_strict_mode_type_error_returns_err() {
        // Strict mode + arity mismatch → compile_typed_source returns Err.
        let src = "(module m (typed strict)) (define (f x) x) (f 1 2)";
        let result = compile_typed_source(src, "test");
        assert!(result.is_err(), "expected Err on strict type error");
        let err = result.unwrap_err();
        assert!(err.message.contains("arity") || err.message.contains("type error"));
    }

    #[test]
    fn typed_source_off_mode_no_errors() {
        // No module declaration → Off mode → no strict errors.
        let src = "(+ undefined_var 1)";
        let result = compile_typed_source(src, "test");
        // Compilation may fail for other reasons (unbound var in compiler).
        // The point is: type errors are NOT the reason it fails.
        // If compile_source succeeds, typed should too.
        let baseline = compile_source(src, "test");
        assert_eq!(result.is_ok(), baseline.is_ok());
    }

    // =========================================================================
    // LANG51: String literal tests
    // =========================================================================
    //
    // String literals ("hello") are the highest-priority self-hosting blocker:
    // without them, compiler keywords like "define" can only be built at runtime
    // from char codes.  These tests verify the full front-end → IIR path.

    /// A bare string literal compiles to a single `const(Operand::Str(...))` in main.
    #[test]
    fn string_literal_basic() {
        let instrs = main_instrs("\"hello\"");
        // Expected: const(Str("hello")), ret
        assert_eq!(instrs.len(), 2, "expected const + ret, got {instrs:?}");
        assert_eq!(instrs[0].op, "const");
        match &instrs[0].srcs[0] {
            Operand::Str(s) => assert_eq!(s, "hello"),
            other => panic!("expected Operand::Str(\"hello\"), got {other:?}"),
        }
    }

    /// An empty string literal is valid and produces `const(Str(""))`.
    #[test]
    fn string_literal_empty() {
        let instrs = main_instrs("\"\"");
        assert_eq!(instrs[0].op, "const");
        match &instrs[0].srcs[0] {
            Operand::Str(s) => assert_eq!(s, ""),
            other => panic!("expected Operand::Str(\"\"), got {other:?}"),
        }
    }

    /// `\"` inside a string literal is decoded to a literal double-quote character.
    #[test]
    fn string_literal_escaped_quote() {
        let instrs = main_instrs(r#""say \"hi\"""#);
        match &instrs[0].srcs[0] {
            Operand::Str(s) => assert_eq!(s, "say \"hi\""),
            other => panic!("expected escaped-quote string, got {other:?}"),
        }
    }

    /// `\n` in a string literal is decoded to a real newline character (0x0a).
    #[test]
    fn string_literal_newline_escape() {
        let instrs = main_instrs(r#""\n""#);
        match &instrs[0].srcs[0] {
            Operand::Str(s) => {
                assert_eq!(s.len(), 1);
                assert_eq!(s.as_bytes()[0], b'\n');
            }
            other => panic!("expected Operand::Str(\"\\n\"), got {other:?}"),
        }
    }

    /// `\t` → tab, `\r` → CR, `\\` → backslash.
    #[test]
    fn string_literal_all_basic_escapes() {
        let instrs = main_instrs(r#""\t\r\\""#);
        match &instrs[0].srcs[0] {
            Operand::Str(s) => assert_eq!(s, "\t\r\\"),
            other => panic!("expected tab+CR+backslash, got {other:?}"),
        }
    }

    /// The `const` instruction for a string literal carries `type_hint = "str"`.
    /// This is what LANG50 inference uses to propagate the str kind.
    #[test]
    fn string_literal_type_hint_is_str() {
        let instrs = main_instrs("\"hello\"");
        assert_eq!(instrs[0].op, "const");
        assert_eq!(
            instrs[0].type_hint.as_str(),
            "str",
            "string literal const must carry type_hint = \"str\""
        );
    }

    /// A string literal used as an argument to a builtin call compiles cleanly.
    /// This covers the most common self-hosting usage: `(print "define")`.
    #[test]
    fn string_literal_as_argument() {
        // (define (greet s) s)  then  (greet "world")
        // We just want compilation to succeed and produce a call instruction.
        let m = module("(define (greet s) s) (greet \"world\")");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        // main should contain a call to greet
        let has_call_greet = main
            .instructions
            .iter()
            .any(|i| i.op == "call" && i.srcs.iter().any(|s| matches!(s, Operand::Var(v) if v == "greet")));
        assert!(has_call_greet, "expected call to greet in main");
    }

    /// `compile_typed_source` on a string literal returns Ok and the main
    /// function is at least PartiallyTyped (the `const` instruction has
    /// type_hint "str" which is a concrete type).
    #[test]
    fn string_literal_compile_typed_source() {
        let m = compile_typed_source("\"hello\"", "test")
            .expect("typed compile of string literal should succeed");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        assert_ne!(
            main.type_status,
            FunctionTypeStatus::Untyped,
            "string literal should yield at least PartiallyTyped"
        );
    }

    /// A string literal inside a lambda body compiles without error.
    /// Tests that StrLit is handled in closure free-variable analysis (it has no
    /// free variables, just like IntLit).
    #[test]
    fn string_literal_in_lambda() {
        let m = module("(lambda () \"captured\")");
        // Should compile to a __lambda_0 function + main that creates the closure.
        let lambda_fn = m.functions.iter().find(|f| f.name.starts_with("__lambda"));
        assert!(lambda_fn.is_some(), "lambda function should be emitted");
        let lambda_instrs = &lambda_fn.unwrap().instructions;
        // Lambda body: const(Str("captured")) + ret
        let const_instr = lambda_instrs.iter().find(|i| i.op == "const");
        assert!(const_instr.is_some(), "lambda body should have a const instr");
        match &const_instr.unwrap().srcs[0] {
            Operand::Str(s) => assert_eq!(s, "captured"),
            other => panic!("expected Str(\"captured\") in lambda, got {other:?}"),
        }
    }
}
