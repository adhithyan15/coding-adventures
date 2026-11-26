//! The command line of VOS31 section 9: `gen`, `check`, exit codes, `--emit-drat`, and the
//! refusals.  The solving is shared with `tests/lane.rs` through the cached evidence.

mod common;

use std::path::Path;
use std::process::Command as Proc;

use common::{evidence, temp_contract};
use vos_model_sat::cli::{parse, run_with, Command, Verb};

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

fn cmd(verb: Verb, dir: &Path, root: &Path, flag: bool) -> Command {
    Command {
        verb,
        dir: dir.to_path_buf(),
        root: Some(root.to_path_buf()),
        emit_drat: None,
        allow_missing_sat_key: flag,
    }
}

#[test]
fn parse_accepts_the_three_verbs_and_refuses_the_rest() {
    let c = parse(&args(&[
        "gen",
        "d",
        "--root",
        "r",
        "--allow-missing-sat-key",
    ]))
    .unwrap();
    assert_eq!(c.verb, Verb::Gen);
    assert!(c.allow_missing_sat_key);
    assert_eq!(c.root.as_deref(), Some(Path::new("r")));
    let c = parse(&args(&["check", "d", "--emit-drat", "out"])).unwrap();
    assert_eq!(c.emit_drat.as_deref(), Some(Path::new("out")));
    assert_eq!(
        parse(&args(&["cross-check", "d"])).unwrap().verb,
        Verb::CrossCheck
    );
    for bad in [
        &[][..],
        &["solve", "d"],
        &["gen"],
        &["gen", "a", "b"],
        &["gen", "d", "--root"],
        &["gen", "d", "--emit-drat", "x"],
        &["cross-check", "d", "--allow-missing-sat-key"],
        &["check", "d", "--emit-drat"],
        &["check", "d", "--nope"],
    ] {
        assert!(parse(&args(bad)).is_none(), "{bad:?}");
    }
}

#[test]
fn gen_then_check_then_tamper_in_process() {
    let (root, dir) = temp_contract("cli");
    let ev = Some(evidence());
    // No `sat` key: MS01, exit 1, nothing written.
    let (code, _, err) = run_with(&cmd(Verb::Gen, &dir, &root, false), ev);
    assert_eq!(code, 1);
    assert!(err[0].starts_with("MS01"), "{err:?}");
    let table = dir.join("warrant").join("lemmas.sat");
    assert!(!table.exists());
    // `check` before `gen`: the table is absent (MS09).
    let (code, _, err) = run_with(&cmd(Verb::Check, &dir, &root, true), ev);
    assert_eq!(code, 1);
    assert!(err[0].starts_with("MS09"), "{err:?}");
    // gen writes it.
    let (code, out, err) = run_with(&cmd(Verb::Gen, &dir, &root, true), ev);
    assert_eq!(code, 0, "{err:?}");
    assert!(out.last().unwrap().starts_with("wrote "));
    let text = std::fs::read_to_string(&table).unwrap();
    assert!(text.starts_with("vos-sat 1\n") && text.ends_with("end\n"));
    // check is current, and can emit the proofs on request.
    let mut c = cmd(Verb::Check, &dir, &root, true);
    c.emit_drat = Some(root.join("proofs"));
    let (code, out, err) = run_with(&c, ev);
    assert_eq!(code, 0, "{err:?}");
    assert!(out.iter().any(|l| l.contains("DRAT files written")));
    assert!(out.last().unwrap().contains("current"));
    assert_eq!(std::fs::read_dir(root.join("proofs")).unwrap().count(), 80);
    // `..` in the emit directory is refused: no verdict.
    c.emit_drat = Some(root.join("a").join("..").join("b"));
    let (code, _, err) = run_with(&c, ev);
    assert_eq!(code, 2);
    assert!(err[0].contains("no verdict"));
    // A tampered table is MS09, exit 1.
    std::fs::write(&table, text.replace("vos-sat 1", "vos-sat 2")).unwrap();
    let (code, _, err) = run_with(&cmd(Verb::Check, &dir, &root, true), ev);
    assert_eq!(code, 1);
    assert!(
        err[0].starts_with("MS09") && err[0].contains("line 1"),
        "{err:?}"
    );
    // The table path is a symlink-free regular file; a directory in its place is refused.
    std::fs::remove_file(&table).unwrap();
    std::fs::create_dir(&table).unwrap();
    let (code, _, err) = run_with(&cmd(Verb::Check, &dir, &root, true), ev);
    assert_eq!(code, 1, "{err:?}");
    assert!(err[0].starts_with("MS01"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn cross_check_verb_and_unloadable_crates() {
    let (root, dir) = temp_contract("cli2");
    let (code, out, _) = run_with(&cmd(Verb::CrossCheck, &dir, &root, false), Some(evidence()));
    assert_eq!(code, 0);
    assert!(out.iter().any(|l| l.contains("0 disagreements")));
    // A crate that is not there: a finding for gen and check, no verdict for cross-check.
    let missing = dir.with_file_name("no-such-crate");
    let (code, _, err) = run_with(&cmd(Verb::Gen, &missing, &root, true), None);
    assert_eq!(code, 1, "{err:?}");
    assert!(err[0].starts_with("MS01"));
    let (code, _, err) = run_with(&cmd(Verb::CrossCheck, &missing, &root, false), None);
    assert_eq!(code, 2);
    assert!(err[0].contains("no verdict"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_binary_gens_and_checks_a_contract() {
    let (root, dir) = temp_contract("bin");
    let run = |verb: &str| {
        let out = Proc::new(env!("CARGO_BIN_EXE_vos-model-sat"))
            .args([
                verb,
                dir.to_str().unwrap(),
                "--root",
                root.to_str().unwrap(),
                "--allow-missing-sat-key",
            ])
            .output()
            .unwrap();
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).to_string(),
        )
    };
    let (code, out) = run("gen");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("36 obligations certified and counted"));
    let (code, out) = run("check");
    assert_eq!(code, 0, "{out}");
    let (code, _) = run("check-nothing");
    assert_eq!(code, 2);
    let _ = std::fs::remove_dir_all(&root);
}
