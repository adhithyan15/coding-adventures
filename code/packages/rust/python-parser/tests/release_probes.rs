//! LANG84 stage P for the Python release pairs, plus release selection (C4),
//! budgets (C5) and error spans (C6).
//!
//! Stage P means: positive and negative parse probes for the release's
//! declared `grammar_subset` (py-int-print) pass through the COMPILED tables,
//! and hostile probes come back as errors. Each probe below runs against
//! every release that claims it in `code/grammars/python/releases.csv`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use coding_adventures_python_lexer::releases as lexer_releases;
use coding_adventures_python_parser::{
    grammar_classes, parse_python, parse_python_at, resolve_release, supported_releases,
    tokenize_python_at, FrontendBudgets, PythonFrontendError, DEFAULT_MAX_SOURCE_BYTES,
    DEFAULT_MAX_TOKENS, DEFAULT_RELEASE, MAX_TRUSTED_SOURCE_BYTES, MAX_TRUSTED_TOKENS,
};
use parser::grammar_parser::GrammarASTNode;

fn grammars_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/python")
}

/// Rows of a CSV with `#` provenance lines, keyed by header name (lesson:
/// read the header, never assume a column position).
fn csv_rows(name: &str) -> Vec<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(grammars_dir().join(name)).unwrap();
    let mut lines = text.lines().filter(|l| !l.starts_with('#') && !l.is_empty());
    let header: Vec<String> = lines.next().unwrap().split(',').map(str::to_string).collect();
    lines
        .map(|line| {
            let cells: Vec<&str> = line.split(',').collect();
            assert_eq!(cells.len(), header.len(), "{name}: {line}");
            header.iter().cloned().zip(cells.into_iter().map(str::to_string)).collect()
        })
        .collect()
}

fn paired_rows() -> Vec<BTreeMap<String, String>> {
    csv_rows("releases.csv")
        .into_iter()
        .filter(|row| row["grammar_stage"] != "none" || row["grammar_status"] != "pending")
        .collect()
}

fn parse(release: &str, source: &str) -> Result<GrammarASTNode, PythonFrontendError> {
    parse_python_at(source, Some(release), &FrontendBudgets::default()).map(|p| p.ast)
}

// ---------------------------------------------------------------------------
// Inventory, tables and pair files agree
// ---------------------------------------------------------------------------

#[test]
fn supported_releases_are_exactly_the_paired_inventory_rows() {
    let paired: Vec<String> = paired_rows().into_iter().map(|row| row["release"].clone()).collect();
    let compiled: Vec<String> = supported_releases().iter().map(|r| r.to_string()).collect();
    assert_eq!(
        compiled, paired,
        "regenerate with grammar-tools generate-rust-compiled-grammars python"
    );
    assert_eq!(lexer_releases::SUPPORTED_RELEASES, supported_releases());
    assert!(supported_releases().contains(&DEFAULT_RELEASE));
}

#[test]
fn every_paired_row_claims_stage_p_for_py_int_print() {
    for row in paired_rows() {
        assert_eq!(
            (
                row["grammar_status"].as_str(),
                row["grammar_stage"].as_str(),
                row["grammar_subset"].as_str()
            ),
            ("partial", "P", "py-int-print"),
            "{}: this test proves exactly partial/P/py-int-print",
            row["release"]
        );
    }
}

