//! LANG84 stage I(`es-number-log`) for every claiming ECMAScript edition
//! (phase 6d).
//!
//! `releases.csv` may claim only what a test proves. This file proves the
//! `iir_status` column:
//!
//! ```text
//!   for each release R whose iir_status lists es-number-log:
//!     for each corpus case C:
//!       positive under R   compile_source_at(C, R) is valid IIR naming R,
//!                          and running it on vm-core prints C.expect exactly
//!       negative under R   it fails with the C6 category in C.reject
//!                          (or C.reject@R), naming R
//! ```
//!
//! It also pins the release API to the inventory: the crate's own tables
//! (`supported_releases`, the unclaimed rows) must agree with the CSV, so a
//! row cannot be claimed or un-claimed in one place only. Where host `node`
//! is installed, every `.expect` is re-checked against it (the oracle the
//! `.cite` files name).

mod common;

use common::{claiming_releases, corpus, corpus_dir, inventory, Outcome};
use javascript_iir_compiler::{
    compile_source_at, resolve_release, run_source_at, supported_releases, DEFAULT_RELEASE, SUBSET,
};

#[test]
fn the_crate_claims_exactly_the_rows_the_inventory_claims() {
    let claimed = claiming_releases();
    assert_eq!(supported_releases(), claimed.as_slice());
    assert_eq!(claimed.first().map(String::as_str), Some("es2016"));
    assert_eq!(claimed.last().map(String::as_str), Some("es2025"));
    assert!(claimed.iter().any(|r| r == DEFAULT_RELEASE), "C4.4: the default claims the subset");
    for row in inventory() {
        let release = row["release"].as_str();
        let claims = row["iir_status"] == SUBSET;
        // Columns that must move together on a claiming row.
        if claims {
            assert_eq!(row["grammar_stage"], "P", "{release}");
            assert_eq!(row["grammar_status"], "partial", "{release}");
            assert_eq!(row["grammar_subset"], SUBSET, "{release}");
            assert_eq!(row["profile"], "ecmascript", "{release}");
            assert!(resolve_release(Some(release)).is_ok(), "{release}");
        } else {
            // Every other row is known to the crate, and refused by name.
            let error = resolve_release(Some(release)).unwrap_err();
            assert_eq!(error.category(), "ReleaseNotAvailable", "{release}: {error}");
            assert_eq!(error.release(), Some(release));
        }
    }
}

#[test]
fn every_corpus_case_holds_on_every_claiming_release() {
    let cases = corpus();
    let positives = cases.iter().filter(|c| c.is_positive()).count();
    assert!(positives >= 12 && cases.len() - positives >= 6, "corpus shrank: {}", cases.len());
    let mut checked = 0;
    for release in claiming_releases() {
        for case in &cases {
            let label = format!("{release} {SUBSET}/{}", case.name);
            match case.outcome_on(&release) {
                Outcome::Runs(expect) => {
                    let module = compile_source_at(&case.source, Some(&release), "corpus")
                        .unwrap_or_else(|e| panic!("{label}: {e}"));
                    assert_eq!(module.release.as_deref(), Some(release.as_str()), "{label}");
                    assert!(module.validate().is_empty(), "{label}");
                    let output = run_source_at(&case.source, Some(&release))
                        .unwrap_or_else(|e| panic!("{label}: {e} after {:?}", e.output));
                    assert_eq!(output, expect, "{label}");
                }
                Outcome::Rejected(category) => {
                    let error = compile_source_at(&case.source, Some(&release), "corpus")
                        .expect_err(&label);
                    assert_eq!(error.category(), category, "{label}: {error}");
                    assert_eq!(error.release(), Some(release.as_str()), "{label}");
                    assert!(error.to_string().starts_with(&format!("JavaScript {release}: ")), "{label}");
                }
            }
            checked += 1;
        }
    }
    eprintln!("es-number-log: {checked} (release, case) cells");
}

/// Editions disagree exactly where the language does: numeric separators
/// arrived in ES2021, so es2016–es2020 refuse `1_000_000` and es2021+ print
/// it. The overrides in the corpus must say so and nothing else.
#[test]
fn numeric_separators_divide_the_editions_at_es2021() {
    let case = corpus().into_iter().find(|c| c.name == "numeric_separators").unwrap();
    let refusing: Vec<&str> = case.overrides.keys().map(String::as_str).collect();
    assert_eq!(refusing, ["es2016", "es2017", "es2018", "es2019", "es2020"]);
    for release in claiming_releases() {
        let compiled = compile_source_at(&case.source, Some(&release), "sep");
        assert_eq!(compiled.is_ok(), release.as_str() >= "es2021", "{release}");
    }
}

