//! `check-publish`, the VOS-D104 sweep (VOS-039b-2b): codes PB01 and PB02, the scope rule and the
//! fail-closed paths.  Every control builds a temporary repository and asserts the finding's code
//! AND text AND that nothing else fired.

mod common;
use common::*;

use std::path::Path;
use std::process::Command;

use vos_link::publish::{prefixed_children, root_above, says_publish_false, swept_name};
use vos_link::{check_publish, NoVerdict, Outcome};

const PKG: &str = "code/packages/rust";
const PROG: &str = "code/programs/rust";

/// A repository with both scan roots and no crate in them.
fn tree(label: &str) -> Repo {
    let r = Repo::new(label);
    std::fs::create_dir_all(r.path(PROG)).unwrap();
    // (`Repo::new` copies the real contract package in, but without a Cargo.toml: not swept.)
    r
}

/// Write `<dir>/<name>/Cargo.toml` (and `vos.toml` when given).
fn krate(r: &Repo, dir: &str, name: &str, cargo_extra: &str, vos: Option<&str>) {
    r.write(
        &format!("{dir}/{name}/Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n{cargo_extra}\n"),
    );
    if let Some(v) = vos {
        r.write(&format!("{dir}/{name}/vos.toml"), v);
    }
}

const ROWS: &str = "code/programs/go/build-tool/repository_inputs.go";

/// Write the build tool's row file with the given `(function, crate)` rows in the consumers slice.
fn register(r: &Repo, rows: &[(&str, &str)]) {
    let mut src = String::from(
        "package main\n\nvar (\n\trepositoryInputConsumers = []repositoryInputConsumer{\n",
    );
    for (f, c) in rows {
        src.push_str(&format!("\t\t{f}(\"{c}\"),\n"));
    }
    src.push_str("\t}\n)\n");
    // The slice ends at a line that is exactly `}`.
    let src = src.replace("\t}\n)\n", "}\n)\n");
    r.write(ROWS, src);
}

fn sweep(r: &Repo) -> Outcome {
    check_publish(Some(&r.root)).expect("a verdict")
}

fn no_verdict(r: &Repo) -> NoVerdict {
    check_publish(Some(&r.root)).expect_err("no verdict")
}

fn only(o: &Outcome, code: &str, text: &str) {
    assert_eq!(o.findings.len(), 1, "{}", show(o));
    assert_eq!(o.findings[0].code, code, "{}", show(o));
    assert!(
        o.findings[0].message.contains(text),
        "{text:?} in {}",
        show(o)
    );
}

#[test]
fn an_empty_tree_and_the_exempt_kinds_pass_with_the_policy_note() {
    let r = tree("pub-pass");
    let o = sweep(&r);
    assert!(o.findings.is_empty(), "{}", show(&o));
    assert!(o.header.contains("0 crates"), "{}", o.header);
    register(
        &r,
        &[
            ("vosImplementationRow", "vos-a"),
            ("vosImplementationRow", "vos-b"),
            ("vosContractRow", "vos-c"),
        ],
    );
    for (kind, name) in [
        ("production", "vos-a"),
        ("reference", "vos-b"),
        ("contract", "vos-c"),
    ] {
        krate(
            &r,
            PKG,
            name,
            "",
            Some(&format!("class = \"K\"\nkind = \"{kind}\"\n")),
        );
    }
    let o = sweep(&r);
    assert!(o.findings.is_empty(), "{}", show(&o));
    let note = o
        .notes
        .iter()
        .find(|n| n.contains("publish policy undecided, release gate"))
        .unwrap();
    for n in ["\"vos-a\"", "\"vos-b\"", "\"vos-c\""] {
        assert!(note.contains(n), "{n} in {note}");
    }
}

