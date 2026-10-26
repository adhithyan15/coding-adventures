//! The shape of the IIR the release path emits, and its refusals.
//!
//! `release_subsets.rs` proves the corpus *runs* right; this file pins *how*:
//! which LANG83 op each construct becomes, the `-1` divisor fold, the `i64`
//! entry point, typed `Unsupported` errors with their spans, and a
//! caller-supplied tree that has been tampered with.

use interpreter_ir::IIRModule;
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use python_iir_compiler::{
    compile_cst, compile_source_at, run_source_at, FrontendBudgets, FrontendError, RunOutput,
};

fn compile(source: &str) -> Result<IIRModule, FrontendError> {
    compile_source_at(source, None, "t", &FrontendBudgets::default())
}

fn ops(source: &str) -> Vec<String> {
    compile(source).unwrap().functions[0]
        .instructions
        .iter()
        .map(|i| i.op.clone())
        .collect()
}

#[test]
fn arithmetic_lowers_to_the_lang83_checked_ops() {
    let ops = ops("print(1 + 2 * 3 - 4 // 5 % 6)\n");
    for op in ["i64_add_ovf", "i64_mul_ovf", "i64_sub_ovf", "i64_div_floor", "i64_mod_floor"] {
        assert!(ops.iter().any(|o| o == op), "{op} missing from {ops:?}");
    }
    // No wrapping op survives: Python ints never wrap.
    for op in ["add", "sub", "mul", "neg", "div", "mod"] {
        assert!(!ops.iter().any(|o| o == op), "{op} in {ops:?}");
    }
    assert!(ops.iter().any(|o| o == "call_builtin"));
}

#[test]
fn unary_minus_is_checked_and_unary_plus_is_the_identity() {
    assert_eq!(ops("print(-5)\n"), ["const", "i64_neg_ovf", "call_builtin", "const", "ret"]);
    assert_eq!(ops("print(+5)\n"), ["const", "call_builtin", "const", "ret"]);
    assert_eq!(ops("print(+-+5)\n"), ["const", "i64_neg_ovf", "call_builtin", "const", "ret"]);
}

#[test]
fn a_minus_one_divisor_never_reaches_a_divide() {
    let floor = ops("print(7 // -1)\n");
    assert!(floor.contains(&"i64_neg_ovf".to_string()) && !floor.contains(&"i64_div_floor".to_string()), "{floor:?}");
    let modulo = ops("print((-9223372036854775807 - 1) % -1)\n");
    assert!(!modulo.contains(&"i64_mod_floor".to_string()), "{modulo:?}");
    // A neighbouring divisor still divides.
    assert!(ops("print(7 // -2)\n").contains(&"i64_div_floor".to_string()));
    assert!(ops("print(7 % 1)\n").contains(&"i64_mod_floor".to_string()));
    // `MIN // -1` is `-MIN`: the checked negation traps, as 2^63 is outside i64.
    let ran = run_source_at("print(0)\nprint((-9223372036854775807 - 1) // -1)\n", None).unwrap();
    assert_eq!(ran, RunOutput { stdout: "0\n".into(), trap: Some("Trap.Overflow".into()) });
}

#[test]
fn main_returns_i64_zero() {
    let module = compile("print()\n").unwrap();
    let main = &module.functions[0];
    assert_eq!((main.name.as_str(), main.return_type.as_str()), ("main", "i64"));
    assert_eq!(module.entry_point.as_deref(), Some("main"));
    let ops: Vec<_> = main.instructions.iter().map(|i| i.op.as_str()).collect();
    assert_eq!(ops, ["const", "call_builtin", "const", "ret"]);
    assert_eq!(module.language, "python");
}

#[test]
fn traps_keep_the_output_before_them() {
    for (source, stdout, kind) in [
        ("print(1)\nprint(1 // 0)\nprint(2)\n", "1\n", "Trap.DivZero"),
        ("print(1)\nprint(9223372036854775807 + 1)\n", "1\n", "Trap.Overflow"),
        ("print(-9223372036854775807 - 2)\n", "", "Trap.Overflow"),
        ("print(3037000500 * 3037000500)\n", "", "Trap.Overflow"),
        ("print(-(-9223372036854775807 - 1))\n", "", "Trap.Overflow"),
    ] {
        let ran = run_source_at(source, None).unwrap();
        assert_eq!(ran, RunOutput { stdout: stdout.into(), trap: Some(kind.into()) }, "{source:?}");
    }
}

