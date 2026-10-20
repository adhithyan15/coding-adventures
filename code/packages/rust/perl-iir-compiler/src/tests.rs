//! Unit tests for the Perl IIR compiler: lowering, release selection,
//! budgets, typed errors, the direct-AST bounds, the LANG84 C8 CST contract
//! (`cst_contract`), and the C5 hostile-input probes, which run in a child
//! process (`hostile_probes`).

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
            // Same code, different provenance: the module names its release.
            assert_eq!(actual.release.as_deref(), Some(*release));
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
    for release in [Some("1.000"), Some("5.004_72"), Some("5.38.0"), None] {
        assert!(
            compile_source_at(crlf, release, "cr", &budgets).is_ok(),
            "{release:?}"
        );
    }
    // From 5.003_97i an integer literal is limited to 250 digits; the older
    // scanners have no such bound, so the literal lexes and is then refused
    // as out of range by the compiler instead.
    let long = format!("print({});", "1".repeat(251));
    for release in [Some("5.004_72"), Some("5.38.0"), Some("5.38.2"), None] {
        assert!(
            matches!(
                compile_source_at(&long, release, "long", &budgets),
                Err(FrontendError::Lex { .. } | FrontendError::Parse { .. })
            ),
            "{release:?}"
        );
    }
    assert!(matches!(
        compile_source_at(&long, Some("1.000"), "long", &budgets),
        Err(FrontendError::Unsupported { .. })
    ));
    // 5.44.0 and 5.45.3 have a 1024-byte tokenbuf (LANG84 4c): 1018 digits
    // lex (and are then out of range), the 1019th does not.
    for release in ["5.44.0", "5.45.3"] {
        for (digits, lexes) in [(251, true), (1018, true), (1019, false)] {
            let source = format!("print({});", "1".repeat(digits));
            let outcome = compile_source_at(&source, Some(release), "long", &budgets);
            assert_eq!(
                matches!(outcome, Err(FrontendError::Unsupported { .. })),
                lexes,
                "{release} {digits}: {outcome:?}"
            );
            assert!(outcome.is_err());
        }
    }
}

/// The one profile switch (`profiles::Arithmetic`): under C-double arithmetic
/// a printed IEEE negative zero is refused, because those releases print it
/// as `0` or `-0` depending on folding and the platform. Every rule of
/// `negative_zero_after` is exercised, and both directions of each.
#[test]
fn a_printed_negative_zero_is_refused_only_under_double_arithmetic() {
    let budgets = FrontendBudgets::default();
    // Each of these is `-0.0` when evaluated in C doubles.
    let negative = [
        "print(-0);",
        "print(-(0));",
        "print(- - -0);",
        "print(0 * -3);",
        "print(-3 * 0);",
        "print(-0 * 3);",
        "print(-0 + -0);",
        "print(-0 - 0);",
        "print(-(3 - 3));",
        "print(1); print(2 * (0 * -1));",
    ];
    // And each of these is `+0.0`, so every release prints `0`.
    let positive = [
        "print(0);",
        "print(- -0);",
        "print(-0 + 0);",
        "print(0 + -0);",
        "print(0 - -0);",
        "print(-0 - -0);",
        "print(3 - 3);",
        "print(-3 + 3);",
        "print(-0 * -3);",
        "print(0 * 3);",
        "print(-(0 * -1));",
    ];
    for release in ["1.000", "5.000", "5.004_72", "5.004_05"] {
        assert_eq!(profile_for(release), Some(&profiles::PERL_NV), "{release}");
        for source in negative {
            match compile_source_at(source, Some(release), "z", &budgets) {
                Err(FrontendError::Unsupported { release: r, rule, message, .. }) => {
                    assert_eq!(r, format!("Perl {release}"));
                    assert_eq!(rule.as_deref(), Some(cst::EXPR), "{source}");
                    assert!(message.contains("-0.0") && message.contains("perl_nv"), "{message}");
                }
                other => panic!("{release} {source}: {other:?}"),
            }
        }
        for source in positive {
            let module = compile_source_at(source, Some(release), "z", &budgets)
                .unwrap_or_else(|error| panic!("{release} {source}: {error}"));
            assert_eq!(run_source_at(source, Some(release)).unwrap(), "0", "{source}");
            // The profile decides acceptance, never the code: the same
            // program lowers identically under the integer profile.
            let integer = compile_source_at(source, Some("5.38.0"), "z", &budgets).unwrap();
            assert_eq!(format!("{:?}", module.functions), format!("{:?}", integer.functions));
        }
    }
    for release in ["5.38.0", "5.38.2", "5.44.0", "5.45.3"] {
        assert_eq!(profile_for(release), Some(&profiles::PERL_IV), "{release}");
        for source in negative {
            let printed = run_source_at(source, Some(release)).unwrap();
            assert_eq!(printed.trim_start_matches('1'), "0", "{release} {source}");
        }
    }
}

