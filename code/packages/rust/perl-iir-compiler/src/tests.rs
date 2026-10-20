//! Unit tests for the Perl IIR compiler: lowering, release selection,
//! budgets, typed errors, and the direct-AST bounds.

use super::*;
use coding_adventures_perl_parser::{parse_perl, SUPPORTED_RELEASES};
use interpreter_ir::op_registry::{registry_entry, OpKind};

/// The programs the subset is defined by. Each is accepted by every checked
/// release pair, so the cross-release identity test below can use all of them.
const CORPUS: &[(&str, &str)] = &[
    ("print(1 + 2);", "3"),
    ("print(7 - 2 * 3);", "1"),
    ("print(-4); print(2);", "-42"),
    ("print((1 + 2) * 3);", "9"),
    ("print(1- -2);", "3"),
    ("print(10 - 3 - 2);", "5"),
    (
        "print(2147483647); print(-2147483647 - 1);",
        "2147483647-2147483648",
    ),
    ("print(46340 * 46340);", "2147395600"),
];

fn ops(module: &IIRModule) -> Vec<String> {
    module.functions[0]
        .instructions
        .iter()
        .map(|instruction| instruction.op.clone())
        .collect()
}

#[test]
fn source_executes_on_native_vm() {
    for (source, expected) in CORPUS {
        assert_eq!(run_source(source).unwrap(), *expected, "{source}");
    }
}

#[test]
fn print_lowers_to_the_lang83_write_i64_builtin() {
    let module = compile_source("print(1 + 2 * 3);", "ir").unwrap();
    let calls: Vec<_> = module.functions[0]
        .instructions
        .iter()
        .filter(|instruction| instruction.op == "call_builtin")
        .collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].srcs[0], Operand::Var("write_i64".into()));
    let ops = ops(&module);
    assert!(ops.contains(&"mul".to_string()));
    assert!(ops.contains(&"add".to_string()));
}

/// LANG84 C7: every emitted instruction is a LANG83 registry builtin, or one
/// of the pre-registry scalar mnemonics the registry does not gate. A
/// language-named helper such as the retired `pl_print_int` fails here.
#[test]
fn every_instruction_is_a_registry_builtin_or_an_ungated_scalar_op() {
    for (source, _) in CORPUS {
        let module = compile_source(source, "c7").unwrap();
        for instruction in &module.functions[0].instructions {
            if instruction.op == "call_builtin" {
                let entry = registry_entry(instruction)
                    .unwrap_or_else(|| panic!("{source}: non-registry builtin {instruction:?}"));
                assert_eq!(entry.kind, OpKind::Builtin);
            } else {
                assert!(
                    ["const", "neg", "add", "sub", "mul", "ret"]
                        .contains(&instruction.op.as_str()),
                    "{source}: unexpected op {}",
                    instruction.op
                );
            }
        }
    }
}

/// The checked release pairs differ only in lexical details outside this
/// corpus (carriage returns, digit limits), so every one of them must lower
/// the corpus to IIR identical to the default's. A grammar that changed a
/// rule name or child shape the compiler depends on fails here (LANG84 C8).
#[test]
fn every_supported_release_lowers_the_corpus_identically() {
    let budgets = FrontendBudgets::default();
    for (source, _) in CORPUS {
        let expected = compile_source_at(source, None, "same", &budgets).unwrap();
        for release in SUPPORTED_RELEASES {
            let actual = compile_source_at(source, Some(release), "same", &budgets)
                .unwrap_or_else(|error| panic!("{release}: {source}: {error}"));
            assert_eq!(
                format!("{:?}", actual.functions),
                format!("{:?}", expected.functions),
                "{release}: {source}"
            );
        }
    }
}

#[test]
fn every_supported_release_runs_on_the_native_vm() {
    for release in SUPPORTED_RELEASES {
        assert_eq!(
            run_source_at("print(-4); print(2);", Some(release)).unwrap(),
            "-42",
            "{release}"
        );
    }
}