#[test]
fn refusals_are_typed_and_located() {
    let cases = [
        ("print(7.5 // 2)\n", "`//` and `%` on a float"),
        ("print(1e400)\n", "overflows to inf"),
        ("print(9007199254740993 / 3)\n", "above 2**53"),
        ("abs(-3)\n", "only `print`"),
        ("print(1, 2)\n", "at most one argument"),
        ("1 + 2\n", "bare expression"),
        ("print(print(1))\n", "used as a value"),
        ("print(x)\n", "names such as `x`"),
        ("print(9223372036854775808)\n", "outside i64"),
    ];
    for (source, needle) in cases {
        let error = compile(source).unwrap_err();
        let FrontendError::Unsupported { release, start, message, .. } = &error else {
            panic!("{source:?}: {error:?}");
        };
        assert_eq!(release, "3.13.16");
        assert!(message.contains(needle), "{source:?}: {message}");
        let (line, column) = start.expect("a refusal points at the source");
        assert!(line == 1 && column >= 1, "{source:?}: {start:?}");
        assert!(error.to_string().starts_with("Python 3.13.16: "), "{error}");
    }
}

// ---------------------------------------------------------------------------
// A caller-supplied tree (`compile_cst`) is checked, not trusted
// ---------------------------------------------------------------------------

fn tree(source: &str) -> GrammarASTNode {
    coding_adventures_python_parser::parse_python_at(source, None, &FrontendBudgets::default())
        .unwrap()
        .ast
}

/// Apply `edit` to the first token for which it returns true.
fn edit_first_token(node: &mut GrammarASTNode, edit: &mut dyn FnMut(&mut Token) -> bool) -> bool {
    for child in &mut node.children {
        match child {
            ASTNodeOrToken::Token(token) => {
                if edit(token) {
                    return true;
                }
            }
            ASTNodeOrToken::Node(inner) => {
                if edit_first_token(inner, edit) {
                    return true;
                }
            }
        }
    }
    false
}

/// A tree is only lowered under the profile whose grammar can produce its
/// `print` shape: `print x` (a `print_stmt`) is a Python 2 statement, and
/// `print(x)` is a Python 3 call. Swapping them would print the wrong thing
/// (Python 2 prints `print ()` as `()`), so each is refused under the other.
#[test]
fn a_tree_from_the_other_profiles_grammar_is_refused() {
    let budgets = FrontendBudgets::default();
    let py3_tree = coding_adventures_python_parser::parse_python_at("print()\nprint(1)\n", Some("3.13.16"), &budgets)
        .unwrap()
        .ast;
    let py2_tree = coding_adventures_python_parser::parse_python_at("print\nprint 1\n", Some("2.7.18"), &budgets)
        .unwrap()
        .ast;
    assert!(compile_cst(&py3_tree, Some("3.13.16"), "t").is_ok());
    assert!(compile_cst(&py2_tree, Some("2.7.18"), "t").is_ok());

    let error = compile_cst(&py3_tree, Some("2.7.18"), "t").unwrap_err();
    assert_eq!(error.category(), "Unsupported", "{error}");
    assert!(error.to_string().contains("call-form"), "{error}");

    let error = compile_cst(&py2_tree, Some("3.13.16"), "t").unwrap_err();
    assert_eq!(error.category(), "Unsupported", "{error}");
    assert!(error.to_string().contains("`print` statement is not in py-float-print"), "{error}");
}