/// Stage G: each pair file's header cites the row's `source_ref` and
/// `source_sha`, declares itself partial, and repeats the syntax-file blob
/// IDs `pair-evidence.csv` records for that release.
#[test]
fn pair_headers_cite_their_inventory_row_and_evidence() {
    let evidence: BTreeMap<String, BTreeMap<String, String>> = csv_rows("pair-evidence.csv")
        .into_iter()
        .map(|row| (row["release"].clone(), row))
        .collect();
    let rows = paired_rows();
    assert_eq!(evidence.len(), rows.len(), "one evidence row per paired release");
    for row in rows {
        let release = &row["release"];
        let ev = &evidence[release];
        assert_eq!(ev["commit"], row["source_sha"], "{release}: evidence commit");
        assert_eq!(ev["basis"], "reviewed", "{release}");
        for suffix in ["tokens", "grammar"] {
            let path = grammars_dir().join(format!("python{release}.{suffix}"));
            let text = std::fs::read_to_string(&path).unwrap();
            let header: Vec<&str> = text.lines().take_while(|l| l.starts_with('#')).collect();
            let joined = header.join("\n");
            assert!(
                header[0].starts_with(&format!("# Python {release} ({})", row["date"])),
                "{path:?}"
            );
            assert!(header.contains(&format!("# source_ref: {}", row["source_ref"]).as_str()), "{path:?}");
            assert!(header.contains(&format!("# source_sha: {}", row["source_sha"]).as_str()), "{path:?}");
            assert!(header.iter().any(|l| l.starts_with("# PARTIAL:")), "{path:?}");
            for column in ["grammar_blob", "tokenizer_blob", "keyword_py_blob"] {
                assert!(joined.contains(&ev[column]), "{path:?} does not cite {column} {}", ev[column]);
            }
            assert!(joined.contains(&format!("{} blob", ev["grammar_path"])), "{path:?}");
        }
    }
}

/// Releases with identical syntax share one compiled module; releases whose
/// lexers differ do not.
#[test]
fn deduplicated_classes_follow_the_evidence() {
    let class = |r: &str| grammar_classes(r).unwrap();
    assert_eq!(class("3.12.3"), class("3.13.16"));
    assert_eq!(class("3.13.16"), class("3.16.0a1"));
    // 3.11.17 has no `!` token; 3.0a1 has no `_` separator, no `@=`/`:=`
    // and no async/await keywords. The subset grammar is the same for all.
    assert_ne!(class("3.11.17").0, class("3.12.3").0);
    assert_ne!(class("3.0a1").0, class("3.11.17").0);
    let grammar = class("3.0a1").1;
    assert!(supported_releases().iter().all(|r| class(r).1 == grammar));
    assert_eq!(grammar_classes("3.12.2"), None);
    assert_eq!(grammar_classes("3.12"), None);
}

// ---------------------------------------------------------------------------
// Release selection: exact IDs only, typed errors, no fallback (C4)
// ---------------------------------------------------------------------------

#[test]
fn release_selection_is_exact_and_typed() {
    assert_eq!(resolve_release(None), Ok(DEFAULT_RELEASE));
    assert_eq!(DEFAULT_RELEASE, "3.13.16");
    assert_eq!(resolve_release(Some("3.0a1")), Ok("3.0a1"));

    // A real row without a pair is not silently replaced by a neighbour.
    assert_eq!(
        resolve_release(Some("3.12.2")),
        Err(PythonFrontendError::ReleaseNotAvailable {
            release: "3.12.2".into(),
            stage: "none".into()
        })
    );
    // Release 3.0 is a row; the legacy python3.0.* minor-line pair is not
    // its pair and must not be reached through it.
    assert!(matches!(
        resolve_release(Some("3.0")),
        Err(PythonFrontendError::ReleaseNotAvailable { .. })
    ));

    // An empty string is an error, not the default.
    assert_eq!(
        resolve_release(Some("")),
        Err(PythonFrontendError::UnknownRelease {
            requested: String::new(),
            suggestions: vec![]
        })
    );

    // A minor line is not a tagged release (LANG84 v1 accepts exact IDs
    // only); the error lists up to five rows sharing the longest prefix.
    let Err(PythonFrontendError::UnknownRelease { suggestions, .. }) = resolve_release(Some("3.12"))
    else {
        panic!("3.12 must be unknown");
    };
    assert!(!suggestions.is_empty() && suggestions.len() <= 5, "{suggestions:?}");
    assert!(suggestions.iter().all(|s| s.starts_with("3.12")), "{suggestions:?}");

    // A hostile-length request is truncated before it is echoed.
    let long = "9".repeat(10_000);
    let Err(PythonFrontendError::UnknownRelease { requested, .. }) = resolve_release(Some(&long))
    else {
        panic!("long ID must be unknown");
    };
    assert!(requested.len() < 100);

    // Control characters in a request are escaped, never echoed raw.
    let Err(error) = resolve_release(Some("x\nINFO: ok\u{1b}[2J")) else {
        panic!("a control-character ID must be unknown");
    };
    let PythonFrontendError::UnknownRelease { requested, .. } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(requested, "x\\nINFO: ok\\u{1b}[2J");
    assert!(!error.to_string().chars().any(char::is_control), "{error}");

    // Every entry point refuses before doing any work.
    let budgets = FrontendBudgets::default();
    assert!(matches!(
        tokenize_python_at("print(1)", Some("2.7.18"), &budgets),
        Err(PythonFrontendError::ReleaseNotAvailable { .. })
    ));
    assert!(matches!(
        parse_python_at("print(1)", Some("python-3.13.16"), &budgets),
        Err(PythonFrontendError::UnknownRelease { .. })
    ));
    assert!(lexer_releases::token_grammar("3.12").is_none());
}

