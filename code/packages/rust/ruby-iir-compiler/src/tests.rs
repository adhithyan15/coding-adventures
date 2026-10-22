//! Unit tests: the walker's output, its refusals, and hostile direct trees.
//!
//! The corpus-driven stage I proof (every `rb-int-puts` case on every
//! release that claims it) is `tests/release_subsets.rs`; this file pins the
//! compiler's own contract case by case.

use super::*;
use coding_adventures_ruby_parser::parse_ruby_at;
use interpreter_ir::op_registry::registry_name;
use interpreter_ir::Operand;

fn run(source: &str) -> RunOutput {
    run_source_at(source, None).unwrap_or_else(|e| panic!("{source:?}: {e}"))
}

fn stdout(source: &str) -> String {
    let output = run(source);
    assert_eq!(output.trap, None, "{source:?} trapped");
    output.stdout
}

fn error(source: &str) -> FrontendError {
    compile_source(source, "t").expect_err(source)
}

fn tree(source: &str) -> GrammarASTNode {
    parse_ruby_at(source, None, &FrontendBudgets::default()).unwrap().ast
}

fn ops(source: &str) -> Vec<String> {
    let module = compile_source(source, "t").unwrap();
    module.functions[0].instructions.iter().map(|i| i.op.clone()).collect()
}

// ---------------------------------------------------------------------------
// What programs do
// ---------------------------------------------------------------------------

#[test]
fn integer_arithmetic_prints_what_ruby_prints() {
    // Every expectation below is host ruby 3.3.6's output for the same text.
    let cases = [
        ("puts(1 + 2)", "3\n"),
        ("puts 1 + 2 * 3 - 8 / 4 % 3", "5\n"),
        ("puts((1 + 2) * 3)", "9\n"),
        ("puts(10 - 3 - 2)", "5\n"),
        ("puts(- -5, -(3 - 10))", "5\n7\n"),
        ("puts(1_000_000 * 3)", "3000000\n"),
        ("puts(+5 - +2)", "3\n"),
        ("puts 4, 5", "4\n5\n"),
        ("puts(1); puts(2)", "1\n2\n"),
        ("puts(1 +\n2)\n# comment\nputs(\n3,\n4\n)", "3\n3\n4\n"),
        ("puts(9223372036854775807, -9223372036854775807 - 1)", "9223372036854775807\n-9223372036854775808\n"),
    ];
    for (source, expected) in cases {
        assert_eq!(stdout(source), expected, "{source:?}");
    }
}

#[test]
fn division_and_modulo_round_toward_negative_infinity() {
    assert_eq!(stdout("puts(7 / 2, -7 / 2, 7 / -2, -7 / -2)"), "3\n-4\n-4\n3\n");
    assert_eq!(stdout("puts(7 % 3, -7 % 3, 7 % -3, -7 % -3)"), "1\n2\n-2\n-1\n");
    // The edges LANG83 defines: MIN % -1 is 0, MIN / 1 is MIN.
    assert_eq!(stdout("puts((-9223372036854775807 - 1) % -1)"), "0\n");
    assert_eq!(stdout("puts((-9223372036854775807 - 1) / 1)"), "-9223372036854775808\n");
}

#[test]
fn empty_puts_writes_one_newline_and_an_empty_program_writes_nothing() {
    assert_eq!(stdout("puts()\nputs(7)\nputs()"), "\n7\n\n");
    assert_eq!(stdout(""), "");
    assert_eq!(stdout("\n;\n"), "");
}

#[test]
fn a_zero_divisor_traps_after_the_earlier_output() {
    let output = run("puts(1)\nputs(2 / 0)\nputs(3)");
    assert_eq!(output.stdout, "1\n");
    assert_eq!(output.trap.as_deref(), Some("Trap.DivZero"));
    assert_eq!(run("puts(5 % 0)").trap.as_deref(), Some("Trap.DivZero"));
    // Ruby evaluates every argument before `puts` writes, so nothing of this
    // call is printed.
    let output = run("puts(1, 1 / 0)");
    assert_eq!((output.stdout.as_str(), output.trap.as_deref()), ("", Some("Trap.DivZero")));
    // Values after a certain trap are unknown and not range-checked: this
    // statement never runs.
    let output = run("puts((1 / 0) * 9223372036854775807 * 9)");
    assert_eq!(output.trap.as_deref(), Some("Trap.DivZero"));
    // The legacy entry point reports the trap as an error.
    assert_eq!(run_source("puts(1 / 0)"), Err("trap: Trap.DivZero".into()));
}

