//! Integration tests for `twig-to-cil`.
//!
//! These tests exercise `compile_twig_to_cil` end-to-end: Twig source
//! string → `CILProgramArtifact` (or `TwigToCilError` on failure).
//!
//! ## Architectural note — the CLR/Twig type-system gap
//!
//! The CLR backend (`iir-to-cil-bytecode`) requires **fully typed** IIR:
//! every instruction's `type_hint` must be a concrete type (not `"any"`).
//!
//! Twig is a dynamically-typed language.  The IIR compiler emits all
//! instructions with `type_hint = "any"`.  Type inference fills in types for
//! constants and arithmetic, but control-flow instructions (`ret`, `label`,
//! `jmp_if_false`, `jmp`) and runtime builtins (`make_nil`, `_move`,
//! `global_set`) retain `"any"`.  Additionally, `call_builtin` instructions
//! that survive the builtin-lowering pass are explicitly unsupported by the
//! CLR backend.
//!
//! As a result, most Twig programs produce a [`TwigToCilError::ClrValidation`]
//! or [`TwigToCilError::ClrBackend`] error from this pipeline.
//!
//! ## Test strategy
//!
//! 1. **Frontend errors** (Section 1): verify broken Twig → `Compile` error.
//!
//! 2. **Pipeline progression** (Section 2): verify that valid Twig reaches the
//!    CLR stage (not a `Compile` or `TypeCheck` error), even if the CLR backend
//!    ultimately rejects it.
//!
//! 3. **Typed IIR** (Section 3): use `run_pipeline_from_iir` with hand-built
//!    fully-typed IIR to verify the CLR backend integration works end-to-end.
//!    These tests prove the backend produces correct CIL bytecode.

use twig_to_cil::{compile_twig_to_cil, error::TwigToCilError, CILProgramArtifact};

// CIL opcode constants used in byte-level assertions.
const RET: u8 = 0x2A;
const ADD: u8 = 0x58;
const SUB: u8 = 0x59;
const MUL: u8 = 0x5A;
const DIV: u8 = 0x5B;

// ===========================================================================
// Helper
// ===========================================================================

/// Assert that the error is a CLR-stage error — proving that the Twig
/// frontend (stages 1-3) succeeded and only the CLR backend rejected the
/// module.
fn assert_clr_stage_error(result: Result<CILProgramArtifact, TwigToCilError>, source: &str) {
    match result {
        Ok(_) => {} // Successful compilation is also acceptable
        Err(TwigToCilError::ClrValidation(_)) | Err(TwigToCilError::ClrBackend(_)) => {}
        Err(TwigToCilError::Compile(e)) => panic!(
            "frontend should not fail for valid Twig {source:?}: compile error: {e}"
        ),
        Err(TwigToCilError::TypeCheck(errs)) => panic!(
            "type-check should not fail for valid Twig {source:?}: {errs:?}"
        ),
    }
}

// ===========================================================================
// 1. Compile errors — broken Twig syntax
// ===========================================================================

/// Unclosed parenthesis — the parser rejects this before any IIR.
#[test]
fn broken_syntax_returns_compile_error() {
    let result = compile_twig_to_cil("(+ 1", "broken");
    assert!(
        matches!(result, Err(TwigToCilError::Compile(_))),
        "expected Compile error for broken syntax"
    );
}

/// Extra closing parenthesis.
#[test]
fn extra_paren_returns_compile_error() {
    let result = compile_twig_to_cil("(+ 1 2))", "broken");
    assert!(
        matches!(result, Err(TwigToCilError::Compile(_))),
        "expected Compile error for extra paren"
    );
}

/// Unbound variable reference — the IIR compiler rejects it.
#[test]
fn unbound_variable_returns_compile_error() {
    let result = compile_twig_to_cil("undefined_var_xyz_123", "bad");
    assert!(
        matches!(result, Err(TwigToCilError::Compile(_))),
        "expected Compile error for unbound var"
    );
}

/// Lambda that captures an unbound name.
#[test]
fn lambda_unbound_capture_returns_compile_error() {
    let result = compile_twig_to_cil("(define (f) (lambda (x) (+ x free_var_xyz)))", "bad");
    assert!(
        matches!(result, Err(TwigToCilError::Compile(_))),
        "expected Compile error for lambda with unbound capture"
    );
}