/// The `.expect` files are node's output (see the `.cite` files). When node
/// is installed, re-run it: a hand-edited `.expect` cannot drift from the
/// oracle it claims. When it is not, say so; stage I above still holds.
#[test]
fn expectations_match_host_node_when_it_is_installed() {
    let dir = corpus_dir();
    let probe = std::process::Command::new("node").arg("--version").output();
    let Some(probe) = probe.ok().filter(|o| o.status.success()) else {
        eprintln!("node not installed: skipping the oracle re-check (the corpus checks above ran)");
        return;
    };
    eprintln!("oracle: node {}", String::from_utf8_lossy(&probe.stdout).trim());
    for case in corpus().into_iter().filter(|c| c.is_positive()) {
        let Outcome::Runs(expect) = &case.outcome else { unreachable!() };
        // The script path is an argument, never interpolated into a shell.
        let output = std::process::Command::new("node")
            .arg(dir.join(format!("{}.js", case.name)))
            .output()
            .unwrap();
        assert!(output.status.success(), "{}: node failed", case.name);
        assert_eq!(&String::from_utf8_lossy(&output.stdout), expect, "{}", case.name);
    }
}

/// `compile_source` (the pre-6d `String` API) is `compile_source_at(None)`:
/// same module, same messages.
#[test]
fn the_default_entry_is_the_default_release() {
    let source = "let a = 8;\nconsole.log(a / 2 / 2)\n";
    let legacy = javascript_iir_compiler::compile_source(source, "m").unwrap();
    let selected = compile_source_at(source, None, "m").unwrap();
    assert_eq!(format!("{legacy:?}"), format!("{selected:?}"));
    assert_eq!(legacy.release.as_deref(), Some(DEFAULT_RELEASE));
    let error = javascript_iir_compiler::compile_source("var x = 1;", "m").unwrap_err();
    let typed = compile_source_at("var x = 1;", None, "m").unwrap_err();
    assert_eq!(error, typed.message());
    assert!(!error.starts_with("JavaScript es2020"), "the legacy text has no release prefix");
}

/// Typed refusals for requests that select nothing (LANG84 C4).
#[test]
fn unknown_and_unclaimed_requests_compile_nothing() {
    for (request, category) in [
        ("es5", "ReleaseNotAvailable"),
        ("es2015", "ReleaseNotAvailable"),
        ("es2026", "ReleaseNotAvailable"),
        ("js1.8.5", "ReleaseNotAvailable"),
        ("ES2020", "UnknownRelease"),
        ("2020", "UnknownRelease"),
        ("", "UnknownRelease"),
    ] {
        let error = compile_source_at("console.log(1);", Some(request), "m").unwrap_err();
        assert_eq!(error.category(), category, "{request:?}: {error}");
        let run = run_source_at("console.log(1);", Some(request)).unwrap_err();
        assert!(run.output.is_empty(), "{request:?}");
    }
}

/// The source budget is charged against the selected edition and reported
/// as a Budget, with the legacy text.
#[test]
fn the_source_budget_is_a_typed_budget_error() {
    let big = "1;".repeat(javascript_iir_compiler::MAX_SOURCE_BYTES / 2 + 1);
    let error = compile_source_at(&big, Some("es2017"), "m").unwrap_err();
    assert_eq!(error.category(), "Budget");
    assert_eq!(error.release(), Some("es2017"));
    assert!(error.message().contains("native pilot limit"), "{error}");
}

/// Review L1: an error's category comes from the producer's fixed text, so
/// source that *quotes* a budget phrase is still an ordinary Parse error.
/// Before, `console.log(1 "spine limit")` was reported as a Budget refusal,
/// because the parse message echoed the string and was matched by substring.
#[test]
fn source_text_cannot_choose_an_error_category() {
    for release in supported_releases() {
        for source in [
            "console.log(1 \"spine limit\");",
            "console.log(1 'nests deeper than the supported limit');",
            "console.log(1 'input within the supported nesting limit');",
            "console.log(1 'JavaScript expression chain ');",
            "console.log(1 'node limit');",
            "console.log(1 'depth limit');",
        ] {
            let error = compile_source_at(source, Some(release), "m").unwrap_err();
            assert_eq!(error.category(), "Parse", "{release} {source:?}: {error}");
        }
    }
    // The real budget refusals keep their category.
    let deep = format!("console.log({}1{});", "(".repeat(400), ")".repeat(400));
    let error = compile_source_at(&deep, Some("es2021"), "m").unwrap_err();
    assert_eq!(error.category(), "Budget", "{error}");
}

/// Review M1: on every claiming edition, a numeric spelling node rejects is
/// refused (as the SyntaxError node reports, category Parse), and every
/// spelling node accepts gets node's value. Before, es2021+ printed
/// `console.log(0_17)` as 15 and `1__0` as 10.
#[test]
fn numeric_literals_follow_node_on_every_edition() {
    for release in supported_releases() {
        for invalid in ["0_17", "01_7", "0_8", "1__0", "0x_F", "1_.5", "1._5", "1e_5", "1_e5", "100_", "017.5", "017e1", "0_0", "09_1"] {
            let source = format!("console.log({invalid});");
            let error = compile_source_at(&source, Some(release), "m").unwrap_err();
            assert_eq!(error.category(), "Parse", "{release} {invalid}: {error}");
        }
        for (valid, shown) in [("5.", "5"), ("5.e1", "50"), ("1.e3", "1000"), ("08.5", "8.5"), ("08e1", "80"), ("017", "15"), ("00", "0"), ("0e1", "0")] {
            let output = run_source_at(&format!("console.log({valid});"), Some(release)).unwrap();
            assert_eq!(output, format!("{shown}\n"), "{release} {valid}");
        }
    }
}