/// LANG84 C4.1/C4.4/C4.5: naming no release compiles against the inventory
/// release `5.38.0`, the module says so, and the unversioned LANG81 pair is
/// not what ran. The two differ observably: `perl5.38.0.tokens` keeps
/// `scan_num`'s 250-digit bound, while `perl.tokens` has none, so a
/// 251-digit literal is rejected before lowering only under the release pair.
#[test]
fn the_default_is_the_5_38_0_inventory_pair_and_is_recorded() {
    let budgets = FrontendBudgets::default();
    let module = compile_source_at("print(1);", None, "d", &budgets).unwrap();
    assert_eq!(module.release.as_deref(), Some("5.38.0"));
    assert_eq!(
        compile_source("print(1);", "d").unwrap().release.as_deref(),
        Some(DEFAULT_RELEASE_ID)
    );
    let long = format!("print({});", "7".repeat(251));
    assert!(parse_perl(&long).is_ok(), "the unversioned pair has no bound");
    assert!(matches!(
        compile_source_at(&long, None, "d", &budgets),
        Err(FrontendError::Lex { ref release, .. } | FrontendError::Parse { ref release, .. })
            if release == "Perl 5.38.0"
    ));
    // A caller-supplied tree is compiled "as" the release it is given.
    let tree = parse_perl("print(2);").unwrap();
    let explicit = resolve_release(Some("5.004_72")).unwrap();
    let module = compile_ast_at(&tree, explicit, "t", &budgets).unwrap();
    assert_eq!(module.release.as_deref(), Some("5.004_72"));
    assert_eq!(
        compile_ast(&tree, "t").unwrap().release.as_deref(),
        Some("5.38.0")
    );
}

