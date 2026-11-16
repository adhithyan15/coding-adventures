//! The VOS01 section 5 toy crates, as written in VOS01 and VOS02, must PASS;
//! deliberately broken variants must FAIL with the expected rule ids.
//!
//! `pass-ready-contract` is VOS01 section 1.2, `pass-ready-bitmap` is section
//! 5.3 with the ghost face of VOS02 section 4.2, `pass-ready-linear` is section
//! 5.4.  Each `fail-*` directory is a copy of the bitmap with one defect.

mod common;
use common::*;

const PASSING: [&str; 3] = [
    "pass-ready-contract",
    "pass-ready-bitmap",
    "pass-ready-linear",
];

#[test]
fn the_toy_crates_pass_in_process() {
    for n in PASSING {
        let r = run(&load(n));
        assert!(r.passed(), "{n}:\n{}", r.render());
        assert_eq!(r.exit_code(), 0);
    }
}

#[test]
fn the_toy_crates_report_their_class_and_kind() {
    let r = run(&load("pass-ready-bitmap"));
    assert_eq!(r.package.as_deref(), Some("vos-ready-bitmap"));
    assert!(r.render().starts_with(
        "vos-shape-check: crate vos-ready-bitmap, class K (vos.toml), kind production\n"
    ));
    let r = run(&load("pass-ready-contract"));
    assert!(r.render().contains("kind contract"));
}

#[test]
fn the_broken_fixture_crates_fail_with_the_expected_rules() {
    let cases: [(&str, &[&str]); 3] = [
        ("fail-bitmap-ghost-leak", &["SH13", "SH18"]),
        ("fail-bitmap-build-rs", &["SH15"]),
        ("fail-bitmap-image-drift", &["SH17"]),
    ];
    for (name, want) in cases {
        let r = run(&load(name));
        assert!(!r.passed(), "{name} must fail");
        for id in want {
            assert!(
                rules(&r).contains(id),
                "{name} should fire {id}:\n{}",
                r.render()
            );
        }
    }
    // the build.rs fixture fires SH15 twice: the file and the key
    let r = run(&load("fail-bitmap-build-rs"));
    assert_eq!(
        rules(&r).iter().filter(|x| **x == "SH15").count(),
        2,
        "{}",
        r.render()
    );
}

/// (fixture, pattern, replacement, rule that must now fire).  Each mutation
/// is applied to one place in the toy's `src/lib.rs`.
#[test]
fn single_mutations_of_the_bitmap_each_break_their_rule() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "pub struct BitmapReady;",
            "pub struct BitmapReady;\nstatic HITS: u8 = 0;",
            "SH3",
        ),
        (
            "fn init() -> Bitmap { Bitmap { mask: 0 } }",
            "fn init() -> Bitmap { Bitmap { mask: 0 / 1 } }",
            "SH7",
        ),
        (
            "let bit = 1u8.wrapping_shl(p as u32);\n        if s.mask & bit != 0",
            "let bit = 1u8 << p;\n        if s.mask & bit != 0",
            "SH9",
        ),
        (
            "fn pick(s: Bitmap) -> (Pick, Bitmap) {",
            "fn pick(s: Bitmap) -> (Pick, Bitmap) {\n        while false { }",
            "SH6",
        ),
        (
            "const fn clz8(x: u8) -> u8 {",
            "const fn clz8(x: u8) -> u8 {\n    if x == 255 { return clz8(0); }",
            "SH5",
        ),
        (
            "(Pick::Member(7u8.wrapping_sub(clz8(s.mask))), s)",
            "(Pick::Member(Some(7u8).unwrap()), s)",
            "SH8",
        ),
        (
            "#![forbid(unsafe_code)]",
            "#![forbid(unsafe_code)]\nfn bad() { unsafe { } }",
            "SH2",
        ),
        (
            "pub struct Bitmap { mask: u8 }",
            "pub struct Bitmap { mask: f32 }",
            "SH11",
        ),
        (
            "pub struct Bitmap { mask: u8 }",
            "pub struct Bitmap { pub mask: u8 }",
            "SH14",
        ),
        (
            "pub struct BitmapReady;",
            "pub struct BitmapReady;\nfn boxed(_b: Box<u8>) {}",
            "SH10",
        ),
        (
            "pub struct BitmapReady;",
            "pub struct BitmapReady;\nfn dynamic(_d: &dyn Fn()) {}",
            "SH4",
        ),
        (
            "pub struct BitmapReady;",
            "pub struct BitmapReady;\nmacro_rules! m { () => {} }",
            "SH15",
        ),
        ("#![no_std]", "extern crate std;", "SH1"),
    ];
    for (from, to, id) in cases {
        let r = run(&mutate(load("pass-ready-bitmap"), from, to));
        assert!(
            rules(&r).contains(id),
            "{to}\nshould fire {id}:\n{}",
            r.render()
        );
    }
}

#[test]
fn single_mutations_of_the_linear_scan_each_break_their_rule() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "for i in 0..LEN {                            // SH6",
            "let n = 8u8; for i in 0..n {                            // SH6",
            "SH6",
        ),
        (
            "if !found && s.cells[i] != 0 {",
            "if !found && s.cells[0] != 0 {",
            "SH8",
        ),
        (
            "for i in 0..LEN {                            // SH6",
            "for i in s.cells.iter() {                            // SH6",
            "SH6",
        ),
        (
            "const LEN: usize = 8; ",
            "const LEN: usize = 8; static T: u8 = 0; ",
            "SH3",
        ),
        (
            "pub struct LinearReady;",
            "pub struct LinearReady;\nfn half(x: u8) -> u8 { x / 2 }",
            "SH7",
        ),
    ];
    for (from, to, id) in cases {
        let r = run(&mutate(load("pass-ready-linear"), from, to));
        assert!(
            rules(&r).contains(id),
            "{to}\nshould fire {id}:\n{}",
            r.render()
        );
    }
}

#[test]
fn the_contract_crate_mutations_break_their_rules() {
    check(
        mutate(load("pass-ready-contract"), "const N: u8;", "const N: f64;"),
        Expect::Fires("SH11".into()),
        "",
    );
    check(
        mutate(
            load("pass-ready-contract"),
            "fn init() -> Self::State;",
            "fn init(&self) -> Self::State;",
        ),
        Expect::Fires("SH10".into()),
        "",
    );
}

#[test]
fn a_third_party_dependency_or_proc_macro_fails_the_toy() {
    let mut i = load("pass-ready-bitmap");
    i.cargo_toml = Some(
        i.cargo_toml
            .unwrap()
            .replace("[dependencies]\n", "[dependencies]\nserde = \"1\"\n"),
    );
    check(i.clone(), Expect::Fires("SH12".into()), "");
    let mut i = load("pass-ready-bitmap");
    i.cargo_toml = Some(
        i.cargo_toml
            .unwrap()
            .replace("[features]", "[lib]\nproc-macro = true\n\n[features]"),
    );
    check(i.clone(), Expect::Fires("SH15".into()), "");
}
