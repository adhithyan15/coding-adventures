//! The command line: exit codes 0, 1 and 2, the filesystem discipline of VOS02
//! section 6.8 (no symlinks, regular files only, bounded reads), determinism.

mod common;
use common::materialize;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vos-shape-check"))
}

fn run_in(dir: &Path, extra: &[&str]) -> Output {
    bin().arg(dir).args(extra).output().unwrap()
}

fn code(o: &Output) -> i32 {
    o.status.code().unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn exit_0_and_pass_report_for_the_toy() {
    let d = materialize("pass-ready-bitmap", "cli-pass");
    let o = run_in(&d, &[]);
    assert_eq!(code(&o), 0, "{}{}", out(&o), err(&o));
    assert_eq!(out(&o), "vos-shape-check: crate vos-ready-bitmap, class K (vos.toml), kind production\nresult: PASS, 0 findings\n");
}

#[test]
fn exit_1_and_findings_for_broken_toys() {
    for (name, rule) in [
        ("fail-bitmap-ghost-leak", "SH18"),
        ("fail-bitmap-build-rs", "SH15"),
        ("fail-bitmap-image-drift", "SH17"),
    ] {
        let d = materialize(name, &format!("cli-{name}"));
        let o = run_in(&d, &[]);
        assert_eq!(code(&o), 1, "{name}");
        let s = out(&o);
        assert!(s.contains(&format!("{rule} ")), "{s}");
        assert!(s.contains("result: FAIL"), "{s}");
    }
}

#[test]
fn the_report_is_byte_identical_across_runs() {
    let d = materialize("fail-bitmap-ghost-leak", "cli-det");
    let a = run_in(&d, &[]);
    let b = run_in(&d, &[]);
    assert_eq!(a.stdout, b.stdout);
}

#[test]
fn the_build_rs_file_is_found_even_if_it_is_a_symlink() {
    // `build.rs` is detected by name from the directory listing, never read.
    let d = materialize("pass-ready-bitmap", "cli-buildrs-link");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/nonexistent", d.join("build.rs")).unwrap();
        let o = run_in(&d, &[]);
        assert_eq!(code(&o), 1, "{}", out(&o));
        assert!(out(&o).contains("SH15 build.rs:1"));
    }
}

#[test]
fn help_and_usage_errors() {
    let o = bin().arg("--help").output().unwrap();
    assert_eq!(code(&o), 0);
    assert!(out(&o).starts_with("usage: vos-shape-check"));
    let o = bin().arg("-h").output().unwrap();
    assert_eq!(code(&o), 0);
    for args in [
        vec![],
        vec!["--bogus"],
        vec!["a", "b"],
        vec!["--evidence-features"],
        vec!["x", "--evidence-features"],
    ] {
        let o = bin().args(&args).output().unwrap();
        assert_eq!(code(&o), 2, "{args:?}");
        assert!(err(&o).contains("usage:"), "{args:?}");
    }
    let long = "n".repeat(40);
    let o = bin()
        .args(["x", "--evidence-features", &long])
        .output()
        .unwrap();
    assert_eq!(code(&o), 2);
    let many = (0..70)
        .map(|n| format!("f{n}"))
        .collect::<Vec<_>>()
        .join(",");
    let o = bin()
        .args(["x", "--evidence-features", &many])
        .output()
        .unwrap();
    assert_eq!(code(&o), 2);
}

#[test]
fn the_central_evidence_feature_list_is_an_argument() {
    let d = materialize("pass-ready-bitmap", "cli-central");
    // `ghost` is not in this central list, so vos.toml declaring it is VT01 and the gating is SH18
    let o = run_in(&d, &["--evidence-features", "cov"]);
    assert_eq!(code(&o), 1);
    assert!(out(&o).contains("VT01 vos.toml"), "{}", out(&o));
    let o = run_in(&d, &["--evidence-features", "ghost,cov"]);
    assert_eq!(code(&o), 0, "{}", out(&o));
}