#[test]
fn forged_trees_are_refused() {
    assert!(compile_cst(&tree("print(1)\n"), None, "t").is_ok());

    // A forged INT spelling, a forged callee, and an INT re-typed as a NAME.
    for (from, to, needle) in [("1", "1x", "not a decimal integer"), ("1", "-1", "not a decimal integer"), ("1", "012", "leading zeros")] {
        let mut ast = tree("print(1)\n");
        assert!(edit_first_token(&mut ast, &mut |t| {
            (t.value == from).then(|| t.value = to.into()).is_some()
        }));
        let error = compile_cst(&ast, None, "t").unwrap_err();
        assert!(error.to_string().contains(needle), "{to:?}: {error}");
    }
    // The same checks under the py2 decoder (phase 7c slice 2): every
    // Python 2 spelling is re-checked, and Python 3's `_` is not one.
    let budgets = FrontendBudgets::default();
    for (to, needle) in [
        ("1_0", "not a Python 2 integer literal"),
        ("0x", "not a Python 2 integer literal"),
        ("08", "not a Python 2 integer literal"),
        ("1LL", "not a Python 2 integer literal"),
        ("L", "not a Python 2 integer literal"),
        ("0b1000000000000000000000000000000000000000000000000000000000000000", "outside i64"),
        ("01000000000000000000000L", "outside i64"),
    ] {
        let mut ast = coding_adventures_python_parser::parse_python_at("print 1\n", Some("2.6"), &budgets).unwrap().ast;
        assert!(edit_first_token(&mut ast, &mut |t| (t.value == "1").then(|| t.value = to.into()).is_some()));
        let error = compile_cst(&ast, Some("2.6"), "t").unwrap_err();
        assert_eq!(error.category(), "Unsupported", "{to:?}: {error}");
        assert!(error.to_string().contains(needle), "{to:?}: {error}");
    }
    let mut ast = tree("print(1)\n");
    assert!(edit_first_token(&mut ast, &mut |t| (t.value == "print").then(|| t.value = "exec".into()).is_some()));
    assert!(compile_cst(&ast, None, "t").unwrap_err().to_string().contains("only `print`"));
    let mut ast = tree("print(1)\n");
    assert!(edit_first_token(&mut ast, &mut |t| {
        (t.value == "1").then(|| t.type_name = Some("NAME".into())).is_some()
    }));
    assert_eq!(compile_cst(&ast, None, "t").unwrap_err().category(), "Unsupported");

    // A wrong root and an over-deep tree.
    let mut wrong = tree("print(1)\n");
    wrong.rule_name = "module".into();
    assert_eq!(compile_cst(&wrong, None, "t").unwrap_err().category(), "Unsupported");
    let node = |rule: &str, children: Vec<ASTNodeOrToken>| GrammarASTNode {
        rule_name: rule.into(),
        children,
        start_line: None,
        start_column: None,
        end_line: None,
        end_column: None,
    };
    let mut deep = node("paren_body", vec![]);
    for _ in 0..300 {
        deep = node("paren_body", vec![ASTNodeOrToken::Node(deep)]);
    }
    let deep = node("file", vec![ASTNodeOrToken::Node(deep)]);
    assert!(matches!(
        compile_cst(&deep, None, "t"),
        Err(FrontendError::Budget { which: "CST depth", .. })
    ));
    // A release with no profile is refused before the tree is read.
    assert_eq!(compile_cst(&tree("print(1)\n"), Some("3.12.2"), "t").unwrap_err().category(), "ReleaseNotAvailable");
    // An oversized module name is a budget error.
    let name = "m".repeat(70_000);
    assert!(matches!(
        compile_cst(&tree("print(1)\n"), None, &name),
        Err(FrontendError::Budget { which: "module name bytes", .. })
    ));
}

// ---------------------------------------------------------------------------
// LANG84 phase 7b: the module records its release; reasons read on their own
// ---------------------------------------------------------------------------

/// Every profiled release compiles and the module names the one that did,
/// from source and from a caller-supplied tree alike; `None` is the default.
#[test]
fn the_module_records_the_release_that_produced_it() {
    let budgets = FrontendBudgets::default();
    for &release in python_iir_compiler::supported_releases() {
        let module = compile_source_at("print(1)\n", Some(release), "t", &budgets).unwrap();
        assert_eq!(module.release.as_deref(), Some(release));
        // The tree comes from the release's own grammar: `print(1)` is a
        // call under py3 and the statement `print (1)` under py2.
        let own_tree = coding_adventures_python_parser::parse_python_at("print(1)\n", Some(release), &budgets)
            .unwrap()
            .ast;
        let from_tree = compile_cst(&own_tree, Some(release), "t").unwrap();
        assert_eq!(from_tree.release.as_deref(), Some(release));
    }
    let default = compile_source_at("print(1)\n", None, "t", &budgets).unwrap();
    assert_eq!(default.release.as_deref(), Some(python_iir_compiler::DEFAULT_RELEASE));
}

