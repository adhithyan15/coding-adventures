//! # Round-4 security review regressions (VER08a divergence D34 onwards)
//!
//! A second review of the round-3 fixes built small crates that the analyzer
//! passed (exit 0) while `cargo check` compiled a capability call into them.
//! Every test here reproduces one of those cases (the case names are the
//! review's directory names) and pins the fail-closed verdict, next to decoys
//! that must stay clean so the fix does not flood real packages with
//! false positives.
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

/// Every source must produce at least one CAP003.
fn check_cap003(label: &str, cases: &[(&str, &str)]) {
    let mut failures = Vec::new();
    for (name, src) in cases {
        let got = verdict(src);
        if !got.contains(&"?".to_string()) {
            failures.push(format!("[{label}] {name}: expected CAP003, got {got:?}\n  src: {src:?}"));
        }
    }
    assert!(failures.is_empty(), "{} case(s) failed:\n{}", failures.len(), failures.join("\n"));
}

fn analyze_files(tag: &str, files: &[(&str, &str)]) -> AnalysisReport {
    let p = Pkg::with(tag, files);
    analyze_package(p.dir(), false).unwrap_or_else(|e| panic!("{tag}: {e}"))
}

fn cap003_reasons(r: &AnalysisReport) -> Vec<String> {
    r.violations
        .iter()
        .filter(|v| v.code() == "CAP003")
        .map(|v| v.format_line())
        .collect()
}

const CARGO_2021: &str = "[package]\nname = \"poc\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";

// ---------------------------------------------------------------------------
// H1: a rename is followed through every segment, to a fixed point
// ---------------------------------------------------------------------------

#[test]
fn h1_renames_resolve_through_every_segment() {
    check(
        "H1",
        &[
            (
                "alias2-whole-std",
                "mod a { pub use std as s; }\nuse a::s as t;\npub fn f() -> bool { t::env::var(\"HOME\").is_ok() }",
                &["env:read"],
            ),
            (
                "alias2-module-io",
                "mod a { pub use std::io as i; }\nuse a::i as j;\npub fn f() -> bool { use std::io::Write; j::stdout().flush().is_ok() }",
                &["stdout:write"],
            ),
            (
                "alias2-module-path",
                "mod a { pub use std::path::Path as Q; }\nuse a::Q as P;\npub fn f() -> bool { P::exists(P::new(\"x\")) }",
                &["fs:list"],
            ),
            (
                "alias2-crate-path",
                "use std::path::Path as Q;\nuse crate::Q as P;\npub fn f() -> bool { P::exists(P::new(\"x\")) }",
                &["fs:list"],
            ),
            (
                "alias2-super",
                "use std::io as i;\npub mod inner { use super::i as j; pub fn f() { j::stdout(); } }",
                &["stdout:write"],
            ),
            (
                "alias2-glob",
                "mod a { pub use std::io as i; }\nuse a::i as j;\nuse j::*;\npub fn f() { stdout(); }",
                &["stdout:write"],
            ),
            (
                "extern-crate-nested-2level",
                "mod m { pub extern crate std as s2; }\nuse m::s2 as s3;\npub fn f() { s3::io::stdout(); }",
                &["stdout:write"],
            ),
            (
                "three levels through modules",
                "mod a { pub use std as s; }\nmod b { pub use crate::a::s as u; }\nuse b::u as v;\npub fn f() { v::process::exit(1); }",
                &["proc:signal"],
            ),
            (
                "G-8: a crate-rooted type alias reaches a same-named binding",
                "mod m { pub use std::fs::File; }\ntype F = super::File;\npub fn f() { F::create(\"x\"); }",
                &["fs:read", "fs:write"],
            ),
            (
                "rename inside a use group",
                "mod a { pub use std as s; }\nuse a::{s as t};\npub fn f() { t::env::var(\"X\"); }",
                &["env:read"],
            ),
        ],
    );
}

#[test]
fn h1_include_through_a_two_level_rename_is_cap003() {
    check_cap003(
        "H1 include",
        &[(
            "alias2-include",
            "mod a { pub use std as s; }\nuse a::s as t;\nt::include!(\"../tests/hidden.rs\");",
        )],
    );
}