#[test]
fn control_an_evidence_only_crate_without_the_flag_is_pb01() {
    for (label, vos) in [
        ("any", Some("class = \"K\"\nkind = \"any\"\n")),
        ("nokind", Some("class = \"H\"\n")),
        ("novos", None),
        ("spine", Some("class = \"K\"\nkind = \"spine\"\n")),
    ] {
        let r = tree(&format!("pub-pb01-{label}"));
        krate(&r, PROG, "vos-x", "", vos);
        let o = sweep(&r);
        only(
            &o,
            "PB01",
            "\"code/programs/rust/vos-x/Cargo.toml\": `publish = false` is missing from [package]",
        );
    }
    let r = tree("pub-pb01-kindname");
    krate(&r, PKG, "vos-x", "", Some("kind = \"any\"\n"));
    only(&sweep(&r), "PB01", "a crate of kind any");
}

#[test]
fn control_publish_true_or_any_other_value_is_pb01() {
    for extra in [
        "publish = true",
        "publish = [\"crates-io\"]",
        "publish = \"yes\"",
        "[dependencies]\npublish = false",
        "[lib]\npublish = false",
    ] {
        let r = tree("pub-true");
        krate(&r, PKG, "vos-x", extra, None);
        only(&sweep(&r), "PB01", "vos-x/Cargo.toml");
    }
}

#[test]
fn the_literal_false_in_package_passes_in_every_toml_spelling() {
    for extra in ["publish = false", "# comment\npublish   =   false"] {
        let r = tree("pub-false");
        krate(&r, PKG, "vos-x", extra, None);
        assert!(sweep(&r).findings.is_empty());
    }
    let r = tree("pub-false-inline");
    r.write(
        &format!("{PKG}/vos-x/Cargo.toml"),
        "package = { name = \"vos-x\", publish = false }\n",
    );
    assert!(sweep(&r).findings.is_empty());
    let r = tree("pub-false-dotted");
    r.write(
        &format!("{PKG}/vos-x/Cargo.toml"),
        "package.name = \"vos-x\"\npackage.publish = false\n",
    );
    assert!(sweep(&r).findings.is_empty());
    assert_eq!(says_publish_false("[package]\npublish = false\n"), Ok(true));
    assert_eq!(says_publish_false("[package]\npublish = true\n"), Ok(false));
    assert!(says_publish_false("[package\n").is_err());
}

#[test]
fn control_a_production_crate_without_the_flag_passes_only_because_it_is_exempt() {
    let r = tree("pub-prod");
    register(&r, &[("vosImplementationRow", "vos-x")]);
    krate(
        &r,
        PKG,
        "vos-x",
        "",
        Some("class = \"K\"\nkind = \"production\"\n"),
    );
    assert!(sweep(&r).findings.is_empty());
    // The same crate declared `any` is judged.
    r.write(
        &format!("{PKG}/vos-x/vos.toml"),
        "class = \"K\"\nkind = \"any\"\n",
    );
    only(&sweep(&r), "PB01", "vos-x");
}

#[test]
fn scope_directories_without_a_cargo_toml_and_other_names_are_not_swept() {
    let r = tree("pub-scope");
    // A `vos-*` directory with no Cargo.toml (a fixture tree, a stray directory).
    r.write(&format!("{PKG}/vos-nocargo/README.md"), "x\n");
    // A name that merely contains `vos`.
    krate(&r, PKG, "xvos-a", "", None);
    krate(&r, PKG, "vosx", "", None);
    krate(&r, PKG, "VOS-upper", "", None);
    // A nested crate (no recursion).
    krate(
        &r,
        &format!("{PKG}/vos-nocargo/tests/fixtures"),
        "vos-nested",
        "",
        None,
    );
    // A FILE named like a crate.
    r.write(&format!("{PKG}/vos-afile"), "x\n");
    // One real crate, so the count shows what WAS swept.
    krate(&r, PKG, "vos-ok", "publish = false", None);
    let o = sweep(&r);
    assert!(o.findings.is_empty(), "{}", show(&o));
    assert!(o.header.contains("1 crates"), "{}", o.header);
}

