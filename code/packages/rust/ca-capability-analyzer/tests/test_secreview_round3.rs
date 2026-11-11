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

// ---------------------------------------------------------------------------
// HIGH-4: `include!` is CAP003 however it is reached through std/core/alloc
// ---------------------------------------------------------------------------

#[test]
fn high4_include_through_the_prelude_is_cap003() {
    check(
        "HIGH-4",
        &[
            // PoC t3: the prelude re-exports `include`, and `prelude` is a pure module.
            ("t3 std::prelude::v1::include!", "std::prelude::v1::include!(\"payload.txt\");", &["?"]),
            ("core::prelude::rust_2021::include!", "core::prelude::rust_2021::include!(\"payload.txt\");", &["?"]),
            ("::std::prelude::rust_2024::include!", "::std::prelude::rust_2024::include!(\"payload.txt\");", &["?"]),
            ("use .. include as i", "use std::prelude::v1::include as i;\ni!(\"payload.txt\");", &["?"]),
            ("use .. include alone", "use core::prelude::v1::include;", &["?"]),
            ("use group", "use std::prelude::v1::{include, Vec};", &["?"]),
            ("alias of the prelude module", "use std::prelude::v1 as p; p::include!(\"x\");", &["?"]),
            ("re-export used through crate", "pub use std::prelude::v1::include as inc; crate::inc!(\"x\");", &["?"]),
            // Still data, not code (VER08a L-B): allowed through any path.
            ("decoy prelude include_str!", "const S: &str = std::prelude::v1::include_str!(\"x\");", &[]),
            ("decoy prelude include_bytes!", "const S: &[u8] = core::prelude::v1::include_bytes!(\"x\");", &[]),
            ("decoy prelude Vec", "fn f() { let _ = std::prelude::v1::Vec::<u8>::new(); }", &[]),
        ],
    );
}

// ---------------------------------------------------------------------------
// HIGH-1: glob re-exports through a local module
// ---------------------------------------------------------------------------

#[test]
fn high1_glob_reexport_through_a_local_module_is_resolved() {
    check(
        "HIGH-1",
        &[
            // PoC t12: `m::process::Command` became `std::m::process::Command`
            // (an unknown std module, whose CAP003 globs suppressed): PASS.
            (
                "t12 pub use std::* in a module",
                "mod m { pub use std::*; }\npub fn f() { let _ = m::process::Command::new(\"sh\").arg(\"-c\").arg(\"id\").status(); }",
                &["?", "proc:exec"],
            ),
            // PoC t13: `m::remove_dir_all` became `std::fs::m::remove_dir_all`: fs:read only.
            (
                "t13 pub use std::fs::* in a module",
                "mod m { pub use std::fs::*; }\npub fn f() { m::remove_dir_all(\"/tmp/x\").unwrap(); }",
                &["fs:delete", "fs:read"],
            ),
            ("crate-rooted path through the glob module", "mod m { pub use std::fs::*; }\nfn f() { crate::m::remove_file(\"x\"); }", &["fs:delete", "fs:read"]),
            ("self-rooted path inside the glob module", "mod m { use std::fs::*; fn f() { self::remove_file(\"x\"); } }", &["fs:delete", "fs:read"]),
            ("alias of an item reached through the glob", "mod m { pub use std::fs::*; }\nuse crate::m::remove_dir_all as r;\nfn f() { r(\"x\"); }", &["fs:delete", "fs:read"]),
            ("two-level glob chain", "mod m { pub use std::*; }\nmod n { pub use crate::m::*; }\nfn f() { n::env::var(\"X\"); }", &["?", "env:read"]),
            ("glob over a crate-rooted std alias", "pub use std as s;\nmod m { pub use crate::s::*; }\nfn f() { m::process::Command::new(\"x\"); }", &["?", "proc:exec"]),
            ("glob over a self-rooted std alias", "use std::fs as q;\nmod m { pub use self::q::*; }\nfn f() { m::remove_file(\"x\"); }", &["fs:delete", "fs:read"]),
            ("decoy pure std glob through a module", "mod m { pub use std::fmt::*; }\nfn f() -> m::Result { Ok(()) }", &[]),
            ("decoy local globs stay local", "use crate::m::*; use super::*; fn f() { read(1); remove_file(2); }", &[]),
        ],
    );
}

#[test]
fn high1_a_glob_over_a_bare_std_root_is_cap003() {
    for src in [
        "use std::*;",
        "use core::*;",
        "use alloc::*;",
        "pub use ::std::*;",
        "use std::{*};",
        "use std as s; use s::*;",
        "extern crate core as c; use c::{*};",
        "pub use std as s; mod m { pub use crate::s::*; }",
    ] {
        assert!(verdict(src).contains(&"?".to_string()), "{src}: {:?}", verdict(src));
    }
    // A glob over a std *module* is resolved, not refused.
    assert_eq!(verdict("use std::fmt::*;"), Vec::<String>::new());
    assert_eq!(verdict("use core::prelude::v1::*;"), Vec::<String>::new());
}