#[test]
fn every_error_names_the_release_and_a_one_based_position() {
    let lex = parse("3.13.16", "print(1)\nprint('x')").unwrap_err();
    assert_eq!(
        lex,
        PythonFrontendError::Lex {
            release: "3.13.16".into(),
            line: 2,
            column: 7,
            message: "Unexpected sequence '\\''".into()
        }
    );
    let parse_error = parse("3.0a1", "print(1)\nprint 2").unwrap_err();
    let PythonFrontendError::Parse {
        release,
        line,
        column,
        message,
    } = &parse_error
    else {
        panic!("{parse_error:?}");
    };
    assert_eq!((release.as_str(), *line), ("3.0a1", 2));
    assert!(*column >= 1);
    // The furthest failure lists the terminals it expected.
    assert!(message.starts_with("Expected "), "{message}");
    assert!(parse_error.to_string().starts_with("Python 3.0a1 parse error at 2:"));
}

// ---------------------------------------------------------------------------
// Stage P: py-int-print positive and negative probes, per release
// ---------------------------------------------------------------------------

const POSITIVE_ALL: &[&str] = &[
    "print(1 + 2)",
    "print(3 * 4)",
    "print(-7 // 2)",
    "print(-7 % 2)",
    "print(7 / 2)", // parses; the compiler refuses `/` (a float result)
    "print()",
    "print(1,)",
    "print(1, 2)", // parses; the compiler refuses two arguments
    "print((2))",
    "print(- 3)",
    "print(+-+3)",
    "print(2 * -3)",
    "print(1 +\n2 *\n3)",
    "print(\n1\n)",
    "print(1); print(2)",
    "print(1);",
    "\n\nprint(1)\n\n",
    "# leading comment\nprint(1) # comment\nprint(2)\n",
    "print(1 + \\\n 2)",
    "print(0)",
    "print(00)",
    "print(9223372036854775807)",
    "abs(1)", // parses; the compiler refuses a callee other than print
    "1 + 2",  // parses; the compiler refuses a bare expression
    "print(print(1))",
];

/// PEP 515 (3.6): `_` separators. Valid from 3.6, a lexer error before.
const UNDERSCORES: &[&str] = &["print(1_000)", "print(0_0)", "print(1_2_3 * 4)"];

const NEGATIVE_ALL: &[&str] = &[
    "print(",
    "print(1",
    "print(1) print(2)",
    "print(1);;",
    ";print(1)",
    "print(1 +)",
    "print(0x10)",
    "print(1.5)",
    "print(1e3)",
    "print('a')",
    "print(012)",
    "print(1__0)",
    "print(1_)",
    "print 1",
    "x = 1",
    "print(x[0])",
    "print(a.b)",
    "print(2 ** 3)",
    "print(~1)",
    "print(1 < 2)",
    "print(())",
    "print(1, end=None)",
    "print(*a)",
    "  print(1)",
    "if 1:\n    print(1)",
    "print(1) \\",
    "print(1 + \\ \n2)",
];