/// Untrusted source reaches the terminal only escaped: the lexer reports an
/// offending ESC or bidi override as `\u{..}` text, never the character.
#[test]
fn lex_errors_never_echo_control_or_bidi_characters() {
    let budgets = FrontendBudgets::default();
    for source in ["print(1);\u{1b}[2J", "print(\u{202e}1);", "print(1);\r\u{7}"] {
        let message = compile_source_at(source, None, "m", &budgets).unwrap_err().to_string();
        assert!(
            !message.chars().any(|c| c.is_control() || ('\u{202a}'..='\u{202e}').contains(&c)),
            "{message:?}"
        );
    }
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
    assert_eq!((release.as_str(), *line), ("Perl 5.38.0", 1));
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

/// LANG84 C8: for every checked release, every name the compiler dispatches
/// on exists in that release's compiled grammar, and the subset corpus
/// parses into exactly the shapes the compiler destructures.
///
/// Perl's `releases.csv` has had the `iir_status` column since the LANG84
/// phase 2b migration, but every row is still `-`: the `pl-int-print`
/// corpus and the `release_subsets` test that would prove stage I arrive in
/// phase 4c. Until then "every release with a non-empty `iir_status`" is
/// read as every checked (non-pending) release: all of them already lower
/// the corpus (`every_supported_release_lowers_the_corpus_identically`).
mod cst_contract {
    use super::*;
    use coding_adventures_perl_lexer::release_token_grammar;
    use coding_adventures_perl_parser::release_parser_grammar;
    use lexer::token::TokenType;
    use parser::grammar_parser::GrammarParser;

    #[test]
    fn every_dispatched_rule_and_token_exists_in_every_checked_release() {
        for release in SUPPORTED_RELEASES {
            let parser_grammar = release_parser_grammar(release).unwrap();
            for rule in CST_RULES {
                assert!(
                    parser_grammar.rules.iter().any(|r| r.name == *rule),
                    "{release}: grammar has no rule {rule:?} the compiler dispatches on"
                );
            }
            let token_grammar = release_token_grammar(release).unwrap();
            for token in CST_TOKEN_TYPES {
                assert!(
                    token_grammar.definitions.iter().any(|d| d.name == *token),
                    "{release}: tokens have no {token:?} the compiler dispatches on"
                );
            }
            for spelling in CST_TOKEN_SPELLINGS {
                let tokens: Vec<_> = GrammarLexer::new(spelling, &token_grammar)
                    .tokenize()
                    .unwrap_or_else(|e| panic!("{release}: {spelling:?} does not lex: {e}"))
                    .into_iter()
                    .filter(|token| token.type_ != TokenType::Eof)
                    .collect();
                assert!(
                    matches!(tokens.as_slice(), [token] if token.value == *spelling),
                    "{release}: {spelling:?} is not one token: {tokens:?}"
                );
            }
        }
    }

    fn token_is(child: &ASTNodeOrToken, value: &str) -> bool {
        matches!(child, ASTNodeOrToken::Token(token) if token.value == value)
    }

    fn node<'a>(child: &'a ASTNodeOrToken, rule: &str) -> Option<&'a GrammarASTNode> {
        match child {
            ASTNodeOrToken::Node(inner) if inner.rule_name == rule => Some(inner),
            _ => None,
        }
    }

    /// Assert the shape the compiler destructures, rule by rule. Iterative,
    /// so a deep corpus case cannot overflow the test's own stack.
    fn assert_shape(release: &str, root: &GrammarASTNode) {
        assert_eq!(root.rule_name, cst::PROGRAM, "{release}");
        assert!(!root.children.is_empty(), "{release}: empty program");
        let mut pending: Vec<&GrammarASTNode> = vec![root];
        while let Some(current) = pending.pop() {
            let children = current.children.as_slice();
            let ok = match current.rule_name.as_str() {
                cst::PROGRAM => children
                    .iter()
                    .all(|c| node(c, cst::PRINT_STATEMENT).is_some()),
                cst::PRINT_STATEMENT => matches!(
                    children,
                    [p, o, e, c, s] if token_is(p, "print") && token_is(o, "(")
                        && node(e, cst::EXPR).is_some() && token_is(c, ")") && token_is(s, ";")
                ),
                cst::EXPR | cst::TERM => {
                    let (operand, operators): (&str, &[&str]) = if current.rule_name == cst::EXPR
                    {
                        (cst::TERM, &["+", "-"])
                    } else {
                        (cst::UNARY, &["*"])
                    };
                    children.len() % 2 == 1
                        && children.iter().enumerate().all(|(index, child)| {
                            if index % 2 == 0 {
                                node(child, operand).is_some()
                            } else {
                                operators.iter().any(|op| token_is(child, op))
                            }
                        })
                }
                cst::UNARY => match children {
                    [a] => node(a, cst::ATOM).is_some(),
                    [m, u] => token_is(m, "-") && node(u, cst::UNARY).is_some(),
                    _ => false,
                },
                cst::ATOM => match children {
                    [ASTNodeOrToken::Token(int)] => int.type_name.as_deref() == Some(cst::INT),
                    [o, e, c] => token_is(o, "(") && node(e, cst::EXPR).is_some() && token_is(c, ")"),
                    _ => false,
                },
                other => panic!("{release}: rule {other:?} is outside the CST contract"),
            };
            assert!(ok, "{release}: unexpected {} shape: {current:?}", current.rule_name);
            pending.extend(children.iter().filter_map(|child| match child {
                ASTNodeOrToken::Node(inner) => Some(inner),
                ASTNodeOrToken::Token(_) => None,
            }));
        }
    }

    #[test]
    fn the_corpus_parses_into_the_contract_shapes_under_every_checked_release() {
        for release in SUPPORTED_RELEASES {
            let token_grammar = release_token_grammar(release).unwrap();
            let parser_grammar = release_parser_grammar(release).unwrap();
            for (source, _) in CORPUS {
                let tokens = GrammarLexer::new(source, &token_grammar).tokenize().unwrap();
                let tree = GrammarParser::new(tokens, parser_grammar.clone())
                    .with_max_depth(MAX_RULE_DEPTH)
                    .parse()
                    .unwrap_or_else(|e| panic!("{release}: {source}: {e:?}"));
                assert_shape(release, &tree);
            }
        }
    }
}

