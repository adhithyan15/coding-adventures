//! The command line: exit codes 0, 1, 2 and the text of the report.

mod common;
use common::*;

use std::process::{Command, Output};

const SCAN: &str = "vos-fixture-scan";

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vos-link"))
        .args(args)
        .output()
        .unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn pkg(r: &Repo) -> String {
    r.pkg(SCAN).to_string_lossy().into_owned()
}

#[test]
fn gen_doc_then_check_doc_exit_zero_and_print_the_report() {
    let r = Repo::with("cli1", SCAN);
    let g = run(&["gen-doc", &pkg(&r)]);
    assert_eq!(g.status.code(), Some(0), "{}{}", out(&g), err(&g));
    let text = out(&g);
    assert!(text.contains("vos-link gen-doc: crate vos-fixture-scan, kind production, contract vos-sched-contract"), "{text}");
    assert!(text.contains("wrote code/specs/vos-fixture-scan.warrant"));
    assert!(text.contains("carries: canonical,safety"));
    assert!(text.contains("result: PASS, 0 findings"));
    let c = run(&["check-doc", &pkg(&r), "--root", &r.root.to_string_lossy()]);
    assert_eq!(c.status.code(), Some(0), "{}", out(&c));
    assert!(out(&c).contains("result: PASS"));
}

#[test]
fn findings_exit_one_and_print_each_code() {
    let r = Repo::with("cli2", SCAN);
    run(&["gen-doc", &pkg(&r)]);
    r.edit(
        "code/specs/vos-fixture-scan.warrant",
        "if t >= 8u8 then (OutOfRange, m)",
        "if t >= 9u8 then (OutOfRange, m)",
    );
    let c = run(&["check-doc", &pkg(&r)]);
    assert_eq!(c.status.code(), Some(1));
    let text = out(&c);
    for code in ["CD02 ", "CD03 ", "CD08 "] {
        assert!(text.contains(code), "{text}");
    }
    assert!(text.contains("result: FAIL, 3 findings"), "{text}");
}

#[test]
fn input_findings_exit_one_without_a_header() {
    let r = Repo::with("cli3", SCAN);
    r.write(
        "code/packages/rust/vos-fixture-scan/vos.toml",
        "kind = \"spine\"\n",
    );
    let g = run(&["gen-doc", &pkg(&r)]);
    assert_eq!(g.status.code(), Some(1));
    assert!(out(&g).contains("GD05 "));
}

#[test]
fn no_verdict_exits_two_on_stderr() {
    let r = Repo::with("cli4", SCAN);
    let o = run(&[
        "check-doc",
        &r.path("code/packages/rust/nothing").to_string_lossy(),
    ]);
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).starts_with("vos-link: no verdict: "), "{}", err(&o));
    assert!(out(&o).is_empty());
}

#[test]
fn usage_errors_exit_two() {
    for args in [
        vec![],
        vec!["frobnicate", "x"],
        vec!["gen-doc"],
        vec!["gen-doc", "a", "b"],
        vec!["gen-doc", "a", "--root"],
        vec!["gen-doc", "a", "--root", "x", "--root", "y"],
        vec!["gen-doc", "a", "--bogus"],
    ] {
        let o = run(&args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(err(&o).contains("usage: vos-link"), "{args:?}: {}", err(&o));
    }
}

#[test]
fn a_root_that_disagrees_with_the_crate_exits_two() {
    let r = Repo::with("cli5", SCAN);
    let other = Repo::new("cli6");
    for cmd in ["gen-doc", "check-doc"] {
        let o = run(&[cmd, &pkg(&r), "--root", &other.root.to_string_lossy()]);
        assert_eq!(o.status.code(), Some(2), "{cmd}");
        assert!(
            err(&o).contains("is not the repository root"),
            "{}",
            err(&o)
        );
    }
}

#[test]
fn a_pass_prints_the_lint_note_and_an_escaped_usage_error() {
    let r = Repo::with("cli7", SCAN);
    let g = run(&["gen-doc", &pkg(&r)]);
    assert!(
        out(&g).contains("note: R6 cone rule: a STRUCTURAL LINT"),
        "{}",
        out(&g)
    );
    let o = run(&["--bad\u{1b}[31m"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(!err(&o).contains('\u{1b}'), "{:?}", err(&o));
}

#[test]
fn the_tests_that_read_the_contract_package_have_a_cargo_edge_to_it() {
    // VOS-D058: see vos-manifest's twin of this test.
    let manifest = include_str!("../Cargo.toml");
    let dev = manifest
        .split("[dev-dependencies]")
        .nth(1)
        .expect("a dev-dependencies table");
    assert!(dev
        .contains("vos-sched-contract = { path = \"../../../packages/rust/vos-sched-contract\" }"));
}
