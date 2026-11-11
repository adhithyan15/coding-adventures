//! # Round-5 security review regressions (VER08a divergence D43 onwards)
//!
//! A third review of the round-3 and round-4 fixes built more small crates
//! that the analyzer passed (exit 0) while `cargo check` compiled a
//! capability call into them.  The cheap ones are fixed and pinned here; the
//! rest are documented as known open gaps in VER08a §8.1.  Each test names
//! the review's case directory, and every group carries decoys that must stay
//! clean so a fix does not flood real packages with false positives.
//!
//! Verdict notation is the one used by `test_evasion_corpus.rs`:
//!
//! * `cat:action` - a capability detection (CAP001 candidate)
//! * `!construct` - a banned construct (CAP002 candidate)
//! * `?`          - an unanalysable construct (CAP003, never waivable)

use coding_adventures_ca_capability_analyzer::{scan_source, ScanResult};

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

// ---------------------------------------------------------------------------
// R5-1: an empty (or any) turbofish between the method name and `(`
// ---------------------------------------------------------------------------
//
// The method heuristic looked only at the token right after the name, so
// `p.exists::<>()` - legal Rust for a method with no generic parameters -
// slipped past it.

#[test]
fn r5_1_turbofish_on_the_method_name_keeps_the_method_heuristic() {
    check(
        "R5-1",
        &[
            (
                "turbofish-exists",
                "pub fn probe() -> bool { std::path::Path::new(\"Cargo.toml\").exists::<>() }",
                &["fs:list"],
            ),
            (
                "turbofish-wait-timeout",
                "use std::sync::{Condvar, Mutex};\nuse std::time::Duration;\npub fn nap() {\n    let m = Mutex::new(());\n    let c = Condvar::new();\n    let g = m.lock().unwrap();\n    let _ = c.wait_timeout::<>(g, Duration::from_millis(10));\n}",
                &["time:sleep"],
            ),
            ("non-empty turbofish", "fn f(p: &P) -> bool { p.is_file::<u8>() }", &["fs:list"]),
            ("nested turbofish", "fn f(t: &T) { t.elapsed::<Vec<Box<u8>>>(); }", &["time:read"]),
            ("turbofish on a set_times call", "fn f(h: &H) { h.set_times::<>(t); }", &["fs:write"]),
        ],
    );
}

#[test]
fn r5_1_turbofish_decoys_stay_clean() {
    check(
        "R5-1 decoys",
        &[
            ("turbofish on a pure method", "fn f(v: Vec<u8>) -> Vec<u16> { v.into_iter().map(u16::from).collect::<Vec<_>>() }", &[]),
            ("field named exists", "fn f(s: &S) -> bool { s.exists }", &[]),
            ("field compared through a turbofish-looking shape", "fn f(s: &S) -> bool { s.exists < 3 }", &[]),
        ],
    );
}

// ---------------------------------------------------------------------------
// R5-2: a macro invocation in type position is an unknown type
// ---------------------------------------------------------------------------
//
// `ty!()` expands to a type the analyzer never sees.  The readers of a
// qualified self type (`<ty!()>::exists`), an impl self type
// (`impl P for ty!()`) and a type alias right-hand side (`type F = ty!();`)
// stopped at the `!` and took the macro's *name* as the type: `ty::exists`,
// a local, pure path.  Each now yields an unknown type, which gets the
// verdict of a method call `.name(` on an unknown receiver, exactly as a
// metavariable does.

#[test]
fn r5_2_a_type_macro_is_an_unknown_type() {
    check(
        "R5-2",
        &[
            (
                "type-macro-qualified",
                "macro_rules! ty { () => { std::path::Path }; }\npub fn probe(p: &std::path::Path) -> bool { <ty!()>::exists(p) }",
                &["fs:list"],
            ),
            (
                "type-macro-alias",
                "macro_rules! ty { () => { std::path::Path }; }\ntype F = ty!();\npub fn probe(p: &std::path::Path) -> bool { F::exists(p) }",
                &["fs:list"],
            ),
            (
                "type-macro-impl-self",
                "macro_rules! ty { () => { std::path::Path }; }\npub trait P { fn go(&self) -> bool; }\nimpl P for ty!() { fn go(&self) -> bool { Self::exists(self) } }\npub fn run() -> bool { std::path::Path::new(\"Cargo.toml\").go() }",
                &["fs:list"],
            ),
            (
                "type-macro-condvar",
                "macro_rules! cv { () => { std::sync::Condvar }; }\npub fn nap() {\n    let m = std::sync::Mutex::new(());\n    let c = <cv!()>::new();\n    let g = m.lock().unwrap();\n    let _ = <cv!()>::wait_timeout(&c, g, std::time::Duration::from_millis(10));\n}",
                &["time:sleep"],
            ),
            (
                "qualified with a trait",
                "macro_rules! ty { () => { std::path::Path }; }\npub trait Probe { fn exists(&self) -> bool; }\nfn f(p: &std::path::Path) -> bool { <ty!() as Probe>::exists(p) }",
                &["fs:list"],
            ),
            (
                "qualified, macro behind a reference",
                "macro_rules! ty { () => { std::path::Path }; }\nfn f(p: &std::path::Path) -> bool { <&ty!()>::is_file(&p) }",
                &["fs:list"],
            ),
            (
                "qualified, path-qualified macro",
                "fn f(p: &std::path::Path) -> bool { <crate::ty!()>::read_link(p).is_ok() }",
                &["fs:list"],
            ),
            (
                "inherent impl over a macro",
                "macro_rules! ty { () => { std::path::Path }; }\nimpl ty!() { fn go(&self) -> bool { Self::is_symlink(self) } }",
                &["fs:list"],
            ),
            (
                "alias to a macro with brackets",
                "type F = ty![];\nfn f(p: &std::path::Path) -> bool { F::try_exists(p).is_ok() }",
                &["fs:list"],
            ),
            (
                "alias to a path-qualified macro",
                "type F = crate::ty!{};\nfn f(p: &std::path::Path) -> bool { F::canonicalize(p).is_ok() }",
                &["fs:list"],
            ),
            // Found while fixing R5-2: a parenthesised qualified self type.
            (
                "parenthesised qualified self type",
                "#![allow(unused_parens)]\npub fn a(p: &std::path::Path) -> bool { <(std::path::Path)>::exists(p) }",
                &["fs:list"],
            ),
        ],
    );
}

#[test]
fn r5_2_type_macro_decoys_stay_clean() {
    check(
        "R5-2 decoys",
        &[
            ("qualified pure item", "macro_rules! ty { () => { Vec<u8> }; }\nfn f() -> Vec<u8> { <ty!()>::new() }", &[]),
            ("qualified trait item", "fn f() -> u8 { <ty!() as Default>::default() }", &[]),
            ("impl self with a local ctor", "impl Default for ty!() { fn default() -> Self { Self::new() } }", &[]),
            ("alias with a pure item", "type V = ty!();\nfn f() -> V { V::new() }", &[]),
            ("never type alias", "type Never = !;", &[]),
            ("macro in a value position", "fn f() -> usize { vec![1u8].len() }", &[]),
            ("parenthesised pure self type", "#![allow(unused_parens)]\nfn f() -> Vec<u8> { <(Vec<u8>)>::new() }", &[]),
        ],
    );
}