#[test]
fn h1_ordinary_rebinding_patterns_converge_cleanly() {
    check(
        "H1 decoys",
        &[
            // A module and a function sharing a name, re-exported and imported
            // elsewhere: substituting `sha256` inside its own binding must not
            // grow `sha256::sha256::sha256::..` until a cap trips.
            (
                "module and function share a name",
                "mod sha256 { pub fn sha256() {} }\npub use self::sha256::sha256;\nmod other { use crate::sha256; pub fn g() { sha256::sha256(); } }",
                &[],
            ),
            (
                "crate Result alias used everywhere",
                "pub type Result<T> = std::result::Result<T, Error>;\npub struct Error;\nmod a { use crate::Result; pub fn f() -> Result<()> { Ok(()) } }\nmod b { use super::Result; pub fn g() -> Result<u8> { Ok(1) } }",
                &[],
            ),
            (
                "error module re-exported under its own name",
                "pub mod error { pub struct Error; }\npub use crate::error::Error;\nmod x { use crate::error; use super::Error; fn f(_: error::Error, _: Error) {} }",
                &[],
            ),
            (
                "same name bound in many modules",
                "mod a { pub struct Node; }\nmod b { use crate::a::Node; }\nmod c { use super::a::Node; }\nmod d { use crate::c::Node; pub fn f(_: Node) {} }",
                &[],
            ),
        ],
    );
}

#[test]
fn h1_a_rename_chain_that_outgrows_the_path_cap_is_cap003() {
    // Each link renames `x` inside the previous one, so the fully expanded
    // target of `a70` is `std::fmt::x::x::..::x` (72 segments): past the
    // 64-segment cap, which is CAP003 rather than a silent truncation.  (Every
    // spelling is pure, so the CAP003 can only come from the cap.)
    let mut src = String::from("pub use std::fmt as a0;\n");
    for k in 1..=70 {
        src.push_str(&format!("pub use crate::a{}::x as a{k};\n", k - 1));
    }
    src.push_str("pub fn f() { a70::y(); }\n");
    let got = verdict(&src);
    assert!(got.contains(&"?".to_string()), "expected CAP003, got {got:?}");
}

// ---------------------------------------------------------------------------
// H2: `Self` names the impl's self type
// ---------------------------------------------------------------------------

#[test]
fn h2_self_resolves_to_the_impl_self_type() {
    check(
        "H2",
        &[
            (
                "self-impl-path",
                "pub trait Probe { fn probe(&self) -> bool; }\nimpl Probe for std::path::Path {\n    fn probe(&self) -> bool { Self::exists(self) }\n}",
                &["fs:list"],
            ),
            (
                "qualified-self-angle",
                "pub trait Probe { fn probe(&self) -> bool; }\nimpl Probe for std::path::Path { fn probe(&self) -> bool { <Self>::is_file(self) } }",
                &["fs:list"],
            ),
            (
                "self type through a rename",
                "use std::path::PathBuf as B;\npub trait Probe { fn probe(&self) -> bool; }\nimpl Probe for B { fn probe(&self) -> bool { Self::is_dir(self) } }",
                &["fs:list"],
            ),
            (
                "self type with generics and a where clause",
                "pub trait Probe<T> { fn probe(&self) -> bool; }\nimpl<T> Probe<T> for std::path::Path where T: Copy { fn probe(&self) -> bool { Self::metadata(self).is_ok() } }",
                &["fs:list"],
            ),
            (
                "parenthesised self type",
                "pub trait Probe { fn probe(&self) -> bool; }\nimpl Probe for (std::path::Path) { fn probe(&self) -> bool { Self::exists(self) } }",
                &["fs:list"],
            ),
            (
                "self type through a type alias",
                "type P = std::path::Path;\npub trait Probe { fn probe(&self) -> bool; }\nimpl Probe for P { fn probe(&self) -> bool { Self::try_exists(self).is_ok() } }",
                &["fs:list"],
            ),
            (
                "inner impl wins over the outer one",
                "struct S;\nimpl S { fn g() { pub trait Q { fn q(&self) -> bool; } impl Q for std::path::Path { fn q(&self) -> bool { Self::exists(self) } } } }",
                &["fs:list"],
            ),
        ],
    );
}