#[test]
fn py_int_print_probes_for_every_release() {
    let mut checked = 0;
    for &release in supported_releases() {
        let early = release == "3.0a1";
        for source in POSITIVE_ALL {
            let ast = parse(release, source).unwrap_or_else(|e| panic!("{release}: {source:?} rejected: {e}"));
            assert_eq!(ast.rule_name, "file");
            checked += 1;
        }
        for source in UNDERSCORES {
            match parse(release, source) {
                Ok(_) => assert!(!early, "3.0a1 must not lex {source:?}"),
                Err(PythonFrontendError::Lex { .. }) => assert!(early, "{release}: {source:?}"),
                Err(other) => panic!("{release}: {source:?}: {other:?}"),
            }
            checked += 1;
        }
        for source in NEGATIVE_ALL {
            let error = parse(release, source)
                .err()
                .unwrap_or_else(|| panic!("{release}: {source:?} accepted"));
            assert!(
                matches!(error, PythonFrontendError::Parse { .. } | PythonFrontendError::Lex { .. }),
                "{release}: {source:?}: {error:?}"
            );
            checked += 1;
        }
    }
    assert_eq!(
        checked,
        supported_releases().len() * (POSITIVE_ALL.len() + UNDERSCORES.len() + NEGATIVE_ALL.len())
    );
}

/// The release's operators lex as the release's tokenizer has them: `:=`
/// and `@=` exist from 3.8/3.5 (not in 3.0a1), `!` alone from 3.12.
#[test]
fn per_release_operator_sets_follow_the_tokenizer() {
    let budgets = FrontendBudgets::default();
    let lexes = |release: &str, source: &str| tokenize_python_at(source, Some(release), &budgets).is_ok();
    let kinds = |release: &str, source: &str| -> Vec<String> {
        let (_, tokens) = tokenize_python_at(source, Some(release), &budgets).unwrap();
        tokens.iter().map(|t| t.effective_type_name().to_string()).collect()
    };
    // 3.0a1's tokenizer has no `:=` or `@=`, so each is two tokens there,
    // exactly as CPython 3.0a1 splits them.
    assert_eq!(kinds("3.0a1", "a := 1")[1..3], ["COLON", "EQUALS"]);
    assert_eq!(kinds("3.11.17", "a := 1")[1], "WALRUS");
    assert_eq!(kinds("3.0a1", "a @= 1")[1..3], ["AT", "EQUALS"]);
    assert_eq!(kinds("3.11.17", "a @= 1")[1], "AT_EQUALS");
    assert!(!lexes("3.11.17", "a ! b") && lexes("3.12.3", "a ! b"));
    // `async` became a hard keyword in 3.7; in 3.0a1 it is a NAME.
    let kind = |release: &str| {
        let (_, tokens) = tokenize_python_at("async\n", Some(release), &budgets).unwrap();
        tokens[0].effective_type_name().to_string()
    };
    assert_eq!(kind("3.0a1"), "NAME");
    assert_eq!(kind("3.13.16"), "KEYWORD");
    // `print` is a NAME in every Python 3 release.
    for &release in supported_releases() {
        let (_, tokens) = tokenize_python_at("print\n", Some(release), &budgets).unwrap();
        assert_eq!(tokens[0].effective_type_name(), "NAME", "{release}");
    }
}

/// The legacy minor-line API is untouched by the release tables.
#[test]
fn legacy_minor_line_api_is_unchanged() {
    assert_eq!(parse_python("x = 1\n", "3.12").unwrap().rule_name, "file");
    assert!(parse_python("print(1)\n", "3.13.16").is_err());
}

// ---------------------------------------------------------------------------
// Budgets and hostile input (C5), in a child process
// ---------------------------------------------------------------------------

const HOSTILE_ENV: &str = "PYTHON_PARSER_HOSTILE_PROBE_CHILD";

