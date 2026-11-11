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

// ---------------------------------------------------------------------------
// HIGH-6: Unicode whitespace and identifiers are lexed as rustc lexes them,
// or the file fails closed
// ---------------------------------------------------------------------------

#[test]
fn high6_pattern_white_space_separates_tokens_like_rustc() {
    check(
        "HIGH-6 whitespace",
        &[
            // PoC t9: U+200E LEFT-TO-RIGHT MARK is Pattern_White_Space, so rustc
            // reads `std::fs::write`; the analyzer read `std::fs` and a stray char.
            ("t9 LRM between segments", "pub fn f() { std::fs::\u{200e}write(\"a\", \"b\").unwrap(); }", &["fs:write"]),
            ("RLM between segments", "fn f() { std::fs::\u{200f}remove_file(\"a\"); }", &["fs:delete"]),
            ("LINE SEPARATOR", "fn f() { std::fs::\u{2028}remove_file(\"a\"); }", &["fs:delete"]),
            ("PARAGRAPH SEPARATOR", "fn f() { std\u{2029}::process::Command::new(\"a\"); }", &["proc:exec"]),
            ("NEXT LINE", "fn f() { std::\u{85}env::set_var(\"a\", \"b\"); }", &["env:write"]),
            ("vertical tab", "fn f() { std::fs::\u{0b}remove_file(\"a\"); }", &["fs:delete"]),
            ("decoy unicode in strings, chars and comments", "const S: &str = \"caf\u{e9} \u{200e}\"; // \u{2603} \u{a0}\n/* \u{3000} */ const C: char = '\u{2603}';", &[]),
        ],
    );
}

#[test]
fn high6_other_non_ascii_outside_literals_is_cap003() {
    for (name, src) in [
        // PoC t4: U+00B7 MIDDLE DOT is XID_Continue, so rustc's alias is `s·x`;
        // the analyzer read the alias `s` and never resolved `s·x::fs`.
        ("t4 middle dot in extern crate alias", "extern crate std as s\u{b7}x;\npub fn f() { s\u{b7}x::fs::write(\"a\", \"b\").unwrap(); }"),
        ("non-ASCII identifier", "fn f() { let caf\u{e9} = 1; }"),
        ("non-ASCII identifier start", "fn \u{e9}t\u{e9}() {}"),
        ("non-ASCII raw identifier", "fn f() { let r#caf\u{e9} = 1; }"),
        ("non-ASCII lifetime", "fn f<'\u{e9}>() {}"),
        ("NBSP is not Rust whitespace", "fn f() { std::fs::\u{a0}write(\"a\", \"b\"); }"),
        ("IDEOGRAPHIC SPACE", "fn f() {\u{3000}}"),
        ("byte-order mark after the start", "fn f() {}\u{feff}"),
        ("zero-width space", "fn f() { std::fs\u{200b}::write(\"a\", \"b\"); }"),
        ("non-ASCII suffix after a literal", "const X: u8 = 1\u{e9};"),
    ] {
        let got = verdict(src);
        assert!(got.contains(&"?".to_string()), "{name}: expected CAP003, got {got:?}\n{src:?}");
    }
}

#[test]
fn high6_extern_crate_alias_must_end_at_a_semicolon() {
    check(
        "HIGH-6 extern crate",
        &[
            ("alias followed by more tokens", "extern crate std as s ::x;\nfn f(){ s::fs::read(\"a\"); }", &["?"]),
            ("name followed by junk", "extern crate std x;", &["?"]),
            ("alias missing", "extern crate std as;", &["?"]),
            ("unterminated", "extern crate std as s", &["?"]),
            ("decoy plain extern crate", "extern crate alloc;", &[]),
            ("decoy extern crate self alias", "extern crate self as me;", &[]),
            ("decoy extern crate alias", "extern crate core as c; fn f() -> c::primitive::u8 { 0 }", &[]),
            ("decoy underscore alias", "extern crate std as _;", &[]),
        ],
    );
}