/// Malformed define — syntactically invalid.
#[test]
fn malformed_define_returns_compile_error() {
    let result = compile_twig_to_cil("(define)", "bad");
    assert!(
        matches!(result, Err(TwigToCilError::Compile(_))),
        "expected Compile error for malformed define"
    );
}

// ===========================================================================
// 2. Pipeline progression — valid Twig reaches the CLR stage
// ===========================================================================

/// `(+ 1 2)` — valid Twig; frontend succeeds; CLR backend may reject.
#[test]
fn add_two_integers_reaches_clr_stage() {
    let result = compile_twig_to_cil("(+ 1 2)", "test");
    assert_clr_stage_error(result, "(+ 1 2)");
}

/// `(- 5 3)` — subtraction; frontend succeeds.
#[test]
fn subtract_two_integers_reaches_clr_stage() {
    let result = compile_twig_to_cil("(- 5 3)", "test");
    assert_clr_stage_error(result, "(- 5 3)");
}

/// `(* 2 4)` — multiplication; frontend succeeds.
#[test]
fn multiply_two_integers_reaches_clr_stage() {
    let result = compile_twig_to_cil("(* 2 4)", "test");
    assert_clr_stage_error(result, "(* 2 4)");
}

/// `(/ 10 2)` — division; frontend succeeds.
#[test]
fn divide_two_integers_reaches_clr_stage() {
    let result = compile_twig_to_cil("(/ 10 2)", "test");
    assert_clr_stage_error(result, "(/ 10 2)");
}

/// `(= 1 1)` — equality comparison; frontend succeeds.
#[test]
fn equality_comparison_reaches_clr_stage() {
    let result = compile_twig_to_cil("(= 1 1)", "test");
    assert_clr_stage_error(result, "(= 1 1)");
}

/// `(< 1 2)` — less-than comparison; frontend succeeds.
#[test]
fn less_than_comparison_reaches_clr_stage() {
    let result = compile_twig_to_cil("(< 1 2)", "test");
    assert_clr_stage_error(result, "(< 1 2)");
}

/// `(> 3 1)` — greater-than comparison; frontend succeeds.
#[test]
fn greater_than_comparison_reaches_clr_stage() {
    let result = compile_twig_to_cil("(> 3 1)", "test");
    assert_clr_stage_error(result, "(> 3 1)");
}

/// `(if (= 1 1) 42 0)` — conditional; frontend succeeds.
#[test]
fn simple_if_reaches_clr_stage() {
    let result = compile_twig_to_cil("(if (= 1 1) 42 0)", "test");
    assert_clr_stage_error(result, "(if (= 1 1) 42 0)");
}

/// Nested `if`; frontend succeeds.
#[test]
fn nested_if_reaches_clr_stage() {
    let result = compile_twig_to_cil("(if (= 1 1) (if (< 2 3) 100 200) 0)", "test");
    assert_clr_stage_error(result, "nested if");
}

/// Factorial; frontend succeeds.
#[test]
fn factorial_reaches_clr_stage() {
    let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)";
    let result = compile_twig_to_cil(src, "test");
    assert_clr_stage_error(result, "factorial");
}

/// Fibonacci; frontend succeeds.
#[test]
fn fibonacci_reaches_clr_stage() {
    let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)";
    let result = compile_twig_to_cil(src, "test");
    assert_clr_stage_error(result, "fibonacci");
}

/// Multiple functions; frontend succeeds.
#[test]
fn multiple_functions_reach_clr_stage() {
    let src = "(define (double x) (* x 2)) (define (triple x) (* x 3)) (+ (double 2) (triple 3))";
    let result = compile_twig_to_cil(src, "test");
    assert_clr_stage_error(result, "multiple functions");
}

/// Mutual recursion; frontend succeeds.
#[test]
fn mutual_recursion_reaches_clr_stage() {
    let src =
        "(define (even? n) (if (= n 0) 1 (odd? (- n 1))))\n\
         (define (odd? n)  (if (= n 0) 0 (even? (- n 1))))\n\
         (even? 4)";
    let result = compile_twig_to_cil(src, "test");
    assert_clr_stage_error(result, "mutual recursion");
}

/// Boolean logic; frontend succeeds.
#[test]
fn boolean_logic_reaches_clr_stage() {
    let result = compile_twig_to_cil("(if #t 1 0)", "test");
    assert_clr_stage_error(result, "boolean if");
}

