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
    assert!(err[1].starts_with("MS09"), "{err:?}");
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
    assert!(err[1].contains("no verdict"));
    // A tampered table is MS09, exit 1.
    std::fs::write(&table, text.replace("vos-sat 1", "vos-sat 2")).unwrap();
    let (code, _, err) = run_with(&cmd(Verb::Check, &dir, &root, true), ev);
    assert_eq!(code, 1);
    assert!(
        err[1].starts_with("MS09") && err[1].contains("line 1"),
        "{err:?}"
    );
    // The table path is a symlink-free regular file; a directory in its place is refused.
    std::fs::remove_file(&table).unwrap();
    std::fs::create_dir(&table).unwrap();
    let (code, _, err) = run_with(&cmd(Verb::Check, &dir, &root, true), ev);
    assert_eq!(code, 1, "{err:?}");
    assert!(err[1].starts_with("MS01"));
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
    assert!(err[1].starts_with("MS01"));
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

/// A directory symlink `link -> target`, or `false` where the platform or the account cannot
/// make one (Windows without the privilege): the symlink tests then skip.
fn make_dir_symlink(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        false
    }
}

#[test]
fn the_bridge_is_loud_on_standard_error_and_in_the_report() {
    let (root, dir) = temp_contract("bridge");
    let ev = Some(evidence());
    let (code, out, err) = run_with(&cmd(Verb::Gen, &dir, &root, true), ev);
    assert_eq!(code, 0, "{err:?}");
    assert_eq!(err[0], vos_model_sat::cli::BRIDGE_WARNING);
    assert!(
        err[0].starts_with("WARNING: MS01 opt-in check skipped (--allow-missing-sat-key bridge")
    );
    assert_eq!(out[0], vos_model_sat::lane::BRIDGED_REPORT_LINE);
    // An unbridged run never prints the warning (it fails MS01 instead).
    let (code, _, err) = run_with(&cmd(Verb::Gen, &dir, &root, false), ev);
    assert_eq!(code, 1);
    assert!(err.iter().all(|l| !l.contains("WARNING")), "{err:?}");
    // The warning also leads a bridged run that fails.
    let missing = dir.with_file_name("nope");
    let (_, _, err) = run_with(&cmd(Verb::Check, &missing, &root, true), None);
    assert_eq!(err[0], vos_model_sat::cli::BRIDGE_WARNING);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn emit_drat_refuses_to_overwrite_and_never_writes_through_a_symlink() {
    let (root, dir) = temp_contract("emit");
    let ev = Some(evidence());
    assert_eq!(run_with(&cmd(Verb::Gen, &dir, &root, true), ev).0, 0);
    let proofs = root.join("proofs");
    let mut c = cmd(Verb::Check, &dir, &root, true);
    c.emit_drat = Some(proofs.clone());
    assert_eq!(run_with(&c, ev).0, 0);
    // A second run into the same directory must not overwrite what is there.
    let name = proofs.join("top_is_the_maximum-n8.drat");
    let first = std::fs::read(&name).unwrap();
    let (code, _, err) = run_with(&c, ev);
    assert_eq!(code, 2, "{err:?}");
    assert!(err.iter().any(|l| l.contains("no verdict")));
    assert_eq!(std::fs::read(&name).unwrap(), first);
    // A pre-existing file at one name is refused, and is left as it was.
    let planted = root.join("planted");
    std::fs::create_dir(&planted).unwrap();
    std::fs::write(planted.join("closure_at_m-n8.cnf"), "keep me").unwrap();
    c.emit_drat = Some(planted.clone());
    assert_eq!(run_with(&c, ev).0, 2);
    assert_eq!(
        std::fs::read_to_string(planted.join("closure_at_m-n8.cnf")).unwrap(),
        "keep me"
    );
    // A symlinked output directory, and a symlinked ancestor of a directory to create.
    let target = root.join("elsewhere");
    std::fs::create_dir(&target).unwrap();
    let link = root.join("link");
    if make_dir_symlink(&target, &link) {
        c.emit_drat = Some(link.clone());
        let (code, _, err) = run_with(&c, ev);
        assert_eq!(code, 2, "{err:?}");
        assert!(err.iter().any(|l| l.contains("symlink")), "{err:?}");
        c.emit_drat = Some(link.join("below"));
        assert_eq!(run_with(&c, ev).0, 2);
        assert_eq!(
            std::fs::read_dir(&target).unwrap().count(),
            0,
            "nothing was written through the link"
        );
    } else {
        eprintln!("cannot create a symlink here: the symlink half is skipped");
    }
    // A path whose existing part is a regular file.
    c.emit_drat = Some(planted.join("closure_at_m-n8.cnf").join("x"));
    assert_eq!(run_with(&c, ev).0, 2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn gen_writes_through_a_temporary_file_and_refuses_a_symlinked_warrant_dir() {
    let (root, dir) = temp_contract("genw");
    let ev = Some(evidence());
    assert_eq!(run_with(&cmd(Verb::Gen, &dir, &root, true), ev).0, 0);
    let warrant = dir.join("warrant");
    let leftovers: Vec<String> = std::fs::read_dir(&warrant)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
    // Running gen again replaces the file (a rename over the target) with the same bytes.
    let before = std::fs::read(warrant.join("lemmas.sat")).unwrap();
    assert_eq!(run_with(&cmd(Verb::Gen, &dir, &root, true), ev).0, 0);
    assert_eq!(std::fs::read(warrant.join("lemmas.sat")).unwrap(), before);
    // A symlinked warrant/ is refused as MS01 before anything is written.
    let moved = dir.join("warrant-real");
    std::fs::rename(&warrant, &moved).unwrap();
    if make_dir_symlink(&moved, &warrant) {
        let (code, _, err) = run_with(&cmd(Verb::Gen, &dir, &root, true), ev);
        assert_eq!(code, 1, "{err:?}");
        assert!(err.iter().any(|l| l.contains("MS01")), "{err:?}");
    } else {
        eprintln!("cannot create a symlink here: the symlinked-warrant half is skipped");
    }
    let _ = std::fs::remove_dir_all(&root);
}