// ---------------------------------------------------------------------------
// HIGH-7: C-string prefixes exist only from edition 2021
// ---------------------------------------------------------------------------

/// PoC t8 without its `macro_rules!` definition: in edition 2018 `cr#"` is the
/// identifier `cr`, a `#` and an ordinary string, so the block is real code; in
/// 2021 the whole thing is one raw C string.
const T8_INVOCATION: &str = "m!{ cr#\" a \" { std::fs::write(\"a\", \"b\").unwrap(); } \" \"# }\n";

fn cargo_with(edition_line: &str) -> String {
    format!("[package]\nname = \"poc\"\nversion = \"0.1.0\"\n{edition_line}\n")
}

#[test]
fn high7_c_string_prefixes_follow_the_package_edition() {
    use coding_adventures_ca_capability_analyzer::cargo::analyze_cargo_text;
    use coding_adventures_ca_capability_analyzer::parser::Edition;

    for (toml, want) in [
        ("edition = \"2015\"", Edition::E2015),
        ("edition = \"2018\"", Edition::E2018),
        ("edition = \"2021\"", Edition::E2021),
        ("edition = \"2024\"", Edition::E2024),
        ("edition = \"2027\"", Edition::Unknown),
        ("", Edition::Unknown),
        ("edition.workspace = true", Edition::Unknown),
        ("edition = { workspace = true }", Edition::Unknown),
        ("edition = \"2021\"\n[lib]\nedition = \"2018\"", Edition::Unknown),
        ("edition = \"2021\"\n[[bin]]\nname = \"b\"\nedition = \"2018\"", Edition::Unknown),
        ("edition = \"2021\"\nbin = [{ name = \"b\", edition = \"2015\" }]", Edition::Unknown),
        ("edition = '''2021'''", Edition::Unknown),
        // `[workspace.package]` only feeds members that opt in; it does not set this edition.
        ("edition = \"2018\"\n[workspace.package]\nedition = \"2021\"", Edition::E2018),
    ] {
        assert_eq!(analyze_cargo_text(&cargo_with(toml)).edition, want, "{toml:?}");
    }

    // PoC t8 (edition 2018): the analyzer lexed a raw C string and printed PASS.
    for ed in ["2015", "2018"] {
        let r = analyze_files("t8", &[("Cargo.toml", &cargo_with(&format!("edition = \"{ed}\""))), ("src/lib.rs", T8_INVOCATION)]);
        assert!(r.detected_capabilities.contains("fs:write:*"), "{ed}: {}", r.format_text(true));
        assert_eq!(r.exit_code(), 1, "{ed}");
    }

    // Edition 2021 and later: one raw C string, nothing to report.
    for ed in ["2021", "2024"] {
        let r = analyze_files("t8c", &[("Cargo.toml", &cargo_with(&format!("edition = \"{ed}\""))), ("src/lib.rs", T8_INVOCATION)]);
        assert!(r.is_clean(), "{ed}: {}", r.format_text(true));
    }

    // Unknown edition: the raw C-string reading is ambiguous, so it is CAP003.
    for line in ["", "edition.workspace = true", "edition = \"2021\"\n[lib]\nedition = \"2018\""] {
        let r = analyze_files("t8d", &[("Cargo.toml", &cargo_with(line)), ("src/lib.rs", T8_INVOCATION)]);
        assert_cap003(line, &r);
    }
    // No Cargo.toml at all: same.
    assert_cap003("no Cargo.toml", &analyze_files("t8e", &[("src/lib.rs", T8_INVOCATION)]));
    // A cooked `c"..."` has the same string boundaries in every edition: never ambiguous.
    let r = analyze_files("t8f", &[("src/lib.rs", "const S: &str = c\"x\\\" std::fs::write \";\n")]);
    assert!(r.is_clean(), "{}", r.format_text(true));
}
