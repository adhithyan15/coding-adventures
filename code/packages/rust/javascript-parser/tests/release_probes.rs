//! LANG84 stages G and P for every ECMAScript edition whose inventory row
//! claims a grammar stage (phase 6d).
//!
//! `code/grammars/ecmascript/releases.csv` may claim only what a test
//! proves. For each row past `grammar_stage none`:
//!
//! ```text
//!   G   both pair files exist, and each header cites the row's source_ref
//!       and source_sha and declares itself PARTIAL
//!   P   positive probes of the row's grammar_subset parse through the
//!       edition's compiled tables (parse_javascript_cst_bounded), negative
//!       probes are refused, and hostile probes are refused without a panic
//!       or a stack overflow
//! ```
//!
//! The probes are the `es-number-log` neighbourhood: numeric literals, the
//! four arithmetic operators, unary minus, `let`/`const`, `console.log`
//! calls, and automatic semicolon insertion between them. The F10 mode table
//! (phase 6a) is probed directly: `/` after a value is division, so a chain
//! of divisions never lexes as a regex.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use coding_adventures_javascript_parser::parse_javascript_cst_bounded;
use coding_adventures_javascript_tokens::EsVersion;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

fn ecmascript_grammars() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/ecmascript")
}

/// Rows of `releases.csv`, keyed by header name (never by position).
fn inventory() -> Vec<BTreeMap<String, String>> {
    let path = ecmascript_grammars().join("releases.csv");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines = text.lines().filter(|l| !l.starts_with('#') && !l.is_empty());
    let header: Vec<String> = lines.next().unwrap().split(',').map(str::to_string).collect();
    lines
        .map(|line| {
            let cells: Vec<&str> = line.split(',').collect();
            assert_eq!(cells.len(), header.len(), "{line}");
            header.iter().cloned().zip(cells.into_iter().map(str::to_string)).collect()
        })
        .collect()
}

/// Rows past `grammar_stage none`, with the edition each one selects.
fn staged() -> Vec<(BTreeMap<String, String>, EsVersion)> {
    inventory()
        .into_iter()
        .filter(|row| row["grammar_stage"] != "none")
        .map(|row| {
            // A release ID is the edition's own spelling (LANG84 6d plan).
            let version = EsVersion::from_str(&row["release"])
                .unwrap_or_else(|_| panic!("{}: not an edition with a pair", row["release"]));
            (row, version)
        })
        .collect()
}

#[test]
fn the_staged_rows_are_es2016_through_es2025() {
    let ids: Vec<String> = staged().into_iter().map(|(row, _)| row["release"].clone()).collect();
    let expected: Vec<String> = (2016..=2025).map(|y| format!("es{y}")).collect();
    assert_eq!(ids, expected);
    for (row, _) in staged() {
        assert_eq!(row["grammar_stage"], "P", "{}", row["release"]);
        assert_eq!(row["grammar_subset"], "es-number-log", "{}", row["release"]);
    }
}

/// Stage G: each pair file's leading comment block cites its row.
#[test]
fn pair_headers_cite_their_inventory_row() {
    for (row, version) in staged() {
        let release = &row["release"];
        assert_ne!(row["source_sha"], "-", "{release}: stage G needs recorded evidence");
        for ext in ["tokens", "grammar"] {
            let path = ecmascript_grammars().join(format!("{}.{ext}", version.as_str()));
            let text = std::fs::read_to_string(&path).unwrap();
            let header: Vec<&str> = text.lines().take_while(|l| l.starts_with('#')).collect();
            let cites = |line: String| header.iter().any(|h| *h == line);
            assert!(cites(format!("# source_ref: {}", row["source_ref"])), "{path:?}");
            assert!(cites(format!("# source_sha: {}", row["source_sha"])), "{path:?}");
            assert!(header.iter().any(|l| l.starts_with("# PARTIAL:")), "{path:?}");
            let edition = release.trim_start_matches("es");
            assert!(cites(format!("# @ecmascript_edition {edition}")), "{path:?}");
        }
    }
}

/// Every token of a tree, in order, as `TYPE:value`.
fn leaves(node: &GrammarASTNode, out: &mut Vec<String>) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(token) => {
                out.push(format!("{}:{}", token.effective_type_name(), token.value))
            }
            ASTNodeOrToken::Node(inner) => leaves(inner, out),
        }
    }
}

const POSITIVE: &[&str] = &[
    "console.log(1 + 2, 7 - 10, 6 * 7);",
    "console.log(8 / 2 / 2, 1 / 3);",
    "let a = 9;\nconsole.log(a / 3 / 3, a /3, a/ 3);",
    "const y = -(4 - 6) * 2; console.log(y, -y);",
    "console.log()\nconsole.log(0x1F, 0b101, 0o17, 0.5, .5)\n",
    "",
];

const NEGATIVE: &[&str] = &[
    "console.log(1 +);",
    "let = 1;",
    "console.log(1",
    "let x = ;",
    "console.log(1 2);",
];

#[test]
fn positive_probes_parse_on_every_staged_edition() {
    for (row, version) in staged() {
        for source in POSITIVE {
            let tree = parse_javascript_cst_bounded(source, version)
                .unwrap_or_else(|e| panic!("{} {source:?}: {e}", row["release"]));
            assert_eq!(tree.rule_name, "program", "{}", row["release"]);
        }
    }
}

/// The F10 table: after a value, `/` is division. A three-slash chain must
/// give three SLASH tokens and no REGEX, on every staged edition.
#[test]
fn division_chains_lex_as_division_on_every_staged_edition() {
    for (row, version) in staged() {
        let tree = parse_javascript_cst_bounded("let a = 8;\nconsole.log(a / 2 / 2 / 1);", version).unwrap();
        let mut tokens = Vec::new();
        leaves(&tree, &mut tokens);
        let slashes = tokens.iter().filter(|t| t.starts_with("SLASH:")).count();
        assert_eq!(slashes, 3, "{}: {tokens:?}", row["release"]);
        assert!(!tokens.iter().any(|t| t.starts_with("REGEX:")), "{}: {tokens:?}", row["release"]);
    }
}

#[test]
fn negative_probes_are_refused_on_every_staged_edition() {
    for (row, version) in staged() {
        for source in NEGATIVE {
            assert!(
                parse_javascript_cst_bounded(source, version).is_err(),
                "{} accepted {source:?}",
                row["release"]
            );
        }
    }
}

/// Hostile input is refused with an error: never a panic, never a stack
/// overflow, never unbounded work.
#[test]
fn hostile_probes_are_refused_without_a_panic() {
    let deep_parens = format!("console.log({}1{});", "(".repeat(5_000), ")".repeat(5_000));
    let long_chain = format!("console.log(1{});", " / 1".repeat(5_000));
    let deep_unary = format!("console.log({}1);", "-".repeat(5_000));
    let exponent = "console.log(2 ** 3 ** 4);".to_string();
    for (row, version) in staged() {
        for source in [&deep_parens, &long_chain, &deep_unary, &exponent] {
            let outcome = std::panic::catch_unwind(|| parse_javascript_cst_bounded(source, version));
            let result = outcome.unwrap_or_else(|_| panic!("{} panicked", row["release"]));
            assert!(result.is_err(), "{} accepted a hostile probe", row["release"]);
        }
    }
}