/// A row with no grammar pair says so in its reason (lang-aot shows it as
/// `cannot compile yet (…)`); a regeneration-bug phrase is not relabelled
/// as a `grammar_stage`.
#[test]
fn release_not_available_reasons_are_self_describing() {
    let budgets = FrontendBudgets::default();
    match compile_source_at("print(1)\n", Some("3.12.2"), "t", &budgets) {
        Err(FrontendError::ReleaseNotAvailable { stage, .. }) => {
            assert_eq!(stage, "no grammar pair: grammar_stage none");
        }
        other => panic!("3.12.2: {other:?}"),
    }
    let phrase = "token table missing (regenerate python-lexer)";
    let bug = FrontendError::from(coding_adventures_python_parser::PythonFrontendError::ReleaseNotAvailable {
        release: "3.13.16".into(),
        stage: phrase.into(),
    });
    assert!(matches!(&bug, FrontendError::ReleaseNotAvailable { stage, .. } if stage == phrase), "{bug:?}");
}

// ---------------------------------------------------------------------------
// LANG84 phase 7c slice 3: `py-float-print`
// ---------------------------------------------------------------------------

/// A float literal is an `f64` `const`; `print` writes it with LANG83
/// `write_f64_py` (no newline) and then `putchar 10`.
#[test]
fn a_float_prints_through_write_f64_py_and_a_newline() {
    let module = compile("print(1.5)\n").unwrap();
    let main = &module.functions[0].instructions;
    let ops: Vec<_> = main.iter().map(|i| i.op.as_str()).collect();
    assert_eq!(ops, ["const", "call_builtin", "const", "call_builtin", "const", "ret"]);
    assert_eq!(main[0].type_hint, "f64");
    assert!(matches!(main[0].srcs[..], [interpreter_ir::Operand::Float(x)] if x == 1.5));
    assert!(matches!(&main[1].srcs[0], interpreter_ir::Operand::Var(name) if name == "write_f64_py"));
    assert!(matches!(&main[3].srcs[0], interpreter_ir::Operand::Var(name) if name == "putchar"));
}

/// An int meeting a float is widened with `int_to_real`; float arithmetic
/// is the PR6c IEEE family, never the classic `add`/`div` (whose edges are
/// backend-dependent) and never the checked `i64_*` ops.
#[test]
fn mixed_arithmetic_widens_the_int_and_uses_the_ieee_ops() {
    assert_eq!(
        ops("print(1 + 0.5)\n"),
        ["const", "const", "int_to_real", "f64_add", "call_builtin", "const", "call_builtin", "const", "ret"]
    );
    let all = ops("print(2 * 0.5 - 1.5 / 3)\n");
    for op in ["f64_mul", "f64_sub", "f64_div", "int_to_real"] {
        assert!(all.iter().any(|o| o == op), "{op} missing from {all:?}");
    }
    for op in ["add", "sub", "mul", "div", "neg", "i64_add_ovf", "i64_sub_ovf", "i64_mul_ovf"] {
        assert!(!all.iter().any(|o| o == op), "{op} in {all:?}");
    }
    // True division of two ints converts both.
    assert_eq!(
        ops("print(7 / 2)\n"),
        ["const", "const", "int_to_real", "int_to_real", "f64_div", "call_builtin", "const", "call_builtin", "const", "ret"]
    );
    // An int sub-expression keeps its checked int ops.
    let nested = ops("print(7 // 2 * 1.5)\n");
    assert!(nested.contains(&"i64_div_floor".to_string()) && nested.contains(&"f64_mul".to_string()), "{nested:?}");
}

/// `-x` on a float flips the sign bit (`f64_neg`); `+x` is the identity.
#[test]
fn float_signs_are_a_sign_flip_and_the_identity() {
    assert_eq!(ops("print(-0.0)\n"), ["const", "f64_neg", "call_builtin", "const", "call_builtin", "const", "ret"]);
    assert_eq!(ops("print(+1.5)\n"), ["const", "call_builtin", "const", "call_builtin", "const", "ret"]);
    for (source, stdout) in [("print(-0.0)\n", "-0.0\n"), ("print(+-0.0)\n", "-0.0\n"), ("print(0.0 - 0.0)\n", "0.0\n")] {
        assert_eq!(run_source_at(source, None).unwrap().stdout, stdout, "{source:?}");
    }
}