// ---------------------------------------------------------------------------
// The IIR it emits
// ---------------------------------------------------------------------------

#[test]
fn lowering_uses_lang83_ops_and_no_ruby_named_builtin() {
    assert_eq!(
        ops("puts(1 + 2, 7 / -2, 3 % 4)"),
        [
            "const", "const", "add", //
            "const", "const", "neg", "i64_div_floor", //
            "const", "const", "i64_mod_floor", //
            "call_builtin", "call_builtin", "call_builtin", //
            "const", "ret",
        ]
    );
    assert_eq!(ops("puts()"), ["const", "call_builtin", "const", "ret"]);
    // LANG84 C7: every instruction is a registry entry, or `const`/`ret`
    // (pre-registry IIR the registry does not gate). No `rb_*` builtin.
    for source in ["puts(1 + 2 * 3 - 4 / 5 % 6, -7)", "puts()", "puts 1, 2"] {
        let module = compile_source(source, "t").unwrap();
        assert_eq!(module.entry_point.as_deref(), Some("main"));
        assert_eq!(module.functions[0].return_type, "i64");
        for instr in &module.functions[0].instructions {
            match registry_name(instr) {
                Some(name) => assert!(
                    ["add", "sub", "mul", "neg", "i64_div_floor", "i64_mod_floor", "print_i64", "putchar"]
                        .contains(&name),
                    "{source}: {name}"
                ),
                None => assert!(["const", "ret"].contains(&instr.op.as_str()), "{source}: {instr:?}"),
            }
        }
    }
}

/// Security review (LANG84 5a): `i64::MIN % -1` never reaches a backend.
/// The compiler folds that one edge to `const 0` (Ruby's answer), so a
/// machine `rem` that would fault or panic on it is never emitted, while the
/// operands are still evaluated.
#[test]
fn min_mod_minus_one_is_folded_and_never_emitted() {
    let source = "puts((-9223372036854775807 - 1) % -1)";
    assert_eq!(stdout(source), "0\n");
    let emitted = ops(source);
    assert!(!emitted.contains(&"i64_mod_floor".to_string()), "{emitted:?}");
    assert_eq!(emitted.iter().filter(|op| *op == "sub").count(), 1, "{emitted:?}");
    // Every other modulus still lowers to the LANG83 op.
    assert!(ops("puts((-9223372036854775807 - 1) % 1, 7 % -1)").contains(&"i64_mod_floor".to_string()));
    // The reviewer's spelling with a space before `(` is outside the R0
    // grammar (`puts (1) + 2` is refused there), and is refused cleanly.
    assert_eq!(error("puts (-9223372036854775807 - 1) % -1").category(), "Parse");
}

#[test]
fn every_argument_is_evaluated_before_any_is_printed() {
    let module = compile_source("puts(1 + 2, 3 * 4)", "t").unwrap();
    let instrs = &module.functions[0].instructions;
    let first_print = instrs.iter().position(|i| i.op == "call_builtin").unwrap();
    assert!(instrs[..first_print].iter().any(|i| i.op == "mul"));
    let printed: Vec<_> = instrs[first_print..]
        .iter()
        .filter(|i| i.op == "call_builtin")
        .map(|i| i.srcs[1].clone())
        .collect();
    // Each printed operand is the register its own expression produced,
    // in source order — not a pre-rendered string.
    let producers: Vec<_> = printed
        .iter()
        .map(|operand| {
            let Operand::Var(name) = operand else { panic!("{operand:?}") };
            instrs.iter().find(|i| i.dest.as_ref() == Some(name)).unwrap().op.clone()
        })
        .collect();
    assert_eq!(producers, ["add", "mul"]);
}

