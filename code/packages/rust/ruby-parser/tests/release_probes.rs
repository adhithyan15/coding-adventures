//! LANG84 stage P for the Ruby release pairs, plus release selection (C4),
//! budgets (C5) and error spans (C6).
//!
//! Stage P means: positive and negative parse probes for the release's
//! declared `grammar_subset` (rb-int-puts) pass through the COMPILED tables,
//! and hostile probes come back as errors. Each probe below runs against
//! every release that claims it in `code/grammars/ruby/releases.csv`.

use std::collections::BTreeSet;
use std::path::Path;

use coding_adventures_ruby_lexer::releases as lexer_releases;
use coding_adventures_ruby_parser::{
    grammar_classes, parse_ruby_at, resolve_release, supported_releases, tokenize_ruby_at,
    FrontendBudgets, RubyFrontendError, DEFAULT_MAX_SOURCE_BYTES, DEFAULT_MAX_TOKENS, DEFAULT_RELEASE,
    MAX_TRUSTED_SOURCE_BYTES, MAX_TRUSTED_TOKENS,
};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

fn grammars_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/ruby")
}

/// (release, date, source_ref, source_sha, grammar_status, grammar_stage, grammar_subset)
fn inventory() -> Vec<Vec<String>> {
    let text = std::fs::read_to_string(grammars_dir().join("releases.csv")).unwrap();
    let mut lines = text.lines().filter(|line| !line.starts_with('#'));
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let col = |name: &str| header.iter().position(|h| *h == name).unwrap();
    let cols = [
        col("release"),
        col("date"),
        col("source_ref"),
        col("source_sha"),
        col("grammar_status"),
        col("grammar_stage"),
        col("grammar_subset"),
    ];
    lines
        .map(|line| {
            let cells: Vec<&str> = line.split(',').collect();
            cols.iter().map(|&c| cells[c].to_string()).collect()
        })
        .collect()
}

fn parse(release: &str, source: &str) -> Result<GrammarASTNode, RubyFrontendError> {
    parse_ruby_at(source, Some(release), &FrontendBudgets::default()).map(|p| p.ast)
}

/// The 1994 grammars have no paren-less command call and no newline before `)`.
fn is_early(release: &str) -> bool {
    release.starts_with("0.")
}

// ---------------------------------------------------------------------------
// Inventory, tables and pair files agree
// ---------------------------------------------------------------------------

#[test]
fn supported_releases_are_exactly_the_paired_inventory_rows() {
    let paired: Vec<String> = inventory()
        .into_iter()
        .filter(|row| row[5] != "none" || row[4] != "pending")
        .map(|row| row[0].clone())
        .collect();
    let compiled: Vec<String> = supported_releases().iter().map(|r| r.to_string()).collect();
    assert_eq!(compiled, paired, "regenerate with grammar-tools generate-rust-compiled-grammars ruby");
    assert_eq!(lexer_releases::SUPPORTED_RELEASES, supported_releases());
    assert!(supported_releases().contains(&DEFAULT_RELEASE));
}

#[test]
fn every_paired_row_claims_stage_p_for_rb_int_puts() {
    for row in inventory().iter().filter(|row| row[5] != "none") {
        assert_eq!(
            (row[4].as_str(), row[5].as_str(), row[6].as_str()),
            ("partial", "P", "rb-int-puts"),
            "{}: this test proves exactly partial/P/rb-int-puts",
            row[0]
        );
    }
}

/// Stage G: each pair file's header cites the row's source_ref and
/// source_sha, and declares itself partial.
#[test]
fn pair_headers_cite_their_inventory_row() {
    for row in inventory().iter().filter(|row| row[5] != "none") {
        for suffix in ["tokens", "grammar"] {
            let path = grammars_dir().join(format!("ruby{}.{suffix}", row[0]));
            let text = std::fs::read_to_string(&path).unwrap();
            let header: Vec<&str> = text.lines().take_while(|l| l.starts_with('#')).collect();
            assert!(header[0].starts_with(&format!("# Ruby {} ({})", row[0], row[1])), "{path:?}");
            assert!(header.contains(&format!("# source_ref: {}", row[2]).as_str()), "{path:?}");
            assert!(header.contains(&format!("# source_sha: {}", row[3]).as_str()), "{path:?}");
            assert!(header.iter().any(|l| l.starts_with("# PARTIAL:")), "{path:?}");
            assert!(header.iter().any(|l| l.starts_with("# evidence: parse.y blob ")), "{path:?}");
        }
    }
}