#[test]
fn h2_an_unknown_self_type_falls_back_to_the_method_rules() {
    check(
        "H2 unknown",
        &[
            (
                "Self inside a macro_rules! transcriber",
                "macro_rules! m { () => { Self::exists(self) } }\npub trait Probe { fn probe(&self) -> bool; }\nimpl Probe for std::path::Path { fn probe(&self) -> bool { m!() } }",
                &["fs:list"],
            ),
            (
                "impl over a metavariable",
                "pub trait Probe { fn probe(&self) -> bool; }\nmacro_rules! m { ($t:ty) => { impl Probe for $t { fn probe(&self) -> bool { Self::is_symlink(self) } } } }\nm!(std::path::Path);",
                &["fs:list"],
            ),
        ],
    );
}

#[test]
fn h2_ordinary_self_paths_stay_clean() {
    check(
        "H2 decoys",
        &[
            ("local constructor", "struct S;\nimpl S { fn new() -> Self { Self } fn g() -> Self { Self::new() } }", &[]),
            (
                "trait impl for a local type",
                "struct S;\nimpl Default for S { fn default() -> Self { Self::make() } }\nimpl S { fn make() -> Self { S } }",
                &[],
            ),
            ("trait default body", "trait T { fn new() -> Self; fn twice() -> (Self, Self) where Self: Sized { (Self::new(), Self::new()) } }", &[]),
            (
                "impl over a pure std type",
                "trait Half { fn half(self) -> Self; }\nimpl Half for std::time::Duration { fn half(self) -> Self { Self::from_nanos(self.as_nanos() as u64 / 2) } }",
                &[],
            ),
            ("macro impl with a local constructor", "macro_rules! m { ($t:ident) => { impl $t { fn z() -> Self { Self::default() } } } }", &[]),
            ("Self in a struct definition", "struct List { next: Option<Box<Self>> }", &[]),
        ],
    );
}

// ---------------------------------------------------------------------------
// H3: type aliases bind every path of the right-hand side, or fail closed
// ---------------------------------------------------------------------------

#[test]
fn h3_type_alias_shapes_are_bound_or_refused() {
    check(
        "H3",
        &[
            (
                "type-paren",
                "#[allow(unused_parens)]\ntype P = (std::path::Path);\npub fn f() -> bool { P::exists(P::new(\"x\")) }",
                &["fs:list"],
            ),
            (
                "type-qualified",
                "type P = <std::path::PathBuf as std::ops::Deref>::Target;\npub fn f() -> bool { P::exists(P::new(\"x\")) }",
                &["fs:list"],
            ),
            (
                "where clause before the =",
                "type X<T> where T: Copy = std::path::Path;\npub fn f() -> bool { X::is_dir(X::new(\"x\")) }",
                &["fs:list"],
            ),
            (
                "path inside generic arguments",
                "type P = Box<std::sync::mpsc::Receiver<u8>>;\npub fn f(r: &std::sync::mpsc::Receiver<u8>) { let _ = P::recv_timeout(r, std::time::Duration::ZERO); }",
                &["time:sleep"],
            ),
            (
                "right-hand side is a metavariable",
                "macro_rules! a { ($t:ty) => { type P = $t; }; }\na!(std::path::Path);\npub fn f() -> bool { P::exists(P::new(\"x\")) }",
                &["fs:list"],
            ),
            ("associated type without a value", "trait T { type Item; type Out: Clone; fn f(&self); }", &[]),
            ("associated type with a pure value", "struct S;\nimpl Iterator for S { type Item = u8; fn next(&mut self) -> Option<u8> { None } }", &[]),
            ("matcher pattern is not a type alias", "macro_rules! m { (type $n:ident = $t:ty) => { }; }", &[]),
            ("existing shape still binds", "type F<'a> = &'a std::fs::File;\nfn f() { F::create(\"x\"); }", &["fs:read", "fs:write"]),
        ],
    );
}

