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
        ("print(7 / 2)\n", "true division"),
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
        let from_tree = compile_cst(&tree("print(1)\n"), Some(release), "t").unwrap();
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