/// LANG84 C5: hostile source must end in a typed error (or a bounded,
/// correct result), never a panic, abort or stack overflow. A stack overflow
/// aborts the whole process, which would take the test harness down with it
/// and look like a crash of "some test". So the probes run in a child copy
/// of this test binary: the parent sees the child's exit status, and an
/// abort (a signal, on Unix) is reported as exactly that.
///
/// ```text
///   parent test ──spawn──► this binary --exact <child> --ignored
///        │                    (PERL_IIR_HOSTILE_PROBE_CHILD=1)
///        │                    runs every probe, prints DONE_MARKER
///        ▼
///   success + DONE_MARKER   → pass
///   killed by a signal      → "aborted" (stack overflow, abort)
///   non-zero exit           → a probe's assertion failed
/// ```
mod hostile_probes {
    use super::*;
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    const CHILD_ENV: &str = "PERL_IIR_HOSTILE_PROBE_CHILD";
    const CHILD_TEST: &str = "tests::hostile_probes::child_runs_every_probe";
    const DONE_MARKER: &str = "perl-iir hostile probes passed:";

    /// What a probe must end in.
    #[derive(Debug)]
    enum Expect {
        Lex,
        Parse,
        Unsupported,
        Budget(&'static str),
        /// Compiles, with exactly this many IIR instructions (bounded work).
        Compiles(usize),
    }

    fn check(name: &str, outcome: Result<IIRModule, FrontendError>, expect: &Expect) {
        let ok = match (&outcome, expect) {
            (Err(FrontendError::Lex { .. }), Expect::Lex)
            | (Err(FrontendError::Parse { .. }), Expect::Parse)
            | (Err(FrontendError::Unsupported { .. }), Expect::Unsupported) => true,
            (Err(FrontendError::Budget { which, .. }), Expect::Budget(expected)) => {
                which == expected
            }
            (Ok(module), Expect::Compiles(count)) => {
                module.functions[0].instructions.len() == *count
            }
            _ => false,
        };
        let shown = match &outcome {
            Ok(module) => format!("Ok({} instructions)", module.functions[0].instructions.len()),
            // Errors can quote kilobytes of source; keep the report short.
            Err(error) => error.to_string().chars().take(160).collect(),
        };
        assert!(ok, "{name}: expected {expect:?}, got {shown}");
    }

    /// `main`'s closing `const 0; ret` (it returns the exit status 0 since
    /// 4a's CLR/BEAM fix; it was a single `ret_void` when these counts were
    /// first written).
    const EPILOGUE: usize = 2;