/// A zero divisor (`0`, `0.0` or `-0.0`, known while lowering) emits LANG83
/// `trap "Trap.DivZero"` before the division, so the program stops after
/// its earlier output as CPython's ZeroDivisionError does. Any other
/// divisor emits no trap: `f64_div` itself never traps.
#[test]
fn a_zero_divisor_traps_before_the_division() {
    for source in ["print(1.0 / 0)\n", "print(1 / 0)\n", "print(1 / -0.0)\n", "print(1.5 / (2 - 2))\n"] {
        let all = ops(source);
        let trap = all.iter().position(|o| o == "trap").unwrap_or_else(|| panic!("{source:?}: {all:?}"));
        let div = all.iter().position(|o| o == "f64_div").unwrap();
        assert!(trap < div, "{source:?}: {all:?}");
        let ran = run_source_at(&format!("print(2.5)\n{source}print(3)\n"), None).unwrap();
        assert_eq!(ran, RunOutput { stdout: "2.5\n".into(), trap: Some("Trap.DivZero".into()) }, "{source:?}");
    }
    assert!(!ops("print(1.0 / 1e-300)\n").contains(&"trap".to_string()));
    // `inf` and `nan` come from arithmetic, never from a trap.
    let ran = run_source_at("print(1e308 * 10)\nprint(1e308 * 10 - 1e308 * 10)\n", None).unwrap();
    assert_eq!(ran, RunOutput { stdout: "inf\nnan\n".into(), trap: None });
}

/// `int / int` at the 2^53 boundary: both exact, accepted; one past it,
/// refused (CPython rounds the exact quotient once).
#[test]
fn true_division_of_ints_stops_at_two_to_the_53() {
    let ran = run_source_at("print(9007199254740992 / 3)\nprint(-9007199254740992 / 9007199254740992)\n", None).unwrap();
    assert_eq!(ran.stdout, "3002399751580330.5\n-1.0\n");
    for source in ["print(9007199254740993 / 3)\n", "print(3 / -9007199254740993)\n"] {
        assert_eq!(compile(source).unwrap_err().category(), "Unsupported", "{source:?}");
    }
    // Mixing is not division: an int above 2^53 meeting a float is
    // converted once, as `PyLong_AsDouble` does.
    assert_eq!(run_source_at("print(9007199254740993 + 0.0)\n", None).unwrap().stdout, "9007199254740992.0\n");
}

/// A caller-supplied FLOAT token is re-checked: only a Python 3 float
/// spelling reaches Rust's parser (never `inf` or `nan`), and one that
/// overflows is refused.
#[test]
fn forged_float_tokens_are_refused() {
    for to in ["1x", "inf", "nan", "1e", ".", "_1.5", "1.5_", "1__0.5", "1e+-3", "+1.5", "0x1p3", "1.5.5", "1", "", "1e5.5"] {
        let mut ast = tree("print(2.5)\n");
        assert!(edit_first_token(&mut ast, &mut |t| (t.value == "2.5").then(|| t.value = to.into()).is_some()));
        let error = compile_cst(&ast, None, "t").unwrap_err();
        assert_eq!(error.category(), "Unsupported", "{to:?}: {error}");
        assert!(error.to_string().contains("not a Python float literal"), "{to:?}: {error}");
    }
    for to in ["1e400", "1_0e3_08"] {
        let mut ast = tree("print(2.5)\n");
        assert!(edit_first_token(&mut ast, &mut |t| (t.value == "2.5").then(|| t.value = to.into()).is_some()));
        assert!(compile_cst(&ast, None, "t").unwrap_err().to_string().contains("overflows to inf"), "{to:?}");
    }
    // Every accepted spelling of one value lowers to the same IIR.
    let iir = |source: &str| {
        let mut module = compile(source).unwrap();
        module.release = None;
        format!("{module:?}")
    };
    for spelling in ["0.5", ".5", "00.5", "5e-1", "5E-1", "0.5e0", "0_0.5", ".5_0", "50e-2"] {
        assert_eq!(iir(&format!("print({spelling})\n")), iir("print(0.5)\n"), "{spelling}");
    }
}