/// Release selection is real: the same bytes lex differently under
/// different releases, exactly as each pair's own source says they should.
#[test]
fn the_selected_release_decides_what_lexes() {
    let budgets = FrontendBudgets::default();
    // 5.003_26 through 5.004_71 refuse a carriage return; 5.004_72's
    // TMP_CRLF_PATCH makes it whitespace; the earliest pairs accept it too.
    let crlf = "print(1);\r\nprint(2);\r\n";
    assert!(matches!(
        compile_source_at(crlf, Some("5.003_26"), "cr", &budgets),
        Err(FrontendError::Lex { line: 1, .. })
    ));
    for release in ["1.000", "5.004_72"] {
        assert!(
            compile_source_at(crlf, Some(release), "cr", &budgets).is_ok(),
            "{release}"
        );
    }
    // From 5.003_97i an integer literal is limited to 250 digits; the older
    // scanners have no such bound, so the literal lexes and is then refused
    // as out of range by the compiler instead.
    let long = format!("print({});", "1".repeat(251));
    assert!(matches!(
        compile_source_at(&long, Some("5.004_72"), "long", &budgets),
        Err(FrontendError::Lex { .. } | FrontendError::Parse { .. })
    ));
    assert!(matches!(
        compile_source_at(&long, Some("1.000"), "long", &budgets),
        Err(FrontendError::Unsupported { .. })
    ));
}

#[test]
fn release_errors_are_typed_and_never_fall_back() {
    let budgets = FrontendBudgets::default();
    assert!(matches!(
        compile_source_at("print(1);", Some("1.001"), "m", &budgets),
        Err(FrontendError::ReleaseNotAvailable {
            stage: "pending",
            ..
        })
    ));
    assert!(matches!(
        compile_source_at("print(1);", Some(""), "m", &budgets),
        Err(FrontendError::UnknownRelease { .. })
    ));
    let error = compile_source_at("print(1);", Some("5.38"), "m", &budgets).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("nearest inventory IDs: 5.38.0-RC1"),
        "{error}"
    );
}

#[test]
fn every_error_after_resolution_names_its_release_and_position() {
    let budgets = FrontendBudgets::default();
    let lex =
        compile_source_at("print(1);\n  print(@);", Some("5.004_72"), "m", &budgets).unwrap_err();
    let FrontendError::Lex {
        release,
        line,
        column,
        ..
    } = &lex
    else {
        panic!("expected a lex error, got {lex:?}");
    };
    assert_eq!((release.as_str(), *line, *column), ("Perl 5.004_72", 2, 9));

    let parse = compile_source_at("print(1 2);", None, "m", &budgets).unwrap_err();
    let FrontendError::Parse {
        release,
        line,
        column,
        ..
    } = &parse
    else {
        panic!("expected a parse error, got {parse:?}");
    };
    assert_eq!((release.as_str(), *line), ("unversioned LANG81 pair", 1));
    assert!(*column >= 9, "{parse}");

    let unsupported =
        compile_source_at("print(50000 * 50000);", Some("1.000"), "m", &budgets).unwrap_err();
    let FrontendError::Unsupported {
        release,
        rule,
        start,
        ..
    } = &unsupported
    else {
        panic!("expected an unsupported error, got {unsupported:?}");
    };
    assert_eq!(release, "Perl 1.000");
    assert_eq!(rule.as_deref(), Some("term"));
    assert_eq!(*start, Some((1, 7)));
    assert!(
        unsupported.to_string().contains("rule term at 1:7"),
        "{unsupported}"
    );
}

#[test]
fn budgets_are_typed_errors() {
    let tight = FrontendBudgets {
        source_bytes: 8,
        ..FrontendBudgets::default()
    };
    assert_eq!(
        compile_source_at("print(12);", Some("1.000"), "m", &tight).unwrap_err(),
        FrontendError::Budget {
            release: "Perl 1.000".into(),
            which: "source bytes",
            limit: 8
        }
    );
    let shallow = FrontendBudgets {
        parser_depth: 8,
        ..FrontendBudgets::default()
    };
    assert!(matches!(
        compile_source_at("print(((((((1)))))));", None, "m", &shallow),
        Err(FrontendError::Parse { .. })
    ));
    let few = FrontendBudgets {
        iir_instructions: 3,
        ..FrontendBudgets::default()
    };
    assert!(matches!(
        compile_source_at("print(1); print(2 + 3);", None, "m", &few),
        Err(FrontendError::Budget {
            which: "IIR instructions",
            limit: 3,
            ..
        })
    ));
}