/// Runs only inside the child spawned below, so a stack overflow or abort
/// shows up as a failed child process rather than taking the harness down.
#[test]
fn hostile_probe_child() {
    if std::env::var_os(HOSTILE_ENV).is_none() {
        return;
    }
    let budgets = FrontendBudgets::default();
    for &release in supported_releases() {
        // Deep nesting stops at the parser's depth guard.
        let deep = format!("print({}1{})", "(".repeat(5_000), ")".repeat(5_000));
        assert!(matches!(
            parse_python_at(&deep, Some(release), &budgets),
            Err(PythonFrontendError::Budget { which: "parser rule depth", .. })
        ));
        // The release grammars admit far more nesting than the legacy 3.12
        // grammar's 9 levels under the same cap.
        let mut admitted = 0;
        for depth in 1..60 {
            let source = format!("print({}1{})", "(".repeat(depth), ")".repeat(depth));
            match parse_python_at(&source, Some(release), &budgets) {
                Ok(_) => admitted = depth,
                Err(PythonFrontendError::Budget { .. }) => break,
                Err(other) => panic!("{release} depth {depth}: {other}"),
            }
        }
        assert!(admitted >= 15, "{release}: {admitted}");
        // Oversized input is refused before lexing.
        let big = format!("print({})", "1".repeat(64 * 1024));
        assert!(matches!(
            parse_python_at(&big, Some(release), &budgets),
            Err(PythonFrontendError::Budget { which: "source bytes", limit: 65_536, .. })
        ));
        // A long flat operator chain parses or fails cleanly; it must not
        // recurse per operator.
        let chain = format!("print(1{})", " + 1".repeat(7_000));
        assert!(chain.len() < 64 * 1024);
        let _ = parse_python_at(&chain, Some(release), &budgets);
        // A long unary chain is recursion, so it stops at the depth guard.
        let unary = format!("print({}1)", "-".repeat(5_000));
        assert!(parse_python_at(&unary, Some(release), &budgets).is_err());
        // Unterminated constructs and junk are ordinary errors.
        for junk in ["print(1 + ", "print((((", "\u{0}\u{1}", "€€€", "print(1) \\", "\t print(1)"] {
            assert!(parse_python_at(junk, Some(release), &budgets).is_err(), "{release}: {junk:?}");
        }
        // A trusted caller may raise the source budget, never the depth cap.
        let trusted = FrontendBudgets {
            max_source_bytes: usize::MAX,
            max_rule_depth: usize::MAX,
            max_tokens: usize::MAX,
        };
        assert!(matches!(
            parse_python_at(&deep, Some(release), &trusted),
            Err(PythonFrontendError::Budget { which: "parser rule depth", limit: 190, .. })
        ));
    }
}

#[test]
fn hostile_probes_return_errors_in_a_child_process() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "hostile_probe_child", "--nocapture", "--test-threads=1"])
        .env(HOSTILE_ENV, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "hostile probes crashed or failed: {:?}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
}

/// The packrat parser's memory grows with the token count, so a dense
/// program inside the 64 KiB source budget is refused before the parser
/// starts, and trusted callers can raise the token budget only to its
/// ceiling.
#[test]
fn token_budget_refuses_dense_input_before_parsing() {
    let budgets = FrontendBudgets::default();
    for release in ["3.0a1", "3.13.16"] {
        // `print(1+1+…)`: two tokens per two bytes, well inside 64 KiB.
        let dense = format!("print(1{})", "+1".repeat(20_000));
        assert!(dense.len() < DEFAULT_MAX_SOURCE_BYTES);
        assert!(
            matches!(
                parse_python_at(&dense, Some(release), &budgets),
                Err(PythonFrontendError::Budget { which: "tokens", limit: DEFAULT_MAX_TOKENS, .. })
            ),
            "{release}"
        );
        let trusted = FrontendBudgets {
            max_tokens: usize::MAX,
            ..FrontendBudgets::default()
        };
        let (_, tokens) = tokenize_python_at(&dense, Some(release), &trusted).expect("within ceiling");
        assert!(tokens.len() > DEFAULT_MAX_TOKENS && tokens.len() <= MAX_TRUSTED_TOKENS);
    }
    assert_eq!(MAX_TRUSTED_SOURCE_BYTES, 1024 * 1024);
}