#[test]
fn exit_2_when_the_directory_is_not_a_crate() {
    let o = bin()
        .arg("/nonexistent/vos-shape-check-dir")
        .output()
        .unwrap();
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("no verdict"));
    let d = materialize("pass-ready-bitmap", "cli-nocargo");
    fs::remove_file(d.join("Cargo.toml")).unwrap();
    let o = run_in(&d, &[]);
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("no Cargo.toml"));
    // a file, not a directory
    let d = materialize("pass-ready-bitmap", "cli-file");
    let o = run_in(&d.join("Cargo.toml"), &[]);
    assert_eq!(code(&o), 2);
}

#[cfg(unix)]
#[test]
fn exit_2_on_symlinks_anywhere_under_the_crate() {
    use std::os::unix::fs::symlink;
    let base = materialize("pass-ready-bitmap", "cli-sym");
    // the crate directory itself
    let link = base.with_file_name(format!(
        "{}-link",
        base.file_name().unwrap().to_string_lossy()
    ));
    let _ = fs::remove_file(&link);
    symlink(&base, &link).unwrap();
    assert_eq!(code(&run_in(&link, &[])), 2);
    fs::remove_file(&link).unwrap();
    // a source file
    let d = materialize("pass-ready-bitmap", "cli-sym-src");
    symlink(d.join("src/lib.rs"), d.join("src/other.rs")).unwrap();
    let o = run_in(&d, &[]);
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("symlink"), "{}", err(&o));
    // the src directory
    let d = materialize("pass-ready-bitmap", "cli-sym-dir");
    fs::rename(d.join("src"), d.join("real-src")).unwrap();
    symlink(d.join("real-src"), d.join("src")).unwrap();
    assert_eq!(code(&run_in(&d, &[])), 2);
    // Cargo.toml and vos.toml
    let d = materialize("pass-ready-bitmap", "cli-sym-cargo");
    fs::rename(d.join("Cargo.toml"), d.join("Cargo.real")).unwrap();
    symlink(d.join("Cargo.real"), d.join("Cargo.toml")).unwrap();
    assert_eq!(code(&run_in(&d, &[])), 2);
    let d = materialize("pass-ready-bitmap", "cli-sym-vos");
    fs::rename(d.join("vos.toml"), d.join("vos.real")).unwrap();
    symlink(d.join("vos.real"), d.join("vos.toml")).unwrap();
    assert_eq!(code(&run_in(&d, &[])), 2);
}

#[test]
fn exit_2_on_oversize_non_utf8_and_structural_limits() {
    // Cargo.toml over its cap
    let d = materialize("pass-ready-bitmap", "cli-cap-cargo");
    fs::write(d.join("Cargo.toml"), "x".repeat(70_000)).unwrap();
    let o = run_in(&d, &[]);
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("cap"), "{}", err(&o));
    // vos.toml over its cap
    let d = materialize("pass-ready-bitmap", "cli-cap-vos");
    fs::write(d.join("vos.toml"), "#".repeat(5000)).unwrap();
    assert_eq!(code(&run_in(&d, &[])), 2);
    // a source file over its cap
    let d = materialize("pass-ready-bitmap", "cli-cap-src");
    fs::write(d.join("src/big.rs"), "/".repeat(1_100_000)).unwrap();
    assert_eq!(code(&run_in(&d, &[])), 2);
    // not UTF-8
    let d = materialize("pass-ready-bitmap", "cli-utf8");
    fs::write(d.join("src/bad.rs"), [0xff, 0xfe, 0x00]).unwrap();
    let o = run_in(&d, &[]);
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("UTF-8"));
    // `src` is a regular file
    let d = materialize("pass-ready-bitmap", "cli-src-file");
    fs::remove_dir_all(d.join("src")).unwrap();
    fs::write(d.join("src"), "x").unwrap();
    assert_eq!(code(&run_in(&d, &[])), 2);
    // too deep
    let d = materialize("pass-ready-bitmap", "cli-deep");
    let mut p = d.join("src");
    for n in 0..18 {
        p = p.join(format!("d{n}"));
    }
    fs::create_dir_all(&p).unwrap();
    assert_eq!(code(&run_in(&d, &[])), 2);
    // too many source files
    let d = materialize("pass-ready-bitmap", "cli-many");
    for n in 0..520 {
        fs::write(d.join(format!("src/m{n}.rs")), "").unwrap();
    }
    assert_eq!(code(&run_in(&d, &[])), 2);
}