#[test]
fn pb02_an_unreadable_cargo_toml_or_vos_toml_is_pb02_not_a_pass() {
    let r = tree("pub-pb02a");
    krate(&r, PKG, "vos-x", "publish = false\n[[", None);
    only(
        &sweep(&r),
        "PB02",
        "\"code/packages/rust/vos-x/Cargo.toml\":",
    );
    let r = tree("pub-pb02b");
    krate(
        &r,
        PKG,
        "vos-x",
        "publish = false",
        Some("kind = \"nonsense\"\n"),
    );
    only(
        &sweep(&r),
        "PB02",
        "\"code/packages/rust/vos-x/vos.toml\":1",
    );
    // A duplicate key is a parse failure of the reader (no silent last-wins).
    let r = tree("pub-pb02c");
    krate(&r, PKG, "vos-x", "publish = true\npublish = false", None);
    only(&sweep(&r), "PB02", "vos-x/Cargo.toml");
}

#[test]
fn a_file_over_its_cap_or_not_a_regular_file_is_no_verdict() {
    let r = tree("pub-big");
    krate(&r, PKG, "vos-x", "publish = false", None);
    r.write(&format!("{PKG}/vos-x/Cargo.toml"), "#".repeat(70_000));
    assert!(no_verdict(&r).to_string().contains("Cargo.toml"));
}

#[test]
fn a_root_that_is_not_a_repository_is_no_verdict() {
    let r = Repo::empty("pub-noroot");
    // `Repo::empty` has code/packages/rust but no code/programs/rust.
    let e = no_verdict(&r);
    assert!(e.to_string().contains("programs"), "{e}");
    let e = check_publish(Some(Path::new("/definitely/not/a/dir/x"))).unwrap_err();
    assert!(!e.to_string().is_empty());
    let e = check_publish(Some(Path::new("a/../b"))).unwrap_err();
    assert!(e.to_string().contains("`..` is not allowed"), "{e}");
}

#[cfg(unix)] // symlink creation needs privileges on Windows; the refusal is the same code path
mod symlinks {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn a_symlinked_crate_directory_is_no_verdict_not_a_pass() {
        let r = tree("pub-link-dir");
        krate(&r, PROG, "vos-real", "publish = false", None);
        symlink(
            r.path(&format!("{PROG}/vos-real")),
            r.path(&format!("{PKG}/vos-alias")),
        )
        .unwrap();
        let e = no_verdict(&r);
        assert!(
            e.to_string().contains("a crate directory is a symlink"),
            "{e}"
        );
    }

    #[test]
    fn a_symlinked_cargo_toml_is_no_verdict() {
        let r = tree("pub-link-toml");
        krate(&r, PKG, "vos-x", "publish = false", None);
        r.write("elsewhere.toml", "[package]\npublish = false\n");
        std::fs::remove_file(r.path(&format!("{PKG}/vos-x/Cargo.toml"))).unwrap();
        symlink(
            r.path("elsewhere.toml"),
            r.path(&format!("{PKG}/vos-x/Cargo.toml")),
        )
        .unwrap();
        let e = no_verdict(&r);
        assert!(e.to_string().contains("symlink"), "{e}");
    }

    #[test]
    fn a_symlinked_scan_root_is_no_verdict() {
        let r = tree("pub-link-root");
        std::fs::remove_dir_all(r.path(PROG)).unwrap();
        std::fs::create_dir_all(r.path("elsewhere")).unwrap();
        symlink(r.path("elsewhere"), r.path(PROG)).unwrap();
        let e = no_verdict(&r);
        assert!(e.to_string().contains("is a symlink"), "{e}");
    }
}

#[test]
fn the_root_is_found_above_the_two_rust_roots_and_nowhere_else() {
    let p = Path::new("/r/code/programs/rust/vos-link/src");
    assert_eq!(root_above(p), Some(Path::new("/r").to_path_buf()));
    let p = Path::new("/r/code/packages/rust/vos-x");
    assert_eq!(root_above(p), Some(Path::new("/r").to_path_buf()));
    assert_eq!(root_above(Path::new("/r/code/packages/python/x")), None);
    assert_eq!(root_above(Path::new("/r/packages/rust/x")), None);
    assert_eq!(root_above(Path::new("/r/other/programs/rust/x")), None);
    assert_eq!(root_above(Path::new("/")), None);
}