#[test]
fn low_glob_expansion_is_capped_counting_pure_globs() {
    // 60 bindings of one name (under the 64-spellings-per-path limit) times 100
    // globs through it: 6,000 glob spellings, every one of them pure.  The old
    // cap counted only non-pure globs, after materialising the whole product.
    let mut src = String::new();
    for i in 0..60 {
        src.push_str(&format!("use p{i} as h;\n"));
    }
    for k in 0..100 {
        src.push_str(&format!("use h::g{k}::*;\n"));
    }
    let start = std::time::Instant::now();
    let got = verdict(&src);
    assert!(got.contains(&"?".to_string()), "{got:?}");
    assert!(start.elapsed() < std::time::Duration::from_secs(10));
}

// ---------------------------------------------------------------------------
// MEDIUM-1: the `.read_dir()` receiver waiver must not trust a type name a
// glob import may supply
// ---------------------------------------------------------------------------

const T11_LOCAL_TYPE: &str = "mod a {\n    pub struct PathBuf;\n    impl PathBuf {\n        pub fn from(_: &str) -> Self { PathBuf }\n        pub fn read_dir(&self) {}\n    }\n}\n";

#[test]
fn medium1_receiver_waiver_is_refused_under_a_foreign_glob() {
    let with = |tail: &str| format!("{T11_LOCAL_TYPE}{tail}");
    let t11 = with("use std::path::*;\npub fn f() -> usize { PathBuf::from(\"/etc\").read_dir().unwrap().count() }\n");
    let third_party = with("use dep::*;\npub fn f() { PathBuf::from(\"/etc\").read_dir(); }\n");
    let local_glob = with("use crate::a::*;\npub fn f() { PathBuf::from(\"/etc\").read_dir(); }\n");
    let no_glob = with("use crate::a::PathBuf as P;\npub fn f() { a::PathBuf::from(\"/etc\").read_dir(); }\n");
    // PoC t11: `PathBuf` at the root is std's (from the glob); the local type
    // with its own `read_dir` lives in `mod a`.  The waiver made this pure.
    assert!(verdict(&t11).contains(&"fs:list".to_string()), "t11: {:?}", verdict(&t11));
    assert!(verdict(&third_party).contains(&"fs:list".to_string()), "third-party glob: {:?}", verdict(&third_party));
    // Local globs only name items of this crate: the waiver still applies.
    assert_eq!(verdict(&local_glob), Vec::<String>::new());
    // (`a::PathBuf::from` is not a simple local path: never waived, as before.)
    assert_eq!(verdict(&no_glob), vec!["fs:list".to_string()]);
}

// ---------------------------------------------------------------------------
// HIGH-2: `macro_rules!` transcribers that splice path or attribute tokens
// ---------------------------------------------------------------------------

#[test]
fn high2_metavariable_splices_into_paths_and_attributes_are_cap003() {
    for (name, src) in [
        // PoC t2: `$s` is `::`, so the expansion is `std::fs::write(..)`; neither
        // the transcriber nor the call site ever shows `std::fs` contiguously.
        ("t2 tt splices ::", "macro_rules! m { ($s:tt) => { std $s fs $s write(\"a\", \"b\").unwrap() } }\npub fn f() { m!(::); }"),
        // PoC t6: `# $a` with `$a = [path = ".."]` is `#[path = ".."]` unseen
        // (and `[no_mangle]` would be `#[no_mangle]` without CAP002).
        ("t6 # then metavariable", "macro_rules! m { ($a:tt) => { # $a pub mod hidden; } }\nm!([path = \"../payload.txt\"]);\npub fn f() { hidden::g() }"),
        ("#! then metavariable", "macro_rules! m { ($a:tt) => { #! $a } }\nm!([no_mangle]);"),
        ("# then repetition", "macro_rules! m { ($($a:tt)*) => { # $($a)* fn f() {} } }"),
        ("lone # before a delimiter", "macro_rules! m { ($($x:ident)*) => { $( $x # )* [path = \"x\"] mod hidden; } }"),
        ("# as repetition separator", "macro_rules! m { ($($x:ident),*) => { $( $x )#* [no_mangle] fn f() {} } }"),
        ("tt metavariable before [", "macro_rules! m { ($h:tt) => { $h [path = \"x\"] mod hidden; } }\nm!(#);"),
        ("tt metavariable before ! [", "macro_rules! m { ($h:tt) => { $h ! [no_mangle] } }"),
        ("tt metavariable before ident", "macro_rules! m { ($s:tt) => { $s fs::write(\"a\", \"b\") } }"),
        ("ident before tt metavariable", "macro_rules! m { ($s:tt) => { std $s } }"),
        ("tt next to tt", "macro_rules! m { ($a:tt, $b:tt, $c:tt, $d:tt, $e:tt) => { $a $b $c $d $e(\"a\", \"b\") } }\npub fn f() { m!(std, ::, fs, ::, write); }"),
        ("tt next to ident metavariable", "macro_rules! m { ($s:tt, $a:ident) => { $s $a $s fs } }"),
        ("include then ! from tt", "macro_rules! m { ($b:tt) => { include $b (\"../x.rs\") } }\nm!(!);"),
        ("tt repetition next to ident", "macro_rules! m { ($($t:tt)*) => { $($t)* write(\"a\", \"b\") } }\npub fn f() { m!(std::fs::); }"),
        ("separator dropped between tt repetitions", "macro_rules! m { ($($t:tt),*) => { $($t)* (\"a\", \"b\") } }\npub fn f() { m!(std, ::, fs, ::, write); }"),
        ("tt repetition with ident separator", "macro_rules! m { ($($t:tt)*) => { $($t)fs* } }"),
        ("unknown metavariable is treated as tt", "macro_rules! m { () => { std $x fs } }"),
        ("crate path keyword next to tt", "macro_rules! m { ($s:tt) => { crate $s helper() } }"),
    ] {
        let got = verdict(src);
        assert!(got.contains(&"?".to_string()), "{name}: expected CAP003, got {got:?}\n{src}");
    }
}

