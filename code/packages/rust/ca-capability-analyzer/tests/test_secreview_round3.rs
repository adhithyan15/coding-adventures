//! # Round-3 security review regressions (VER08a divergence D25 onwards)
//!
//! An independent review built the analyzer and found inputs for which it
//! printed `PASS` (exit 0) while `cargo check` compiled the capability in.
//! Every test here reproduces one of those proofs of concept (PoC `t1`..`t13`)
//! and pins the fail-closed verdict, next to decoys that must stay clean.
//!
//! Verdict notation is the one used by `test_evasion_corpus.rs`:
//!
//! * `cat:action` - a capability detection (CAP001 candidate)
//! * `!construct` - a banned construct (CAP002 candidate)
//! * `?`          - an unanalysable construct (CAP003, never waivable)

mod common;

use coding_adventures_ca_capability_analyzer::{analyze_package, scan_source, AnalysisReport, ScanResult};
use common::{pure_manifest, Pkg};

fn verdict_of(res: &ScanResult) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for d in &res.detections {
        v.push(format!("{}:{}", d.category, d.action));
    }
    for b in &res.banned {
        v.push(format!("!{}", b.construct));
    }
    if !res.unanalyzable.is_empty() {
        v.push("?".to_string());
    }
    v.sort();
    v.dedup();
    v
}

fn verdict(src: &str) -> Vec<String> {
    let res = scan_source("src/case.rs", src).unwrap_or_else(|e| panic!("scan error: {e}\n{src}"));
    verdict_of(&res)
}

type Case = (&'static str, &'static str, &'static [&'static str]);

/// Run a group of cases and report every mismatch at once.
fn check(label: &str, cases: &[Case]) {
    let mut failures = Vec::new();
    for (name, src, expected) in cases {
        let got = verdict(src);
        let mut want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        want.sort();
        if got != want {
            failures.push(format!("[{label}] {name}\n  src: {src:?}\n  want: {want:?}\n  got:  {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{} case(s) failed:\n{}", failures.len(), failures.join("\n"));
}

/// Analyse a temporary package; the PoC packages carry no manifest unless given one.
fn analyze_files(tag: &str, files: &[(&str, &str)]) -> AnalysisReport {
    let p = Pkg::with(tag, files);
    analyze_package(p.dir(), false).unwrap_or_else(|e| panic!("{tag}: {e}"))
}

/// The report must fail (exit 1) with at least one CAP003.
fn assert_cap003(tag: &str, r: &AnalysisReport) {
    let codes: Vec<&str> = r.violations.iter().map(|v| v.code()).collect();
    assert!(codes.contains(&"CAP003"), "{tag}: expected CAP003, got {codes:?}\n{}", r.format_text(true));
    assert_eq!(r.exit_code(), 1, "{tag}");
}

const CARGO_2021: &str = "[package]\nname = \"poc\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";

// ---------------------------------------------------------------------------
// HIGH-3: a leading `#!` is a shebang line exactly when rustc says it is
// ---------------------------------------------------------------------------

#[test]
fn high3_shebang_line_is_skipped_like_rustc_does() {
    check(
        "HIGH-3",
        &[
            // PoC t1: the `"` on the shebang line opened a string in the analyzer's
            // reading that swallowed the real code up to the `"` in the comment.
            (
                "t1 shebang with a quote",
                "#!/x\"\npub fn f() { std::fs::write(\"a\", \"b\").unwrap(); }\n// \"\n",
                &["fs:write"],
            ),
            ("shebang with block comment opener", "#!/x /*\npub fn f() { std::process::exit(1); }\n// */\n", &["proc:signal"]),
            ("shebang with raw string opener", "#!/x r#\"\nfn f() { std::env::var(\"X\"); }\n// \"#\n", &["env:read"]),
            ("BOM then shebang", "\u{feff}#!/x\"\nfn f() { std::fs::read(\"a\"); }\n// \"\n", &["fs:read"]),
            ("shebang then CRLF", "#!/usr/bin/env x\"\r\nfn f() { std::fs::read(\"a\"); }\r\n", &["fs:read"]),
            // `#!` followed (after whitespace and plain comments) by `[` is an inner
            // attribute, not a shebang.
            ("inner attribute", "#![no_mangle]\nfn f(){}", &["!foreign_linkage"]),
            ("inner attribute after spaces", "#!   [no_mangle]\nfn f(){}", &["!foreign_linkage"]),
            ("inner attribute after newline", "#!\n[no_mangle]\nfn f(){}", &["!foreign_linkage"]),
            ("inner attribute after comments", "#! /* c */ // d\n [no_mangle]\nfn f(){}", &["!foreign_linkage"]),
            // A doc comment is a token, so `#! //! x` + `[` is a shebang line.
            ("doc comment makes it a shebang", "#! //! x\n[no_mangle]\nfn f(){}", &[]),
            // Not at offset 0: an ordinary `#` `!` pair.
            ("not at file start", "\n#![no_mangle]", &["!foreign_linkage"]),
            ("decoy shebang alone", "#!/usr/bin/env run-cargo-script\nfn f(){}", &[]),
        ],
    );
}