    #[test]
    #[ignore = "run in a child process by hostile_inputs_end_in_typed_errors_in_a_child_process"]
    fn child_runs_every_probe() {
        if std::env::var_os(CHILD_ENV).is_none() {
            return;
        }
        let budgets = FrontendBudgets::default();
        let limit = budgets.source_bytes;
        let mut count = 0;
        let mut probe = |name: &str, source: &str, release: Option<&str>, expect: Expect| {
            assert!(source.len() <= limit + 1, "{name}: probe exceeds the source budget");
            check(name, compile_source_at(source, release, "hostile", &budgets), &expect);
            count += 1;
        };
        let nest = |depth: usize| format!("print({}1{});", "(".repeat(depth), ")".repeat(depth));
        for release in [None, Some("1.000"), Some("5.004_72")] {
            // Deep nesting: bounded by the parser's rule-depth cap.
            probe("deep parentheses", &nest(30_000), release, Expect::Parse);
            probe("deep unary minus", &format!("print({}1);", "- ".repeat(30_000)), release, Expect::Parse);
            // Unterminated constructs.
            for source in ["print", "print(", "print(1", "print(1 +", "print(1);print(", "print((((((((1"] {
                probe("unterminated", source, release, Expect::Parse);
            }
            probe("unterminated deep", &"print(".repeat(10_000), release, Expect::Parse);
            // Long flat operator chains: the grammar's `{ }` repetition is
            // not depth-bounded, and the compiler folds it iteratively. A
            // chain within the CST item budget (16,384) compiles with work
            // linear in its length; a longer one is refused by that budget
            // before any lowering.
            for op in ["+", "*", "-"] {
                let short = 2_000;
                probe("flat chain", &format!("print({}1);", format!("1{op}").repeat(short)),
                    release, Expect::Compiles(2 * short + 1 + 1 + EPILOGUE));
                probe("long flat chain", &format!("print({}1);", format!("1{op}").repeat(20_000)),
                    release, Expect::Unsupported);
            }
            // Adjacent minus signs lex as decrement tokens, which no rule accepts.
            probe("decrement run", &format!("print(1{}1);", "-".repeat(40_000)), release, Expect::Parse);
            // Many statements: the same item budget bounds the program width.
            probe("statements", &"print(1);".repeat(1_000), release, Expect::Compiles(2 * 1_000 + EPILOGUE));
            probe("statement flood", &"print(1);".repeat(7_000), release, Expect::Unsupported);
            // Characters outside every token rule.
            for source in ["print(1);\0", "print(\u{FF11});", "print(1);\u{1b}[2J", "\u{FEFF}print(1);"] {
                probe("foreign character", source, release, Expect::Lex);
            }
            // The source budget is checked before lexing.
            probe("oversized source", &format!("print(1);{}", " ".repeat(limit - 8)), release,
                Expect::Budget("source bytes"));
        }
        // The 250-digit decimal bound (5.003_97i onwards, 5.38.0 included)
        // and the unbounded early scanners, whose huge literal is refused by
        // the compiler's range check instead.
        for release in [None, Some("5.004_72")] {
            // INT takes the first 250 digits and the 251st becomes a second
            // INT, which no rule accepts after an INT.
            probe("251 digits", &format!("print({});", "9".repeat(251)), release, Expect::Parse);
            probe("250 digits", &format!("print({});", "9".repeat(250)), release, Expect::Unsupported);
        }
        probe("60,000 digits", &format!("print({});", "9".repeat(60_000)), Some("1.000"), Expect::Unsupported);
        // A tight instruction budget stops a long chain with a typed error.
        let tight = FrontendBudgets { iir_instructions: 100, ..FrontendBudgets::default() };
        check(
            "instruction budget",
            compile_source_at(&"print(1);".repeat(1_000), None, "hostile", &tight),
            &Expect::Budget("IIR instructions"),
        );
        count += 1;
        // Caller-supplied trees: deep and wide, bounded before any recursion.
        let mut deep = empty_node("wrapper");
        for _ in 0..2_000 {
            let mut parent = empty_node("wrapper");
            parent.children.push(ASTNodeOrToken::Node(deep));
            deep = parent;
        }
        deep.rule_name = cst::PROGRAM.into();
        check("deep caller tree", compile_ast_at(&deep, DEFAULT_RELEASE, "hostile", &budgets), &Expect::Unsupported);
        let mut wide = empty_node(cst::PROGRAM);
        wide.children = (0..100_000)
            .map(|_| ASTNodeOrToken::Node(empty_node(cst::PRINT_STATEMENT)))
            .collect();
        check("wide caller tree", compile_ast_at(&wide, DEFAULT_RELEASE, "hostile", &budgets), &Expect::Unsupported);
        count += 2;
        println!("{DONE_MARKER} {count}");
    }

    /// How long the child may run. A probe that made the frontend
    /// quadratic (or hang) must fail this test, not stall CI; the child
    /// normally finishes in seconds.
    const CHILD_DEADLINE: Duration = Duration::from_secs(600);

    /// Drain a child pipe on its own thread, so a chatty child can never
    /// block on a full pipe while the parent is waiting for it to exit.
    fn drain(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            String::from_utf8_lossy(&bytes).into_owned()
        })
    }

    #[test]
    fn hostile_inputs_end_in_typed_errors_in_a_child_process() {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([CHILD_TEST, "--exact", "--ignored", "--nocapture", "--test-threads=1"])
            .env(CHILD_ENV, "1")
            // The deep-nesting probes must run on the default 2 MiB test
            // stack: an inherited RUST_MIN_STACK would give them a deeper
            // stack than a real caller is guaranteed.
            .env_remove("RUST_MIN_STACK")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = drain(child.stdout.take().unwrap());
        let stderr = drain(child.stderr.take().unwrap());
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if started.elapsed() > CHILD_DEADLINE {
                let _ = child.kill();
                let _ = child.wait();
                panic!("hostile probe child still running after {CHILD_DEADLINE:?}; killed");
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let (stdout, stderr) = (stdout.join().unwrap(), stderr.join().unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert!(
                status.signal().is_none(),
                "hostile probe child ABORTED by signal {:?} (stack overflow or abort):\n{stderr}",
                status.signal()
            );
        }
        assert!(
            status.success(),
            "hostile probe child failed ({status}):\n{stdout}\n{stderr}"
        );
        // libtest prints "test <name> ... " before the test's own output, so
        // the marker shares a line with it.
        let passed = stdout
            .lines()
            .find_map(|line| line.split_once(DONE_MARKER).map(|(_, count)| count))
            .unwrap_or_else(|| panic!("the child never reported its probes:\n{stdout}"));
        let passed: usize = passed.trim().parse().unwrap();
        assert!(passed >= 60, "only {passed} hostile probes ran");
    }
}