/// Releases with identical syntax blobs share one compiled module; releases
/// whose lexers differ do not.
#[test]
fn deduplicated_classes_follow_the_evidence() {
    let class = |r: &str| grammar_classes(r).unwrap();
    assert_eq!(class("3.3.5"), class("3.3.6"));
    assert_eq!(class("3.3.6"), class("3.3.7"));
    // 0.50 lexes a backtick as an xstring opener and changes the keyword
    // table, but the subset grammar is the same.
    assert_ne!(class("0.49").0, class("0.50").0);
    assert_eq!(class("0.49").1, class("0.50").1);
    assert_ne!(class("0.50"), class("3.3.6"));
    let token_classes: BTreeSet<_> = supported_releases().iter().map(|r| class(r).0).collect();
    assert_eq!(token_classes.len(), 3);
    assert_eq!(grammar_classes("2.7.8"), None);
}

// ---------------------------------------------------------------------------
// Release selection: exact IDs only, typed errors, no fallback (C4)
// ---------------------------------------------------------------------------

#[test]
fn release_selection_is_exact_and_typed() {
    assert_eq!(resolve_release(None), Ok(DEFAULT_RELEASE));
    assert_eq!(resolve_release(Some("0.49")), Ok("0.49"));
    assert_eq!(resolve_release(Some("3.3.7")), Ok("3.3.7"));

    // A real row without a pair is not silently replaced by a neighbour.
    assert_eq!(
        resolve_release(Some("3.3.4")),
        Err(RubyFrontendError::ReleaseNotAvailable {
            release: "3.3.4".into(),
            stage: "none".into()
        })
    );
    assert!(matches!(
        resolve_release(Some("0.51")),
        Err(RubyFrontendError::ReleaseNotAvailable { .. })
    ));

    // An empty string is an error, not the default.
    assert_eq!(
        resolve_release(Some("")),
        Err(RubyFrontendError::UnknownRelease {
            requested: String::new(),
            suggestions: vec![]
        })
    );

    // A line shorthand is not an ID (LANG84 v1 accepts exact IDs only); the
    // error lists up to five rows sharing the longest prefix.
    let Err(RubyFrontendError::UnknownRelease { suggestions, .. }) = resolve_release(Some("3.3"))
    else {
        panic!("3.3 must be unknown");
    };
    assert!(!suggestions.is_empty() && suggestions.len() <= 5, "{suggestions:?}");
    assert!(suggestions.iter().all(|s| s.starts_with("3.3")), "{suggestions:?}");

    // A hostile-length request is truncated before it is echoed.
    let long = "9".repeat(10_000);
    let Err(RubyFrontendError::UnknownRelease { requested, .. }) = resolve_release(Some(&long))
    else {
        panic!("long ID must be unknown");
    };
    assert!(requested.len() < 100);

    // Every entry point refuses before doing any work.
    let budgets = FrontendBudgets::default();
    assert!(matches!(
        tokenize_ruby_at("puts 1", Some("2.7.8"), &budgets),
        Err(RubyFrontendError::ReleaseNotAvailable { .. })
    ));
    assert!(matches!(
        parse_ruby_at("puts 1", Some("ruby-3.3.6"), &budgets),
        Err(RubyFrontendError::UnknownRelease { .. })
    ));
    assert!(lexer_releases::token_grammar("2.7.8").is_none());
}

#[test]
fn every_error_names_the_release_and_a_one_based_position() {
    let lex = parse("3.3.6", "puts(1)\nputs(\"x\")").unwrap_err();
    assert_eq!(
        lex,
        RubyFrontendError::Lex {
            release: "3.3.6".into(),
            line: 2,
            column: 6,
            message: "Unexpected sequence '\"'".into()
        }
    );
    let parse_error = parse("0.49", "puts(1)\nputs 2").unwrap_err();
    let RubyFrontendError::Parse {
        release,
        line,
        column,
        message,
    } = &parse_error
    else {
        panic!("{parse_error:?}");
    };
    assert_eq!((release.as_str(), *line), ("0.49", 2));
    assert!(*column >= 1);
    // The furthest failure lists the terminals it expected.
    assert!(message.starts_with("Expected "), "{message}");
    assert!(parse_error.to_string().starts_with("Ruby 0.49 parse error at 2:"));
}