#[test]
fn h3_unparseable_type_aliases_are_cap003() {
    check_cap003(
        "H3 CAP003",
        &[
            (
                "type-ident-name",
                "macro_rules! alias {\n    ($n:ident) => {\n        type $n = std::path::Path;\n    };\n}\nalias!(P);\npub fn f() -> bool { P::exists(P::new(\"x\")) }",
            ),
            ("no terminating semicolon", "type P = std::path::Path"),
            ("closing delimiter before the semicolon", "mod m { type P = std::path::Path }"),
            ("unbalanced generics", "type P<T = std::path::Path;"),
        ],
    );
}

// ---------------------------------------------------------------------------
// H4: a metavariable that can be a keyword, in a keyword position
// ---------------------------------------------------------------------------

#[test]
fn h4_ident_metavariable_in_a_keyword_position_is_cap003() {
    check_cap003(
        "H4",
        &[
            (
                "ident-keyword-use",
                "macro_rules! kw { ($k:ident) => { $k std::io as out; }; }\nkw!(use);\npub fn f() -> bool { out::stdout().flush().is_ok() }",
            ),
            (
                "ident-keyword-extern-narrow",
                "macro_rules! kw { ($k:ident) => { $k crate std as s; }; }\nkw!(extern);\npub fn f() -> String { s::env::var(\"HOME\").unwrap_or_default() }",
            ),
            ("use group via ident", "macro_rules! u { ($k:ident) => { $k {std::env as out}; }; }\nu!(use);\npub fn f() { out::var(\"H\"); }"),
            ("use group via tt", "macro_rules! u { ($k:tt) => { $k {std::env as out}; }; }\nu!(use);\npub fn f() { out::var(\"H\"); }"),
            ("type alias via ident", "macro_rules! t { ($k:ident) => { $k P = std::path::Path; }; }\nt!(type);"),
            ("impl generics via ident", "macro_rules! i { ($k:ident) => { $k<T> Probe for T {} }; }\ni!(impl);"),
            ("keyword then metavariable", "macro_rules! k { ($k:ident, $p:path) => { $k $p as out; }; }\nk!(use, std::env);"),
            ("repetition of idents before a name", "macro_rules! k { ($($k:ident)*) => { $($k)* std::env as out; }; }\nk!(pub use);"),
            ("extern ABI string via ident", "macro_rules! k { ($k:ident) => { $k \"C\" fn f() {} }; }"),
            // `use` directly followed by a metavariable: the use tree is chosen
            // by the caller (`u!(std)` is `use std as out;`).
            ("use of an ident metavariable", "macro_rules! u { ($i:ident) => { use $i as out; }; }\nu!(std);\npub fn f() { out::env::var(\"H\"); }"),
            ("use of a path metavariable", "macro_rules! u { ($p:path) => { use $p as out; }; }\nu!(std::path::Path);\npub fn f() { out::exists(out::new(\"x\")); }"),
        ],
    );
}

#[test]
fn h4_the_review_controls_still_fail() {
    check(
        "H4 controls",
        &[
            (
                "ident-keyword-extern (pure manifest)",
                "macro_rules! kw { ($k:ident) => { $k crate std as s; }; }\nkw!(extern);\npub fn f() -> String { s::env::var(\"HOME\").unwrap_or_default() }",
                &["?", "ffi:call"],
            ),
            (
                "ident-keyword-unsafe",
                "macro_rules! kw { ($k:ident) => { $k { core::ptr::read(&1u8 as *const u8) } }; }\npub fn f() -> u8 { kw!(unsafe) }",
                &["!unsafe_code"],
            ),
        ],
    );
}