/// `let` binding; frontend succeeds.
#[test]
fn let_binding_reaches_clr_stage() {
    let result = compile_twig_to_cil("(let ((x 5)) (* x x))", "test");
    assert_clr_stage_error(result, "let binding");
}

/// `begin` expression; frontend succeeds.
#[test]
fn begin_expression_reaches_clr_stage() {
    let result = compile_twig_to_cil("(begin 1 2 3)", "test");
    assert_clr_stage_error(result, "begin");
}

/// Two-argument function; frontend succeeds.
#[test]
fn two_arg_function_reaches_clr_stage() {
    let result = compile_twig_to_cil("(define (add a b) (+ a b)) (add 3 4)", "test");
    assert_clr_stage_error(result, "two-arg function");
}

// ===========================================================================
// 3. Programs that compile via run_pipeline_from_iir
//
// These use hand-crafted, fully-typed IIR to bypass the Twig frontend's
// dynamic-typing limitation.  They verify the CLR backend integration.
// ===========================================================================

/// A typed `add(a: i32, b: i32) -> i32` function compiles to CIL.
///
/// The body must contain the `add` opcode (0x58) and the `ret` opcode (0x2A).
#[test]
fn typed_add_function_compiles_to_cil() {
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
    use twig_to_cil::pipeline::run_pipeline_from_iir;
    use twig_to_cil::IIRClrConfig;

    let fn_ = IIRFunction::new(
        "add",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "add",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let mut module = IIRModule::new("add_test", "test");
    module.entry_point = Some("add".into());
    module.add_or_replace(fn_);

    let config = IIRClrConfig::new("AddAssembly");
    let artifact = run_pipeline_from_iir(module, config).unwrap();
    assert!(!artifact.methods.is_empty(), "expected at least one method");
    let add_method = artifact.methods.iter().find(|m| m.name == "add")
        .expect("expected method named 'add'");
    assert!(!add_method.body.is_empty(), "method body must be non-empty");
    assert!(add_method.body.contains(&ADD), "must contain ADD (0x58)");
    assert!(add_method.body.contains(&RET), "must contain RET (0x2A)");
}

/// A typed `sub(a: i32, b: i32) -> i32` function compiles with `sub` opcode.
#[test]
fn typed_sub_function_compiles_to_cil() {
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
    use twig_to_cil::pipeline::run_pipeline_from_iir;
    use twig_to_cil::IIRClrConfig;

    let fn_ = IIRFunction::new(
        "sub",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("sub", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let mut module = IIRModule::new("sub_test", "test");
    module.entry_point = Some("sub".into());
    module.add_or_replace(fn_);

    let artifact = run_pipeline_from_iir(module, IIRClrConfig::default()).unwrap();
    let method = artifact.methods.iter().find(|m| m.name == "sub").unwrap();
    assert!(method.body.contains(&SUB), "must contain SUB (0x59)");
}

/// A typed `mul` function compiles with `mul` opcode.
#[test]
fn typed_mul_function_compiles_to_cil() {
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
    use twig_to_cil::pipeline::run_pipeline_from_iir;
    use twig_to_cil::IIRClrConfig;

    let fn_ = IIRFunction::new(
        "mul",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("mul", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let mut module = IIRModule::new("mul_test", "test");
    module.entry_point = Some("mul".into());
    module.add_or_replace(fn_);

    let artifact = run_pipeline_from_iir(module, IIRClrConfig::default()).unwrap();
    let method = artifact.methods.iter().find(|m| m.name == "mul").unwrap();
    assert!(method.body.contains(&MUL), "must contain MUL (0x5A)");
}

/// A typed `div` function compiles with `div` opcode.
#[test]
fn typed_div_function_compiles_to_cil() {
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
    use twig_to_cil::pipeline::run_pipeline_from_iir;
    use twig_to_cil::IIRClrConfig;

    let fn_ = IIRFunction::new(
        "div",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("div", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let mut module = IIRModule::new("div_test", "test");
    module.entry_point = Some("div".into());
    module.add_or_replace(fn_);

    let artifact = run_pipeline_from_iir(module, IIRClrConfig::default()).unwrap();
    let method = artifact.methods.iter().find(|m| m.name == "div").unwrap();
    assert!(method.body.contains(&DIV), "must contain DIV (0x5B)");
}

/// A void function with `ret_void` compiles to CIL with the `ret` opcode.
#[test]
fn typed_void_function_compiles_to_cil() {
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule};
    use twig_to_cil::pipeline::run_pipeline_from_iir;
    use twig_to_cil::IIRClrConfig;

    let fn_ = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let mut module = IIRModule::new("void_test", "test");
    module.entry_point = Some("main".into());
    module.add_or_replace(fn_);

    let artifact = run_pipeline_from_iir(module, IIRClrConfig::default()).unwrap();
    assert!(!artifact.methods.is_empty());
    assert!(artifact.methods[0].body.contains(&RET));
}

/// Entry method is accessible via `artifact.entry_method()`.
#[test]
fn artifact_entry_method_accessible() {
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule};
    use twig_to_cil::pipeline::run_pipeline_from_iir;
    use twig_to_cil::IIRClrConfig;

    let fn_ = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let mut module = IIRModule::new("entry_test", "test");
    module.entry_point = Some("main".into());
    module.add_or_replace(fn_);

    let artifact = run_pipeline_from_iir(module, IIRClrConfig::default()).unwrap();
    let entry = artifact.entry_method();
    assert!(entry.is_some(), "artifact must have an entry method");
    assert_eq!(entry.unwrap().name, "main");
}

/// Two functions in the same module — both appear in the artifact.
#[test]
fn two_typed_functions_both_appear_in_artifact() {
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
    use twig_to_cil::pipeline::run_pipeline_from_iir;
    use twig_to_cil::IIRClrConfig;

    let double_fn = IIRFunction::new(
        "double",
        vec![("x".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("add", Some("r".into()),
                vec![Operand::Var("x".into()), Operand::Var("x".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let triple_fn = IIRFunction::new(
        "triple",
        vec![("x".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("mul", Some("r".into()),
                vec![Operand::Var("x".into()),
                     Operand::Var("x".into())],  // simplified: x*x instead of x*3
                "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let mut module = IIRModule::new("multi_test", "test");
    module.entry_point = Some("double".into());
    module.add_or_replace(double_fn);
    module.add_or_replace(triple_fn);

    let artifact = run_pipeline_from_iir(module, IIRClrConfig::default()).unwrap();
    let names: Vec<&str> = artifact.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"double"), "expected double method; got {names:?}");
    assert!(names.contains(&"triple"), "expected triple method; got {names:?}");
    assert_eq!(artifact.methods.len(), 2);
}

/// Every method in a multi-function artifact has a non-empty body.
#[test]
fn every_method_body_non_empty_in_multi_fn_artifact() {
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
    use twig_to_cil::pipeline::run_pipeline_from_iir;
    use twig_to_cil::IIRClrConfig;

    let f1 = IIRFunction::new("f1",
        vec![("a".into(), "i32".into())], "i32",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "i32")]);
    let f2 = IIRFunction::new("f2",
        vec![("b".into(), "i32".into())], "i32",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i32")]);

    let mut module = IIRModule::new("multi2", "test");
    module.entry_point = Some("f1".into());
    module.add_or_replace(f1);
    module.add_or_replace(f2);

    let artifact = run_pipeline_from_iir(module, IIRClrConfig::default()).unwrap();
    for method in &artifact.methods {
        assert!(!method.body.is_empty(), "method {:?} has empty body", method.name);
        assert!(method.body.contains(&RET), "method {:?} missing ret", method.name);
    }
}

// ===========================================================================
// 4. Error type specificity
// ===========================================================================

/// Broken syntax produces exactly `Compile`, not a CLR-stage error.
#[test]
fn broken_syntax_is_exactly_compile_error_not_clr_error() {
    let result = compile_twig_to_cil("(+ 1", "broken");
    assert!(
        matches!(result, Err(TwigToCilError::Compile(_))),
        "broken syntax must produce a Compile error"
    );
    // Should NOT reach the CLR stage:
    assert!(!matches!(result, Err(TwigToCilError::ClrValidation(_))));
    assert!(!matches!(result, Err(TwigToCilError::ClrBackend(_))));
}

/// Valid Twig programs must not produce `TypeCheck` errors.
#[test]
fn valid_twig_does_not_produce_type_check_error() {
    let result = compile_twig_to_cil("(+ 1 2)", "test");
    assert!(
        !matches!(result, Err(TwigToCilError::TypeCheck(_))),
        "valid Twig must not produce a TypeCheck error"
    );
}

/// Diagnostic: print full CLR artifact for fib
#[test]
fn diag_clr_fib_exec() {
    use std::collections::HashMap;
    use interpreter_ir::{IIRInstr, Operand};
    use iir_type_checker::infer_and_check;
    use twig_ir_compiler::compile_source;
    use twig_to_cil::pipeline::run_pipeline_from_iir;
    use twig_to_cil::IIRClrConfig;

    const FIB_PROGRAM: &str =
        "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)";
    const CLR_BUILTIN_MAP: &[(&str, &str)] = &[
        ("+",  "add"),   ("-",  "sub"),  ("*",  "mul"),  ("/",  "div"),
        ("=",  "cmp_eq"), ("<", "cmp_lt"), (">", "cmp_gt"),
        ("<=", "cmp_le"), (">=", "cmp_ge"),
        ("not", "not"),  ("_move", "mov"),
    ];

    let mut iir = compile_source(FIB_PROGRAM, "twig_fib").unwrap();
    // pre_lower_builtins_clr
    for func in &mut iir.functions {
        let old = std::mem::take(&mut func.instructions);
        func.instructions = old.into_iter().map(|instr| {
            if instr.op != "call_builtin" { return instr; }
            let name = match instr.srcs.first() {
                Some(Operand::Var(n)) => n.as_str(),
                _ => return instr,
            };
            let Some((_, op)) = CLR_BUILTIN_MAP.iter().find(|(b, _)| *b == name) else { return instr; };
            let args: Vec<Operand> = instr.srcs[1..].to_vec();
            IIRInstr::new(*op, instr.dest.clone(), args, &instr.type_hint)
        }).collect();
    }
    infer_and_check(&mut iir);
    
    // fixup_control_flow_types
    for func in &mut iir.functions {
        let mut env: HashMap<String, String> = HashMap::new();
        for (param_name, _) in &func.params {
            env.insert(param_name.clone(), "i64".to_string());
        }
        for instr in &func.instructions {
            if let Some(dest) = &instr.dest {
                let ty = &instr.type_hint;
                if ty != "any" && ty != "polymorphic" {
                    env.insert(dest.clone(), ty.clone());
                }
            }
        }
        for instr in &mut func.instructions {
            if instr.type_hint != "any" { continue; }
            let fixed = match instr.op.as_str() {
                "ret_void" | "label" | "jmp" | "jmp_if_true" | "jmp_if_false" => "void".to_string(),
                "ret" => match instr.srcs.first() {
                    Some(Operand::Var(src)) => env.get(src).cloned().unwrap_or("void".into()),
                    Some(Operand::Int(_)) => "i64".to_string(),
                    _ => "void".to_string(),
                },
                "call" => {
                    if let Some(dest) = &instr.dest {
                        env.get(dest).cloned().unwrap_or("i64".into())
                    } else { "void".to_string() }
                }
                "mov" => match instr.srcs.first() {
                    Some(Operand::Var(src)) => env.get(src).cloned().unwrap_or("i64".into()),
                    _ => "i64".to_string(),
                },
                "add" | "sub" | "mul" | "div" => {
                    instr.srcs.iter().find_map(|s| {
                        if let Operand::Var(n) = s { env.get(n).cloned() } else { None }
                    }).unwrap_or("i64".into())
                }
                "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => "bool".to_string(),
                _ => "any".to_string(),
            };
            if fixed != "any" {
                instr.type_hint = fixed.clone();
                if let Some(dest) = &instr.dest {
                    env.insert(dest.clone(), fixed);
                }
            }
        }
    }
    
    let config = IIRClrConfig::new("TwigFib");
    match run_pipeline_from_iir(iir, config) {
        Err(e) => {
            eprintln!("CLR compile FAILED: {:?}", e);
            return;
        }
        Ok(artifact) => {
            eprintln!("CLR compile OK: {} methods", artifact.methods.len());
            for (i, m) in artifact.methods.iter().enumerate() {
                eprintln!("  method[{}] = {} ({} params): {:02X?}", 
                    i, m.name, m.parameter_types.len(), &m.body[..m.body.len().min(30)]);
            }
        }
    }
}