// ---------------------------------------------------------------------------
// Stage P: rb-int-puts positive and negative probes, per release
// ---------------------------------------------------------------------------

const POSITIVE_ALL: &[&str] = &[
    "puts(1 + 2)",
    "puts(3 * 4)",
    "puts(-7 / 2)",
    "puts()",
    "puts(1, 2)",
    "puts(1 + 2, -7 / 2, 3 * 4)",
    "puts((2))",
    "puts(- 3)",
    "puts(2 * -3)",
    "puts(1 +\n2 *\n3)",
    "puts(\n1,\n2)",
    "puts(1); puts(2)",
    "\n\nputs(1)\n\n",
    "puts(1) # comment\nputs(2)\n",
    "puts(1 + \\\n 2)",
    "puts(1_000)",
];

const POSITIVE_MODERN: &[&str] = &[
    "puts 1 + 2",
    "puts(1\n)",
    "puts 1, 2",
    "puts(1);;puts(2)",
    ";puts(1)",
    "puts(+2)",
    "puts(\n)",
];

const NEGATIVE_ALL: &[&str] = &[
    "puts(",
    "puts(1,)",
    "puts(1) puts(2)",
    "puts(1)(2)",
    "puts(1 +)",
    "puts(0x10)",
    "puts(1.5)",
    "puts(1) +",
    "1 + 2",
    "puts(\"1\")",
];

const NEGATIVE_MODERN: &[&str] = &[
    // Space-sensitive in Ruby (`puts -1` is a unary argument, `puts - 1` a
    // subtraction); outside this subset until E6.
    "puts -1",
    "puts 1 if x",
    "puts(?a)",
];

/// Valid in 3.3 and a syntax error in the 1994 grammars.
const NEGATIVE_EARLY: &[&str] = &["puts 1 + 2", "puts(1\n)", "puts 1, 2"];

#[test]
fn rb_int_puts_probes_for_every_release() {
    let mut checked = 0;
    for &release in supported_releases() {
        let positives = POSITIVE_ALL
            .iter()
            .chain(if is_early(release) { &[][..] } else { POSITIVE_MODERN });
        for source in positives {
            let ast = parse(release, source)
                .unwrap_or_else(|e| panic!("{release}: {source:?} rejected: {e}"));
            assert_eq!(ast.rule_name, "program");
            checked += 1;
        }
        let negatives = NEGATIVE_ALL.iter().chain(if is_early(release) {
            NEGATIVE_EARLY
        } else {
            NEGATIVE_MODERN
        });
        for source in negatives {
            let error = parse(release, source)
                .err()
                .unwrap_or_else(|| panic!("{release}: {source:?} accepted"));
            assert!(
                matches!(error, RubyFrontendError::Parse { .. } | RubyFrontendError::Lex { .. }),
                "{release}: {source:?}: {error:?}"
            );
            checked += 1;
        }
    }
    assert!(checked >= 5 * 29, "{checked}");
}

/// The release CST keeps the rule names and shapes the Ruby IIR compiler
/// walks (LANG84 C8): the same skeleton the unversioned grammar produces.
#[test]
fn release_cst_keeps_the_iir_compiler_rule_names() {
    fn skeleton(node: &GrammarASTNode, out: &mut Vec<String>) {
        out.push(node.rule_name.clone());
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(inner) => skeleton(inner, out),
                ASTNodeOrToken::Token(token) => out.push(format!("'{}'", token.value)),
            }
        }
    }
    for source in ["puts(1 + 2, -7 / 2)", "puts((3) * 4)", "puts 5 - 6"] {
        let mut legacy = Vec::new();
        skeleton(&coding_adventures_ruby_parser::parse_ruby(source), &mut legacy);
        for &release in supported_releases() {
            let Ok(ast) = parse(release, source) else {
                // `puts 5 - 6` is not 1994 syntax (no command calls).
                assert!(is_early(release) && source.starts_with("puts "), "{release}: {source}");
                continue;
            };
            let mut current = Vec::new();
            skeleton(&ast, &mut current);
            if is_early(release) && source.contains("-7") {
                // yylex folds `-7` into one INTEGER in expression-start
                // position, so that factor is a single NUMBER token.
                assert!(current.contains(&"'-7'".to_string()), "{release}: {current:?}");
                continue;
            }
            assert_eq!(current, legacy, "{release}: {source}");
        }
    }
}