#[test]
fn h4_ordinary_ident_metavariables_stay_clean() {
    check(
        "H4 decoys",
        &[
            ("fn name", "macro_rules! m { ($name:ident, $t:ty) => { fn $name(x: $t) -> $t { x } } }", &[]),
            ("impl block", "macro_rules! m { ($i:ident) => { impl $i { fn new() -> Self { $i } } } }", &[]),
            ("struct literal", "macro_rules! m { ($s:ident, $f:ident) => { $s { $f: 1, other: 2 } } }", &[]),
            ("empty struct literal", "macro_rules! m { ($s:ident) => { $s {} } }", &[]),
            ("shorthand struct literal", "macro_rules! m { ($s:ident, $a:ident, $b:ident) => { $s { $a, $b } } }", &[]),
            ("for loop", "macro_rules! m { ($i:ident) => { for $i in 0..3 { let _ = $i; } } }", &[]),
            ("cast", "macro_rules! m { ($x:ident) => { $x as u8 } }", &[]),
            ("call", "macro_rules! m { ($f:ident) => { $f(1); } }", &[]),
            ("let", "macro_rules! m { ($x:ident) => { let $x = 1; let y = $x + 1; } }", &[]),
            ("field type", "macro_rules! m { ($n:ident, $t:ty) => { struct S { $n: $t } } }", &[]),
            ("comparison", "macro_rules! m { ($a:ident, $b:ident) => { let c = $a < $b; } }", &[]),
            ("generic type in a field", "macro_rules! m { ($n:ident) => { struct S { v: Vec<$n> } fn f() -> $n<u8> { todo!() } } }", &[]),
            ("use of $crate", "macro_rules! m { () => { use $crate::inner::Thing; } }", &[]),
            ("use pattern in a matcher", "macro_rules! m { (use $p:path) => { }; }", &[]),
            ("keyword argument to a known macro", "fn f() { let v = vec![if true { 1 } else { 2 }]; let _ = v; }", &[]),
        ],
    );
}

// ---------------------------------------------------------------------------
// H5: Cargo target paths and build scripts must be `.rs` files
// ---------------------------------------------------------------------------

#[test]
fn h5_cargo_targets_not_ending_in_rs_are_cap003() {
    use coding_adventures_ca_capability_analyzer::cargo::analyze_cargo_text;
    for (name, extra) in [
        ("cargo-lib-nonrs", "[lib]\npath = \"src/lib.txt\"\n"),
        ("cargo-bin-nonrs", "[[bin]]\nname = \"tool\"\npath = \"src/tool.inc\"\n"),
        ("cargo-build-nonrs", "build = \"build.txt\"\n"),
        ("build without extension", "build = \"src/build\"\n"),
        ("example", "[[example]]\nname = \"e\"\npath = \"src/e.txt\"\n"),
        ("test", "[[test]]\nname = \"t\"\npath = \"src/t\"\n"),
        ("bench (extension case differs)", "[[bench]]\nname = \"b\"\npath = \"src/b.RS\"\n"),
        // Top-level inline tables come before `[package]`.
        ("inline lib table", "lib = { path = \"src/lib\" }\n"),
        ("bin array of tables", "bin = [{ name = \"x\", path = \"src/x.rs.txt\" }]\n"),
    ] {
        // `build` is a `[package]` key; the target tables start their own section.
        let text = if extra.starts_with("lib =") || extra.starts_with("bin =") {
            format!("{extra}{CARGO_2021}")
        } else {
            format!("{CARGO_2021}{extra}")
        };
        let facts = analyze_cargo_text(&text);
        assert!(
            facts.findings.iter().any(|f| f.reason.contains(".rs")),
            "{name}: expected a non-.rs finding, got {:?}",
            facts.findings
        );
    }
    for (name, extra) in [
        ("explicit default lib path", "[lib]\npath = \"src/lib.rs\"\n"),
        ("bin path", "[[bin]]\nname = \"tool\"\npath = \"src/bin/tool.rs\"\n"),
        ("build script", "build = \"build.rs\"\n"),
        ("build disabled", "build = false\n"),
        ("metadata path is not a target", "[package.metadata.x]\npath = \"docs\"\n"),
    ] {
        let facts = analyze_cargo_text(&format!("{CARGO_2021}{extra}"));
        assert!(facts.findings.is_empty(), "{name}: unexpected {:?}", facts.findings);
    }
}

#[test]
fn h5_a_non_rs_library_root_fails_the_package() {
    let r = analyze_files(
        "h5lib",
        &[
            ("Cargo.toml", &format!("{CARGO_2021}[lib]\npath = \"src/lib.txt\"\n")),
            ("src/lib.txt", "pub fn f() { std::fs::remove_file(\"x\").unwrap(); }\n"),
            ("src/lib.rs", "pub fn decoy() {}\n"),
            ("required_capabilities.json", &pure_manifest("h5lib")),
        ],
    );
    assert_eq!(r.exit_code(), 1, "{}", r.format_text(true));
    assert!(cap003_reasons(&r).iter().any(|l| l.contains("Cargo.toml")), "{:?}", cap003_reasons(&r));
}