#[test]
fn high2_ordinary_macro_rules_stay_clean() {
    check(
        "HIGH-2 decoys",
        &[
            ("visibility and name", "macro_rules! m { ($vis:vis $name:ident) => { $vis struct $name; } }", &[]),
            ("fn with typed params", "macro_rules! m { ($name:ident, $t:ty) => { fn $name(x: $t) -> $t { x } } }", &[]),
            ("index an expression", "macro_rules! m { ($x:expr) => { $x[0] } }", &[]),
            ("impl block", "macro_rules! m { ($i:ident) => { impl $i { fn new() -> Self { $i } } } }", &[]),
            ("tt passthrough", "macro_rules! m { ($($t:tt)*) => { $($t)* } }", &[]),
            ("tt passthrough into another macro", "macro_rules! m { ($($t:tt)*) => { foo!($($t)*) } }", &[]),
            ("tt passthrough with plus", "macro_rules! m { ($($t:tt)+) => { [$($t)+] } }", &[]),
            ("tt operand in an expression", "macro_rules! m { ($t:tt) => { $t + 1 } }", &[]),
            ("tt after keyword", "macro_rules! m { ($t:tt) => { return $t; } }", &[]),
            ("tt in a let", "macro_rules! m { ($t:tt) => { let x = $t; } }", &[]),
            ("idents next to each other", "macro_rules! m { ($a:ident $b:ident) => { $a + $b } }", &[]),
            ("mod with an ident name", "macro_rules! m { ($name:ident) => { mod $name {} } }", &[]),
            ("forwarded attributes are whole", "macro_rules! m { ($(#[$a:meta])* $n:ident) => { #[derive(Debug)] struct $n; } }", &[]),
            ("separated repetition", "macro_rules! m { ($($x:expr),*) => { vec![$($x),*] } }", &[]),
            ("$crate path", "macro_rules! m { () => { $crate::helper() } }", &[]),
            ("several arms", "macro_rules! m { () => { 0 }; ($x:expr) => { $x }; ($x:expr, $($y:expr),+) => { $x + m!($($y),+) } }", &[]),
        ],
    );
}

// ---------------------------------------------------------------------------
// HIGH-5: root-level files must not reach the directories the scanner skips
// ---------------------------------------------------------------------------

fn files_verdict(files: &[(&str, &str)]) -> Vec<String> {
    verdict_of(&coding_adventures_ca_capability_analyzer::scan_sources(files).unwrap())
}