// ---------------------------------------------------------------------------
// Refusals, each typed and naming the release
// ---------------------------------------------------------------------------

#[test]
fn values_outside_i64_are_refused_at_compile_time() {
    for source in [
        "puts(9223372036854775808)",
        "puts(9223372036854775807 + 1)",
        "puts(-9223372036854775807 - 2)",
        "puts(4294967296 * 4294967296)",
        "puts((-9223372036854775807 - 1) / -1)",
        "puts(-(-9223372036854775807 - 1))",
        "puts(123456789012345678901234567890)",
    ] {
        let FrontendError::Unsupported { release, message, start, .. } = error(source) else {
            panic!("{source}: not Unsupported");
        };
        assert_eq!(release, DEFAULT_RELEASE);
        assert!(message.contains("outside i64"), "{source}: {message}");
        assert!(start.is_some(), "{source}");
    }
}

#[test]
fn only_puts_is_a_callee() {
    let FrontendError::Unsupported { message, rule, start, .. } = error("print(1)") else {
        panic!("print(1) must be Unsupported");
    };
    assert!(message.contains("`print`"), "{message}");
    assert_eq!((rule.as_str(), start), ("method_call", Some((1, 1))));
    assert_eq!(error("p 1, 2").category(), "Unsupported");
}

#[test]
fn syntax_outside_the_subset_is_a_parse_or_lex_error_with_a_position() {
    // `puts -7`: unary-or-binary depends on spacing (LANG84 E6); refused.
    for (source, category) in [
        ("puts -7", "Parse"),
        ("puts(0x10)", "Parse"),
        ("puts(010)", "Parse"),
        ("puts(1,)", "Parse"),
        ("x = 1", "Parse"),
        ("puts", "Parse"),
        ("puts(\"1\")", "Lex"),
    ] {
        let error = error(source);
        assert_eq!(error.category(), category, "{source}: {error}");
        assert_eq!(error.release(), Some(DEFAULT_RELEASE));
        assert!(matches!(error, FrontendError::Parse { line: 1, .. } | FrontendError::Lex { line: 1, .. }));
    }
}

#[test]
fn release_selection_is_exact_typed_and_never_falls_back() {
    let budgets = FrontendBudgets::default();
    // Each profiled release compiles and the module records which one did
    // (LANG84 phase 5b); `None` records the default.
    for release in ["3.3.5", "3.3.6", "3.3.7"] {
        let module = compile_source_at("puts(1)", Some(release), "t", &budgets).unwrap();
        assert_eq!(module.release.as_deref(), Some(release));
        let ast = tree("puts(1)");
        assert_eq!(compile_cst(&ast, Some(release), "t").unwrap().release.as_deref(), Some(release));
    }
    assert_eq!(compile_source("puts(1)", "t").unwrap().release.as_deref(), Some(DEFAULT_RELEASE));
    // 0.49 parses the subset but has no profile (see profiles.rs).
    for release in ["0.49", "0.50"] {
        let error = compile_source_at("puts(1)", Some(release), "t", &budgets).unwrap_err();
        let FrontendError::ReleaseNotAvailable { release: named, stage } = &error else {
            panic!("{release}: {error:?}");
        };
        assert_eq!(named, release);
        assert!(stage.contains("no IIR profile"), "{stage}");
    }
    // A row with no grammar pair says so in its reason, which reads on its
    // own (lang-aot shows it as `cannot compile yet (…)`).
    match compile_source_at("puts(1)", Some("2.7.8"), "t", &budgets) {
        Err(FrontendError::ReleaseNotAvailable { stage, .. }) => {
            assert_eq!(stage, "no grammar pair: grammar_stage none");
        }
        other => panic!("2.7.8: {other:?}"),
    }
    // A regeneration-bug phrase already says what is wrong and is not
    // relabelled as a `grammar_stage`.
    let bug = FrontendError::from(coding_adventures_ruby_parser::RubyFrontendError::ReleaseNotAvailable {
        release: "3.3.6".into(),
        stage: "token table missing (regenerate ruby-lexer)".into(),
    });
    assert!(
        matches!(&bug, FrontendError::ReleaseNotAvailable { stage, .. } if stage == "token table missing (regenerate ruby-lexer)"),
        "{bug:?}"
    );
    assert!(matches!(
        compile_source_at("puts(1)", Some(""), "t", &budgets),
        Err(FrontendError::UnknownRelease { .. })
    ));
    let error = compile_source_at("puts(1)", Some("3.3\n\u{1b}[2J"), "t", &budgets).unwrap_err();
    assert!(!error.to_string().chars().any(char::is_control), "{error}");
}