#[test]
fn h5_the_certificate_refuses_a_library_root_other_than_src_lib_rs() {
    use coding_adventures_ca_capability_analyzer::{generate_certificate, write_module_to_disk};
    let p = Pkg::with(
        "h5cert",
        &[
            ("Cargo.toml", &format!("{CARGO_2021}[lib]\npath = \"src/real.rs\"\n")),
            ("src/real.rs", "pub fn hidden_entry() {}\n"),
            ("src/lib.rs", "pub fn decoy() {}\n"),
            ("required_capabilities.json", &pure_manifest("h5cert")),
        ],
    );
    write_module_to_disk(p.dir()).unwrap();
    let err = generate_certificate(p.dir()).expect_err("certificate must be refused");
    assert!(err.contains("src/lib.rs"), "{err}");

    // The default layout (and an explicit `src/lib.rs`) still certifies.
    for cargo in [CARGO_2021.to_string(), format!("{CARGO_2021}[lib]\npath = \"./src/lib.rs\"\n")] {
        let q = Pkg::with(
            "h5cert_ok",
            &[
                ("Cargo.toml", &cargo),
                ("src/lib.rs", "pub fn entry() {}\n"),
                ("required_capabilities.json", &pure_manifest("h5cert_ok")),
            ],
        );
        write_module_to_disk(q.dir()).unwrap();
        let cert = generate_certificate(q.dir()).unwrap_or_else(|e| panic!("{cargo}: {e}"));
        assert_eq!(cert.entry_points.len(), 1);
    }
}

// ---------------------------------------------------------------------------
// H6: clock reads and timed waits reached through the pure list
// ---------------------------------------------------------------------------

#[test]
fn h6_time_reached_through_pure_modules_is_classified() {
    check(
        "H6",
        &[
            (
                "unix-epoch-elapsed",
                "pub fn f() -> u64 {\n    std::time::UNIX_EPOCH.elapsed().map(|d| d.as_secs()).unwrap_or(0)\n}",
                &["time:read"],
            ),
            ("UNIX_EPOCH imported", "use std::time::UNIX_EPOCH;\nfn f() { let _ = UNIX_EPOCH; }", &["time:read"]),
            ("elapsed on any receiver", "fn f(t: T) { let _ = t.elapsed(); }", &["time:read"]),
            (
                "sync-timed-wait",
                "use std::sync::{mpsc, Condvar, Mutex};\nuse std::time::Duration;\npub fn f() -> bool {\n    let (_tx, rx) = mpsc::channel::<()>();\n    let a = rx.recv_timeout(Duration::from_millis(1)).is_err();\n    let m = Mutex::new(());\n    let c = Condvar::new();\n    let b = c.wait_timeout(m.lock().unwrap(), Duration::from_millis(1)).is_ok();\n    a && b\n}",
                &["time:sleep"],
            ),
            ("wait_timeout_while", "fn f(c: &C, g: G) { let _ = c.wait_timeout_while(g, D, |_| true); }", &["time:sleep"]),
            ("wait_timeout_ms", "fn f(c: &C, g: G) { let _ = c.wait_timeout_ms(g, 1); }", &["time:sleep"]),
            ("Condvar UFCS", "fn f(c: &std::sync::Condvar, g: G) { let _ = std::sync::Condvar::wait_timeout(c, g, D); }", &["time:sleep"]),
            ("Receiver UFCS", "use std::sync::mpsc::Receiver;\nfn f(r: &Receiver<u8>) { let _ = Receiver::recv_timeout(r, D); }", &["time:sleep"]),
            ("decoy Duration", "fn f() -> std::time::Duration { std::time::Duration::from_millis(1) }", &[]),
            ("decoy untimed wait", "fn f(c: &std::sync::Condvar, g: G) { let _ = c.wait(g); let _ = rx.recv(); }", &[]),
        ],
    );
}

// ---------------------------------------------------------------------------
// H7: a method name spliced by a metavariable
// ---------------------------------------------------------------------------

