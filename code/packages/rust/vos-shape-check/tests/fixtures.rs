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
    let cases: [(&str, &[&str]); 5] = [
        ("fail-contract-poke", &["SH19"]),
        ("fail-contract-addr", &["SH19"]),
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

#[test]
fn the_command_type_fixtures_fire_sh19_and_nothing_else_for_a_wide_payload() {
    // `Write { addr: u32 }` is fixed-width, so only the command-type rule objects.
    let r = run(&load("fail-contract-addr"));
    assert_eq!(rules(&r), ["SH19"], "{}", r.render());
    assert!(
        r.render()
            .contains("enum `Pick` variant `Write` field `addr`: type `u32`"),
        "{}",
        r.render()
    );
    // `Poke(usize)` is also an SH10 matter (`usize` is not fixed-width); both fire.
    let r = run(&load("fail-contract-poke"));
    assert!(rules(&r).contains(&"SH19"), "{}", r.render());
    assert!(
        r.render()
            .contains("variant `Poke` field `0`: type `usize`"),
        "{}",
        r.render()
    );
    // The passing contract is the same crate with `Member(u8)`.
    assert!(run(&load("pass-ready-contract")).passed());
}

/// The real scheduler contract is judged by the real rule (its own BUILD runs the
/// binary too; this puts the same crate in the library's test).
#[test]
fn the_real_scheduler_contract_passes_sh19() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../vos-sched-contract");
    let input = vos_shape_check::loader::load_crate(&dir).expect("the contract crate loads");
    let r = run(&input);
    assert!(r.passed(), "{}", r.render());
    assert_eq!(r.kind, Some(vos_shape_check::manifest::Kind::Contract));
}

#[test]
fn load_sources_reads_src_only_and_refuses_what_load_crate_refuses() {
    use vos_shape_check::loader::load_sources;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../vos-sched-contract");
    let files = load_sources(&dir).expect("sources");
    assert!(files.iter().any(|f| f.path == "src/lib.rs"), "{files:?}");
    // A directory with no `src` has no sources (and no Cargo.toml is needed).
    let bare = fixtures_dir().join("pass-ready-contract").join("src");
    assert!(load_sources(&bare).expect("no src below it").is_empty());
    // A file is not a crate directory.
    let file = fixtures_dir().join("pass-ready-contract").join("vos.toml");
    assert!(load_sources(&file).is_err());
    assert!(load_sources(&fixtures_dir().join("no-such-crate")).is_err());
    // A symlinked crate directory is refused (Unix: creating one needs no privilege).
    #[cfg(unix)]
    {
        let t = materialize("pass-ready-contract", "ls-symlink");
        let link = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("vos-shape-check-link-{}", std::process::id()));
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&*t, &link).unwrap();
        let e = load_sources(&link).unwrap_err();
        let _ = std::fs::remove_file(&link);
        assert!(e.contains("not a plain directory"), "{e}");
    }
}
