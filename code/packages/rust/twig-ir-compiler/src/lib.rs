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

use interpreter_ir::IIRModule;
use twig_parser::{parse, Program};

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
    Compiler::new().compile(program, module_name)
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
    fn anonymous_lambda_emits_make_closure() {
        let i = fn_instrs("(define (adder n) (lambda (x) (+ x n)))", "adder");
        let mc = i.iter().find(|x| x.op == "call_builtin"
            && matches!(&x.srcs[0], Operand::Var(s) if s == "make_closure"));
        assert!(mc.is_some(), "expected make_closure call_builtin in adder");
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
    fn closure_call_uses_apply_closure() {
        // ((adder 5) 3) — the inner (adder 5) returns a closure
        // handle; the outer call goes through apply_closure.
        let m = module(
            "(define (adder n) (lambda (x) (+ x n)))\n\
             ((adder 5) 3)",
        );
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        let ac = main.instructions.iter().find(|x| x.op == "call_builtin"
            && matches!(&x.srcs[0], Operand::Var(s) if s == "apply_closure"));
        assert!(ac.is_some(), "expected apply_closure call_builtin in main");
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
        // Reference to top-level fn name in non-call position
        // produces a `make_closure` (0 captures).
        let m = module("(define (id x) x) id");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        let mc = main.instructions.iter().find(|x| x.op == "call_builtin"
            && matches!(&x.srcs[0], Operand::Var(s) if s == "make_closure"));
        assert!(mc.is_some(), "fn-as-value should wrap in make_closure");
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
}