#[test]
fn nested_modules_and_non_rust_files_are_read_or_ignored() {
    let d = materialize("pass-ready-bitmap", "cli-nested");
    fs::create_dir_all(d.join("src/sub")).unwrap();
    fs::write(d.join("src/sub/mod.rs"), "pub fn f() {}\n").unwrap();
    fs::write(d.join("src/notes.txt"), "unsafe { }").unwrap();
    let o = run_in(&d, &[]);
    assert_eq!(code(&o), 0, "{}", out(&o));
    // a violation in a nested module is found, with its path
    fs::write(
        d.join("src/sub/mod.rs"),
        "pub fn f(x: u8) -> u8 { x / 2 }\n",
    )
    .unwrap();
    let o = run_in(&d, &[]);
    assert_eq!(code(&o), 1);
    assert!(out(&o).contains("SH7 src/sub/mod.rs:1"), "{}", out(&o));
}

#[test]
fn a_crate_without_a_src_directory_has_no_crate_root() {
    let d = materialize("pass-ready-bitmap", "cli-nosrc");
    fs::remove_dir_all(d.join("src")).unwrap();
    let o = run_in(&d, &[]);
    assert_eq!(code(&o), 1);
    assert!(out(&o).contains("VT03"), "{}", out(&o));
}

#[test]
fn a_crate_without_vos_toml_is_class_k_by_default() {
    let d = materialize("pass-ready-bitmap", "cli-novos");
    fs::remove_file(d.join("vos.toml")).unwrap();
    // without vos.toml the `ghost` feature is still central, so the crate passes as class K
    let o = run_in(&d, &[]);
    assert_eq!(code(&o), 0, "{}", out(&o));
    assert!(out(&o).contains("class K (default), kind unspecified"));
}

#[test]
fn exit_2_for_non_regular_files_and_unreadable_entries() {
    // Cargo.toml is a directory
    let d = materialize("pass-ready-bitmap", "cli-cargo-dir");
    fs::remove_file(d.join("Cargo.toml")).unwrap();
    fs::create_dir(d.join("Cargo.toml")).unwrap();
    let o = run_in(&d, &[]);
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("not a regular file"), "{}", err(&o));
    // too many root entries
    let d = materialize("pass-ready-bitmap", "cli-root-many");
    for n in 0..4100 {
        fs::write(d.join(format!("f{n}")), "").unwrap();
    }
    assert_eq!(code(&run_in(&d, &[])), 2);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // a FIFO under src/ is neither a regular file nor a directory
        let d = materialize("pass-ready-bitmap", "cli-fifo");
        let st = Command::new("mkfifo")
            .arg(d.join("src/pipe"))
            .status()
            .unwrap();
        assert!(st.success());
        let o = run_in(&d, &[]);
        assert_eq!(code(&o), 2);
        assert!(
            err(&o).contains("not a regular file or directory"),
            "{}",
            err(&o)
        );
        // an unreadable source file (skipped when running as root)
        let d = materialize("pass-ready-bitmap", "cli-noread");
        let p = d.join("src/lib.rs");
        fs::set_permissions(&p, fs::Permissions::from_mode(0o000)).unwrap();
        let readable = fs::read(&p).is_ok();
        let o = run_in(&d, &[]);
        fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
        if !readable {
            assert_eq!(code(&o), 2);
        }
        // an unreadable src directory
        let d = materialize("pass-ready-bitmap", "cli-noread-dir");
        let s = d.join("src");
        fs::set_permissions(&s, fs::Permissions::from_mode(0o000)).unwrap();
        let readable = fs::read_dir(&s).is_ok();
        let o = run_in(&d, &[]);
        fs::set_permissions(&s, fs::Permissions::from_mode(0o755)).unwrap();
        if !readable {
            assert_eq!(code(&o), 2);
        }
    }
}