#[test]
fn rejects_unsupported_or_out_of_range_source() {
    for source in [
        "print(x);",
        "print(1 / 2);",
        "print(1, 2);",
        "print(2147483648);",
        "print(2147483647 + 1);",
        "print(50000 * 50000);",
        "print(-2147483647 - 2);",
    ] {
        assert!(compile_source(source, "negative").is_err(), "{source}");
    }
}

#[test]
fn legacy_octal_is_not_lowered_as_decimal() {
    for source in ["print(010);", "print(08);", "print(09);"] {
        assert!(compile_source(source, "octal").is_err(), "{source}");
    }
}

#[test]
fn decrement_is_not_two_unary_minuses() {
    assert!(compile_source("print(1--2);", "decrement").is_err());
    assert_eq!(run_source("print(1- -2);").unwrap(), "3");
}

#[test]
fn deep_unary_minus_is_refused_without_overflowing() {
    let source = format!("print({}1);", "- ".repeat(10_000));
    assert!(compile_source(&source, "deep").is_err());
}

fn empty_node(rule_name: &str) -> GrammarASTNode {
    GrammarASTNode {
        rule_name: rule_name.into(),
        children: vec![],
        start_line: None,
        start_column: None,
        end_line: None,
        end_column: None,
    }
}

#[test]
fn direct_ast_budget_rejects_deep_and_wide_input() {
    let mut deep = empty_node("leaf");
    for _ in 0..MAX_AST_DEPTH {
        let mut parent = empty_node("wrapper");
        parent.children.push(ASTNodeOrToken::Node(deep));
        deep = parent;
    }
    deep.rule_name = "program".into();
    assert!(compile_ast(&deep, "deep")
        .unwrap_err()
        .contains("depth limit"));

    let mut wide = empty_node("program");
    wide.children = (0..MAX_AST_ITEMS)
        .map(|_| ASTNodeOrToken::Node(empty_node("print_statement")))
        .collect();
    assert!(compile_ast(&wide, "wide")
        .unwrap_err()
        .contains("item limit"));
}

#[test]
fn direct_ast_text_and_module_name_are_bounded() {
    let mut oversized_token = empty_node("program");
    let mut parsed = parse_perl("print(1);").unwrap();
    let ASTNodeOrToken::Node(statement) = &mut parsed.children[0] else {
        panic!("expected statement");
    };
    let ASTNodeOrToken::Token(token) = &mut statement.children[0] else {
        panic!("expected print token");
    };
    token.value = "9".repeat(64 * 1024 + 1);
    oversized_token.children.push(statement.children[0].clone());
    assert!(compile_ast(&oversized_token, "text")
        .unwrap_err()
        .contains("text limit"));

    let mut cv_tree = parse_perl("print(1);").unwrap();
    let ASTNodeOrToken::Node(statement) = &mut cv_tree.children[0] else {
        panic!("expected statement");
    };
    let ASTNodeOrToken::Token(token) = &mut statement.children[0] else {
        panic!("expected print token");
    };
    token.cv = Some("c".repeat(64 * 1024 + 1));
    assert!(compile_ast(&cv_tree, "cv")
        .unwrap_err()
        .contains("text limit"));

    let mut type_tree = parse_perl("print(1);").unwrap();
    let ASTNodeOrToken::Node(statement) = &mut type_tree.children[0] else {
        panic!("expected statement");
    };
    let ASTNodeOrToken::Token(token) = &mut statement.children[0] else {
        panic!("expected print token");
    };
    token.type_name = Some("T".repeat(64 * 1024 + 1));
    assert!(compile_ast(&type_tree, "type")
        .unwrap_err()
        .contains("text limit"));

    let mut aggregate = empty_node("program");
    aggregate.children = (0..18)
        .map(|_| {
            let mut child = empty_node("r");
            child.rule_name = "r".repeat(60_000);
            ASTNodeOrToken::Node(child)
        })
        .collect();
    assert!(compile_ast(&aggregate, "aggregate")
        .unwrap_err()
        .contains("text limit"));

    assert!(
        compile_ast(&empty_node("program"), &"m".repeat(64 * 1024 + 1))
            .unwrap_err()
            .contains("module name")
    );
}