// ---------------------------------------------------------------------------
// Budgets and hostile input (C5), in a child process
// ---------------------------------------------------------------------------

const HOSTILE_ENV: &str = "RUBY_PARSER_HOSTILE_PROBE_CHILD";

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
        let deep = format!("puts({}1{})", "(".repeat(5_000), ")".repeat(5_000));
        assert!(matches!(
            parse_ruby_at(&deep, Some(release), &budgets),
            Err(RubyFrontendError::Budget { which: "parser rule depth", .. })
        ));
        // Oversized input is refused before lexing.
        let big = format!("puts({})", "1".repeat(64 * 1024));
        assert!(matches!(
            parse_ruby_at(&big, Some(release), &budgets),
            Err(RubyFrontendError::Budget { which: "source bytes", limit: 65_536, .. })
        ));
        // A long flat operator chain just under the budget parses or fails
        // cleanly; it must not recurse per operator.
        let chain = format!("puts(1{})", " + 1".repeat(15_000));
        assert!(chain.len() < 64 * 1024);
        let _ = parse_ruby_at(&chain, Some(release), &budgets);
        // Unterminated constructs and junk are ordinary errors.
        for junk in ["puts(1 + ", "puts((((", "\u{0}\u{1}", "€€€", "puts(1) \\"] {
            assert!(parse_ruby_at(junk, Some(release), &budgets).is_err(), "{release}: {junk:?}");
        }
        // A trusted caller may raise the source budget, never the depth cap.
        let trusted = FrontendBudgets {
            max_source_bytes: usize::MAX,
            max_rule_depth: usize::MAX,
            max_tokens: usize::MAX,
        };
        assert!(matches!(
            parse_ruby_at(&deep, Some(release), &trusted),
            Err(RubyFrontendError::Budget { which: "parser rule depth", limit: 180, .. })
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


/// Security review (LANG84 R0): the packrat parser's memory grows ~3 KB per
/// token, so a dense program within the 64 KiB source budget used ~192 MB and
/// a trusted 16 MiB budget would have needed ~48 GB (an OOM abort, not an
/// error). The token budget refuses such input before the parser starts, and
/// trusted callers can raise it only to `MAX_TRUSTED_TOKENS`.
#[test]
fn token_budget_refuses_dense_input_before_parsing() {
    let budgets = FrontendBudgets::default();
    for release in ["0.49", "3.3.6"] {
        // `puts(1,1,…)`: two tokens per three bytes, well inside 64 KiB.
        let dense = format!("puts(1{})", ",1".repeat(20_000));
        assert!(dense.len() < DEFAULT_MAX_SOURCE_BYTES);
        assert!(matches!(
            parse_ruby_at(&dense, Some(release), &budgets),
            Err(RubyFrontendError::Budget { which: "tokens", limit: DEFAULT_MAX_TOKENS, .. })
        ), "{release}");
        assert!(matches!(
            tokenize_ruby_at(&dense, Some(release), &budgets),
            Err(RubyFrontendError::Budget { which: "tokens", .. })
        ), "{release}");
        // A trusted caller can raise the budget, but only to the ceiling.
        let trusted = FrontendBudgets { max_tokens: usize::MAX, ..FrontendBudgets::default() };
        let (_, tokens) = tokenize_ruby_at(&dense, Some(release), &trusted).expect("within ceiling");
        assert!(tokens.len() > DEFAULT_MAX_TOKENS && tokens.len() <= MAX_TRUSTED_TOKENS);
    }
    assert_eq!(MAX_TRUSTED_SOURCE_BYTES, 1024 * 1024);
}