#[test]
fn the_real_repository_passes_from_the_crates_own_directory() {
    // `cargo test` runs in this crate's directory, as the BUILD line does: the root is found
    // from the working directory, with no argument.
    let o = check_publish(None).expect("a verdict");
    assert!(o.findings.is_empty(), "{}", show(&o));
    let note = o
        .notes
        .iter()
        .find(|n| n.contains("publish policy undecided"))
        .unwrap();
    assert!(note.contains("\"vos-sched-fp-bitmap\"") && note.contains("\"vos-sched-contract\""));
}

#[test]
fn the_command_line_sweeps_and_exits_0_1_2() {
    let r = tree("pub-cli");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_vos-link"))
            .args(args)
            .output()
            .unwrap()
    };
    let root = r.root.to_string_lossy().into_owned();
    let o = run(&["check-publish", "--root", &root]);
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stdout)
    );
    assert!(String::from_utf8_lossy(&o.stdout).contains("result: PASS, 0 findings"));
    krate(&r, PKG, "vos-x", "", None);
    let o = run(&["check-publish", "--root", &root]);
    assert_eq!(o.status.code(), Some(1));
    let text = String::from_utf8_lossy(&o.stdout);
    assert!(
        text.contains("PB01 \"code/packages/rust/vos-x/Cargo.toml\""),
        "{text}"
    );
    assert!(text.contains("result: FAIL, 1 findings"), "{text}");
    // A crate directory is a usage error, and so is no root outside the repository.
    let o = run(&["check-publish", "somewhere"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("takes no crate directory"));
    let o = run(&["check-publish", "--root", "a/../b"]);
    assert_eq!(o.status.code(), Some(2));
    let o = run(&["check-component"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("no crate directory"));
}

#[test]
fn the_entry_cap_is_no_verdict_and_only_vos_names_are_listed() {
    let r = tree("pub-cap");
    for n in ["vos-b", "vos-a", "other", "xvos-c"] {
        std::fs::create_dir_all(r.path(&format!("{PKG}/{n}"))).unwrap();
    }
    let dir = r.path(PKG);
    // `vos-sched-contract` is there too (copied by `Repo::new`): 5 entries, 3 of them `vos-*`.
    assert_eq!(
        prefixed_children(&dir, 100).unwrap(),
        ["vos-a", "vos-b", "vos-sched-contract"]
    );
    let e = prefixed_children(&dir, 4).unwrap_err();
    assert!(e.to_string().contains("more than 4 entries"), "{e}");
    assert!(prefixed_children(&r.path("nope"), 4).is_err());
}

#[test]
fn a_vos_name_that_is_not_utf8_is_refused_and_other_names_are_not_looked_at() {
    use std::ffi::OsStr;
    assert_eq!(swept_name(OsStr::new("vos-a")), Ok(Some("vos-a")));
    assert_eq!(swept_name(OsStr::new("other")), Ok(None));
    #[cfg(unix)] // a non-UTF-8 OsStr can be built from bytes on Unix only
    {
        use std::os::unix::ffi::OsStrExt;
        assert_eq!(swept_name(OsStr::from_bytes(b"vos-\xff")), Err(()));
        assert_eq!(swept_name(OsStr::from_bytes(b"\xffvos-")), Ok(None));
    }
}

#[test]
fn the_command_line_outside_any_repository_is_no_verdict() {
    // No --root, and a working directory that is not under code/(packages|programs)/rust.
    let r = tree("pub-cwd");
    let o = Command::new(env!("CARGO_BIN_EXE_vos-link"))
        .arg("check-publish")
        .current_dir(&r.root)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(
        err.contains("not inside code/packages/rust or code/programs/rust"),
        "{err}"
    );
}

#[test]
fn pb03_a_kind_claim_without_a_registered_row_is_not_exempt() {
    for kind in ["production", "reference", "contract"] {
        // No row at all, a row of another crate, a commented row and a row of the wrong shape.
        let r = tree(&format!("pub-pb03-{kind}"));
        krate(
            &r,
            PROG,
            "vos-x",
            "",
            Some(&format!("class = \"K\"\nkind = \"{kind}\"\n")),
        );
        register(&r, &[("vosImplementationRow", "vos-other")]);
        let o = sweep(&r);
        only(
            &o,
            "PB03",
            &format!("claims kind {kind} but is not a registered VOS component"),
        );
        assert!(o.findings[0].message.contains("set publish = false"));
        assert!(
            o.notes.iter().all(|n| !n.contains("exempt")),
            "{}",
            show(&o)
        );
    }
    let r = tree("pub-pb03-comment");
    krate(&r, PROG, "vos-x", "", Some("kind = \"production\"\n"));
    r.write(
        ROWS,
        "package main\nvar repositoryInputConsumers = []repositoryInputConsumer{\n\t// vosImplementationRow(\"vos-x\"),\n}\n",
    );
    only(&sweep(&r), "PB03", "vos-x");
    // Either row function registers it, whatever the kind.
    for f in ["vosImplementationRow", "vosContractRow"] {
        let r = tree("pub-pb03-ok");
        krate(&r, PROG, "vos-x", "", Some("kind = \"production\"\n"));
        register(&r, &[(f, "vos-x")]);
        let o = sweep(&r);
        assert!(o.findings.is_empty(), "{}", show(&o));
    }
}

#[test]
fn pb03_a_missing_or_unreadable_row_file_is_no_verdict() {
    let r = tree("pub-pb03-nofile");
    krate(&r, PROG, "vos-x", "", Some("kind = \"production\"\n"));
    let e = no_verdict(&r);
    assert!(e.to_string().contains("repository_inputs.go"), "{e}");
    // Not UTF-8.
    r.write(ROWS, [0xffu8, 0xfe]);
    assert!(no_verdict(&r).to_string().contains("not UTF-8"));
    // A tree with no exempt crate never needs the file.
    let r = tree("pub-pb03-unneeded");
    krate(&r, PROG, "vos-x", "publish = false", None);
    assert!(sweep(&r).findings.is_empty());
}

#[cfg(unix)] // Windows forbids a newline and an escape in a file name
#[test]
fn a_hostile_directory_name_cannot_forge_a_report_line() {
    let r = tree("pub-inject");
    let bare = |name: &str, vos: Option<&str>| {
        // (A fixed `name` in the manifest: a newline in it would be invalid TOML, PB02.)
        r.write(
            &format!("{PKG}/{name}/Cargo.toml"),
            "[package]\nname = \"vos-x\"\nversion = \"0.1.0\"\n",
        );
        if let Some(v) = vos {
            r.write(&format!("{PKG}/{name}/vos.toml"), v);
        }
    };
    for evil in ["vos-evil\nresult: PASS", "vos-esc\u{1b}[2Jx"] {
        bare(evil, None);
    }
    // An exempt-kind one reaches the `exempt` note (its row is one line, so no newline here).
    bare("vos-ex\u{1b}[2Jy", Some("kind = \"production\"\n"));
    register(&r, &[("vosImplementationRow", "vos-ex\u{1b}[2Jy")]);
    let o = sweep(&r);
    assert_eq!(o.findings.len(), 2, "{}", show(&o));
    for f in &o.findings {
        assert_eq!(f.code, "PB01");
        assert!(
            !f.message.contains('\n') && !f.message.contains('\u{1b}'),
            "{}",
            f.message
        );
    }
    for n in &o.notes {
        assert!(!n.contains('\n') && !n.contains('\u{1b}'), "{n}");
    }
    assert!(o.notes.iter().any(|n| n.contains("vos-ex\\u{1b}[2Jy")));
    assert!(o
        .findings
        .iter()
        .any(|f| f.message.contains("vos-evil\\nresult: PASS")));
}