#[test]
fn budgets_are_typed_errors() {
    let tight = FrontendBudgets {
        max_source_bytes: 8,
        ..FrontendBudgets::default()
    };
    assert!(matches!(
        compile_source_at("puts(1 + 2)", None, "t", &tight),
        Err(FrontendError::Budget { which: "source bytes", .. })
    ));
    assert!(matches!(
        compile_source(&" ".repeat(MAX_SOURCE_BYTES + 1), "t"),
        Err(FrontendError::Budget { .. })
    ));
    assert!(matches!(
        compile_source("puts(1)", &"m".repeat(MAX_AST_FIELD_BYTES + 1)),
        Err(FrontendError::Budget { which: "module name bytes", .. })
    ));
    // A deep but legal nesting hits the parser's depth budget, not the stack.
    let deep = format!("puts({}1{})", "(".repeat(5_000), ")".repeat(5_000));
    assert_eq!(error(&deep).category(), "Budget");
}

// ---------------------------------------------------------------------------
// Caller-supplied trees: shape and token type, never spelling alone
// ---------------------------------------------------------------------------

/// Every token in `node`, depth first, mutably.
fn tokens_mut(node: &mut GrammarASTNode) -> Vec<&mut lexer::token::Token> {
    let mut out = Vec::new();
    for child in &mut node.children {
        match child {
            ASTNodeOrToken::Token(token) => out.push(token),
            ASTNodeOrToken::Node(inner) => out.extend(tokens_mut(inner)),
        }
    }
    out
}

/// The child-index path of every node in `root`, the root's (empty) path last.
fn node_paths(root: &GrammarASTNode) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    for (index, child) in root.children.iter().enumerate() {
        if let ASTNodeOrToken::Node(inner) = child {
            for mut path in node_paths(inner) {
                path.insert(0, index);
                out.push(path);
            }
        }
    }
    out.push(Vec::new());
    out
}

fn node_at_mut<'a>(root: &'a mut GrammarASTNode, path: &[usize]) -> &'a mut GrammarASTNode {
    let mut node = root;
    for &index in path {
        let ASTNodeOrToken::Node(inner) = &mut node.children[index] else {
            panic!("path leads through a token");
        };
        node = inner;
    }
    node
}

#[test]
fn forged_token_types_and_spellings_are_refused() {
    let source = "puts(1 + 2 * (3 - 4) / 5 % 6, -7)";
    let original = tree(source);
    assert!(compile_ast(&original, "t").is_ok());
    let count = tokens_mut(&mut original.clone()).len();
    let mut refused = 0;
    for index in 0..count {
        for mutation in 0..3 {
            let mut forged = original.clone();
            let mut tokens = tokens_mut(&mut forged);
            let token = &mut tokens[index];
            match mutation {
                // A STRING (or any other type) spelling the same text.
                0 => token.type_name = Some("STRING".into()),
                // Retype as an operator of the other precedence level (a
                // `sum` operator never appears in a `term`, nor the reverse),
                // or a non-operator as `PLUS`.
                1 => {
                    let other = match token.effective_type_name() {
                        "PLUS" | "MINUS" => "STAR",
                        _ => "PLUS",
                    };
                    token.type_name = Some(other.into());
                }
                // Same type, different spelling.
                _ => token.value = format!("{}x", token.value),
            }
            let accepted = compile_ast(&forged, "t");
            // Mutation 2 on an operator or delimiter keeps its type; the
            // walker dispatches on type, so a respelled `+` is still `+`.
            let kind = tokens_mut(&mut original.clone())[index].effective_type_name().to_string();
            if mutation == 2 && !matches!(kind.as_str(), "NAME" | "NUMBER") {
                assert!(accepted.is_ok(), "token {index} ({kind}) mutation {mutation}");
                continue;
            }
            assert!(accepted.is_err(), "token {index} ({kind}) mutation {mutation} accepted");
            refused += 1;
        }
    }
    assert!(refused >= 2 * count, "{refused} of {count} tokens");
}

