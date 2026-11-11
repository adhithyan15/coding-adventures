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