#[test]
fn h7_method_name_splice_is_cap003() {
    check_cap003(
        "H7",
        &[
            (
                "ident-method-splice",
                "macro_rules! call {\n    ($e:expr, $m:ident) => {\n        $e.$m()\n    };\n}\npub fn f() -> bool {\n    call!(std::path::Path::new(\"Cargo.toml\"), exists)\n}",
            ),
            ("tt method", "macro_rules! call { ($e:expr, $m:tt) => { $e.$m(1) }; }"),
            ("method on a literal receiver", "macro_rules! call { ($m:ident) => { p.$m() }; }"),
        ],
    );
    check(
        "H7 decoys",
        &[
            ("field access", "macro_rules! get { ($e:expr, $f:ident) => { $e.$f }; }", &[]),
            ("tuple field", "macro_rules! get { ($e:expr) => { $e.0 }; }", &[]),
            ("fixed method", "macro_rules! call { ($e:expr) => { $e.len() }; }", &[]),
        ],
    );
}

// ---------------------------------------------------------------------------
// MEDIUM: metadata writes through an open handle
// ---------------------------------------------------------------------------

#[test]
fn medium_handle_metadata_writes_are_fs_write() {
    check(
        "MEDIUM",
        &[
            (
                "handle-method-escalation",
                "pub fn f() -> bool {\n    match std::fs::File::open(\"Cargo.toml\") {\n        Ok(file) => file.set_times(std::fs::FileTimes::new()).is_ok(),\n        Err(_) => false,\n    }\n}",
                &["fs:read", "fs:write"],
            ),
            ("set_permissions on a handle", "fn f(h: &H, p: P) { let _ = h.set_permissions(p); }", &["fs:write"]),
            ("set_modified on a handle", "fn f(h: &H, t: T) { let _ = h.set_modified(t); }", &["fs:write"]),
            ("File::set_times UFCS", "fn f(h: &std::fs::File) { let _ = std::fs::File::set_times(h, T); }", &["fs:read", "fs:write"]),
            ("File::set_permissions UFCS", "use std::fs::File;\nfn f(h: &File) { let _ = File::set_permissions(h, P); }", &["fs:read", "fs:write"]),
        ],
    );
}

// ---------------------------------------------------------------------------
// LOW: pass-through detection is linear in the matcher size
// ---------------------------------------------------------------------------

#[test]
fn low_many_tt_repetitions_against_a_long_matcher_finish_quickly() {
    // `perf-valid-40000` in miniature: a long literal matcher and many
    // transcriber repetitions.  The old code rescanned the matcher once per
    // repetition (quadratic); the per-arm set keeps it linear.
    let n = 40_000;
    let mut src = String::from("macro_rules! m { (");
    for _ in 0..n {
        src.push_str("a ");
    }
    src.push_str("$($t:tt)*) => { ");
    for _ in 0..n {
        src.push_str("$($t)* ; ");
    }
    src.push_str("}; }\n");
    let start = std::time::Instant::now();
    let got = verdict(&src);
    let took = start.elapsed();
    assert_eq!(got, Vec::<String>::new());
    assert!(took < std::time::Duration::from_secs(10), "took {took:?}");
}

// ---------------------------------------------------------------------------
// LOW: `--generate-module` replaces the file, never writes through a hard link
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn low_generate_module_does_not_write_through_a_hard_link() {
    use coding_adventures_ca_capability_analyzer::write_module_to_disk;
    let outside = Pkg::with("h_outside", &[("victim.txt", "precious\n")]);
    let p = Pkg::with(
        "h_link",
        &[("required_capabilities.json", &pure_manifest("h_link")), ("src/lib.rs", "pub fn f() {}\n")],
    );
    std::fs::hard_link(outside.path("victim.txt"), p.path("src/capabilities.rs")).unwrap();
    write_module_to_disk(p.dir()).unwrap();
    assert_eq!(outside.read("victim.txt"), "precious\n", "the hard-linked file outside the package was overwritten");
    assert!(p.read("src/capabilities.rs").contains("Generated from required_capabilities.json"));
    // No temporary file is left behind.
    let left: Vec<String> = std::fs::read_dir(p.path("src"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    let mut left = left;
    left.sort();
    assert_eq!(left, vec!["capabilities.rs".to_string(), "lib.rs".to_string()]);
}