#[test]
fn high5_root_level_mod_into_a_skipped_directory_is_cap003() {
    // PoC t5: build.rs (always root-level) declares `mod examples;`, which
    // compiles `examples/mod.rs` - a directory the scanner skips at the root.
    let r = analyze_files(
        "t5",
        &[
            ("Cargo.toml", CARGO_2021),
            ("src/lib.rs", "pub fn f() {}\n"),
            ("build.rs", "mod examples;\nfn main() { examples::go(); }\n"),
            ("examples/mod.rs", "pub fn go() { let _ = std::fs::write(std::env::var(\"OUT_DIR\").unwrap() + \"/pwn\", \"x\"); }\n"),
        ],
    );
    assert_cap003("t5", &r);
    // PoC t10: `[lib] path = "lib.rs"` makes lib.rs the crate root, and
    // `mod tests;` compiles `tests/mod.rs`.
    let r = analyze_files(
        "t10",
        &[
            ("Cargo.toml", &format!("{CARGO_2021}autotests = false\n\n[lib]\npath = \"lib.rs\"\n")),
            ("lib.rs", "mod tests;\npub fn f() { tests::go() }\n"),
            ("tests/mod.rs", "pub fn go() { std::fs::write(\"a\", \"b\").unwrap(); }\n"),
            ("required_capabilities.json", &pure_manifest("t10")),
        ],
    );
    assert_cap003("t10", &r);

    for (name, files) in [
        ("inline module holding an out-of-line one", vec![("lib.rs", "mod tests { mod x; }")]),
        ("case-insensitive filesystems", vec![("build.rs", "mod Tests;\nfn main() {}")]),
        ("raw identifier", vec![("build.rs", "mod r#benches;\nfn main() {}")]),
        ("every skipped root", vec![("main.rs", "mod target; mod node_modules; mod _build; fn main() {}")]),
        ("root file named like a skipped directory", vec![("lib.rs", "mod examples;"), ("examples.rs", "mod x;")]),
        ("`mod` passed as a token in a root file", vec![("build.rs", "m!(mod);\nfn main() {}")]),
        ("module name from a metavariable", vec![("src/lib.rs", "macro_rules! mk { ($n:ident) => { mod $n; } }")]),
        ("module body from a metavariable", vec![("src/lib.rs", "macro_rules! mk { ($n:ident, $($b:tt)*) => { mod $n { $($b)* } } }")]),
    ] {
        let got = files_verdict(&files);
        assert!(got.contains(&"?".to_string()), "{name}: expected CAP003, got {got:?}");
    }

    // Decoys: modules under src/ and root modules with ordinary names are scanned.
    assert_eq!(files_verdict(&[("src/lib.rs", "mod tests;"), ("src/tests.rs", "fn t() {}")]), Vec::<String>::new());
    assert_eq!(files_verdict(&[("build.rs", "mod helpers;\nfn main() {}"), ("helpers.rs", "pub fn h() {}")]), Vec::<String>::new());
    assert_eq!(files_verdict(&[("src/lib.rs", "#[cfg(test)]\nmod tests { #[test] fn t() {} }")]), Vec::<String>::new());
    assert_eq!(files_verdict(&[("src/lib.rs", "macro_rules! mk { ($n:ident) => { mod $n { pub fn f() {} } } }")]), Vec::<String>::new());
}

// ---------------------------------------------------------------------------
// LOW: `--generate-module` must not follow a symlinked `src` or target file
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn low_generate_module_refuses_symlinks() {
    use coding_adventures_ca_capability_analyzer::write_module_to_disk;
    use std::os::unix::fs::symlink;

    // `src` is a symlink to a directory outside the package.
    let outside = Pkg::empty("symlink_outside");
    let p = Pkg::with("symlink_src", &[("required_capabilities.json", &pure_manifest("s"))]);
    symlink(outside.dir(), p.path("src")).unwrap();
    assert!(write_module_to_disk(p.dir()).is_err());
    assert!(!outside.path("capabilities.rs").exists(), "wrote through a symlinked src/");

    // `src/capabilities.rs` is a symlink to a file elsewhere.
    let victim = Pkg::with("symlink_victim", &[("victim.txt", "keep me")]);
    let q = Pkg::with("symlink_file", &[("required_capabilities.json", &pure_manifest("s")), ("src/lib.rs", "")]);
    symlink(victim.path("victim.txt"), q.path("src/capabilities.rs")).unwrap();
    assert!(write_module_to_disk(q.dir()).is_err());
    assert_eq!(victim.read("victim.txt"), "keep me", "wrote through a symlinked src/capabilities.rs");

    // A regular package still works.
    let ok = Pkg::with("symlink_none", &[("required_capabilities.json", &pure_manifest("s")), ("src/lib.rs", "")]);
    write_module_to_disk(ok.dir()).unwrap();
    assert!(ok.read("src/capabilities.rs").contains("PureCapabilities"));
}

// ---------------------------------------------------------------------------
// INFO: `File::create_buffered` creates/truncates like `File::create`
// ---------------------------------------------------------------------------

#[test]
fn info_file_create_buffered_is_fs_write() {
    check(
        "INFO",
        &[
            // PoC t7: classified as fs:read, so a read-only manifest passed.
            ("t7 std::fs::File::create_buffered", "pub fn f() { let _ = std::fs::File::create_buffered(\"x\"); }", &["fs:write"]),
            ("imported File", "use std::fs::File; fn f() { let _ = File::create_buffered(\"x\"); }", &["fs:read", "fs:write"]),
            ("decoy open_buffered reads", "fn f() { let _ = std::fs::File::open_buffered(\"x\"); }", &["fs:read"]),
        ],
    );
}