#[test]
fn forged_shapes_are_refused() {
    let original = tree("puts(1 + 2, (3))");
    for path in node_paths(&original) {
        let rule = node_at_mut(&mut original.clone(), &path).rule_name.clone();
        for mutation in 0..3 {
            let mut forged = original.clone();
            let node = node_at_mut(&mut forged, &path);
            match mutation {
                0 => node.rule_name = format!("{}_x", node.rule_name),
                1 => node.children.clear(),
                _ => {
                    let extra = node.children[0].clone();
                    node.children.push(extra);
                }
            }
            let result = compile_ast(&forged, "t");
            // Clearing `program` leaves an empty program, and duplicating
            // its first child repeats a statement: both are valid Ruby.
            if rule == "program" && mutation > 0 {
                assert!(result.is_ok(), "program mutation {mutation}");
                continue;
            }
            assert!(result.is_err(), "{path:?} ({rule}) mutation {mutation} accepted");
        }
    }
}

#[test]
fn caller_supplied_trees_are_measured_before_they_are_walked() {
    fn node(rule: &str, children: Vec<ASTNodeOrToken>) -> GrammarASTNode {
        GrammarASTNode {
            rule_name: rule.into(),
            children,
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        }
    }
    let budget = |ast: &GrammarASTNode| match compile_ast(ast, "t") {
        Err(FrontendError::Budget { which, .. }) => which,
        other => panic!("{other:?}"),
    };

    let wide = node(
        "program",
        (0..MAX_AST_ITEMS).map(|_| ASTNodeOrToken::Node(node("statement", vec![]))).collect(),
    );
    assert_eq!(budget(&wide), "CST items");

    let mut deep = node("expression", vec![]);
    for _ in 0..MAX_AST_DEPTH {
        deep = node("expression", vec![ASTNodeOrToken::Node(deep)]);
    }
    assert_eq!(budget(&node("program", vec![ASTNodeOrToken::Node(deep)])), "CST depth");

    let mut huge = tree("puts(1)");
    tokens_mut(&mut huge)[2].value = "9".repeat(MAX_AST_FIELD_BYTES + 1);
    assert_eq!(budget(&huge), "CST field bytes");
    let mut cv = tree("puts(1)");
    tokens_mut(&mut cv)[2].cv = Some("c".repeat(MAX_AST_FIELD_BYTES + 1));
    assert_eq!(budget(&cv), "CST field bytes");

    let heavy = node(
        "program",
        (0..18).map(|_| ASTNodeOrToken::Node(node(&"r".repeat(60_000), vec![]))).collect(),
    );
    assert_eq!(budget(&heavy), "CST text bytes");
}

#[test]
fn a_hostile_number_token_is_refused_without_echoing_it_whole() {
    for value in ["", "_", "1__0", "_1", "1_", "0x10", "1e5", "--1", "+", "010", "١"] {
        let mut forged = tree("puts(1)");
        tokens_mut(&mut forged)[2].value = value.into();
        let error = compile_ast(&forged, "t").expect_err(value);
        assert_eq!(error.category(), "Unsupported", "{value:?}");
    }
    let mut forged = tree("puts(1)");
    tokens_mut(&mut forged)[2].value = format!("1\n{}", "9".repeat(10_000));
    let message = compile_ast(&forged, "t").unwrap_err().to_string();
    assert!(message.len() < 300 && !message.contains('\n'), "{message}");
}
