//! Path containment, caps and the write path (VOS02 section 6.8, NC18).  Every refusal here
//! is a "no verdict" (exit 2): the rules could not even be applied.

mod common;
use common::*;

use std::path::Path;
use vos_link::fsio;

const SCAN: &str = "vos-fixture-scan";

fn nv(r: Result<vos_link::Outcome, vos_link::NoVerdict>) -> String {
    r.expect_err("expected no verdict").to_string()
}

#[test]
fn caps_are_no_verdict_not_findings() {
    let p = "code/packages/rust/vos-fixture-scan/warrant/impl.part";
    let r = Repo::with("cap1", SCAN);
    r.write(p, vec![b' '; 1024 * 1024 + 1]);
    assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("over the 1048576-byte cap"));
    let r = Repo::with("cap2", SCAN);
    r.write(
        "code/packages/rust/vos-sched-contract/contract.vos",
        vec![b'a'; 64 * 1024 + 1],
    );
    assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("byte cap"));
    let r = Repo::with("cap3", SCAN);
    r.write(
        "code/packages/rust/vos-fixture-scan/vos.toml",
        vec![b'#'; 4097],
    );
    assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("byte cap"));
    // A long line inside the file cap.
    let r = Repo::with("cap4", SCAN);
    r.write(p, format!("{}\n", "a".repeat(4_097)));
    assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("a line is over 4096 bytes"));
    let r = Repo::with("cap5", SCAN);
    r.write(
        "code/packages/rust/vos-sched-contract/contract.vos",
        format!("vos-contract 1\nname {}\nend\n", "a".repeat(1_100)),
    );
    assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("a line is over"));
    // An oversized document on disk.
    let r = Repo::with("cap6", SCAN);
    r.gen(SCAN);
    r.write(
        "code/specs/vos-fixture-scan.warrant",
        vec![b'a'; 1024 * 1024 + 1],
    );
    assert!(nv(vos_link::check_doc(&r.pkg(SCAN), None)).contains("byte cap"));
    // Many entries in a directory.
    let r = Repo::with("cap7", SCAN);
    for i in 0..70 {
        r.write(
            &format!("code/packages/rust/vos-sched-contract/warrant/extra{i}.txt"),
            "x",
        );
    }
    assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("more than 64 entries"));
}

#[test]
fn the_crate_must_live_at_code_packages_rust_name() {
    let r = Repo::with("layout1", SCAN);
    std::fs::create_dir_all(r.path("elsewhere/vos-fixture-scan")).unwrap();
    assert!(nv(vos_link::gen_doc(
        &r.path("elsewhere/vos-fixture-scan"),
        None
    ))
    .contains("lives at <root>/code/packages/rust/<package>"));
    std::fs::create_dir_all(r.path("code/programs/rust/vos-fixture-scan")).unwrap();
    assert!(nv(vos_link::gen_doc(
        &r.path("code/programs/rust/vos-fixture-scan"),
        None
    ))
    .contains("lives at"));
    // A directory whose name is not a package name.
    std::fs::create_dir_all(r.path("code/packages/rust/Bad_Name")).unwrap();
    assert!(nv(vos_link::gen_doc(
        &r.path("code/packages/rust/Bad_Name"),
        None
    ))
    .contains("is not a package name"));
    // A missing directory.
    assert!(nv(vos_link::gen_doc(
        &r.path("code/packages/rust/nothing-here"),
        None
    ))
    .contains("cannot resolve"));
    assert!(nv(vos_link::gen_doc(Path::new(""), None)).contains("empty path"));
}

#[test]
fn root_must_agree_with_the_crate_location() {
    let r = Repo::with("root1", SCAN);
    let other = Repo::new("root2");
    assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), Some(&other.root)))
        .contains("is not the repository root"));
    let ok = vos_link::gen_doc(&r.pkg(SCAN), Some(&r.root)).unwrap();
    assert!(ok.findings.is_empty());
}

#[test]
fn dot_dot_is_refused_before_anything_is_resolved() {
    let r = Repo::with("dots", SCAN);
    let p = r.path("code/packages/rust/vos-sched-contract/../vos-fixture-scan");
    assert!(nv(vos_link::gen_doc(&p, None)).contains("`..` is not allowed"));
    assert!(
        nv(vos_link::gen_doc(&r.pkg(SCAN), Some(&r.path("code/.."))))
            .contains("`..` is not allowed")
    );
    assert!(nv(vos_link::check_doc(&p, None)).contains("`..`"));
}

#[test]
fn a_relative_path_and_a_dot_component_are_fine() {
    let r = Repo::with("rel", SCAN);
    let cwd = std::env::current_dir().unwrap();
    let abs = fsio::absolute_plain(&r.pkg(SCAN).join(".")).unwrap();
    assert_eq!(abs, r.pkg(SCAN));
    // Relative to the (canonical) working directory: use a path that cannot exist.
    let e = fsio::absolute_plain(Path::new("definitely/not/here")).unwrap_err();
    assert!(e.to_string().contains("cannot resolve"), "{e}");
    assert!(cwd.is_absolute());
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn a_symlinked_crate_directory_or_root_component_is_refused() {
        let r = Repo::with("sym1", SCAN);
        symlink(r.pkg(SCAN), r.path("code/packages/rust/vos-alias")).unwrap();
        assert!(nv(vos_link::gen_doc(
            &r.path("code/packages/rust/vos-alias"),
            None
        ))
        .contains("a component is a symlink"));
        // A path that reaches the crate through a symlinked directory.
        symlink(r.path("code/packages/rust"), r.path("rust-link")).unwrap();
        assert!(nv(vos_link::gen_doc(
            &r.path("rust-link/vos-fixture-scan"),
            None
        ))
        .contains("a component is a symlink"));
    }

    #[test]
    fn a_symlink_at_the_output_path_fails() {
        let r = Repo::with("sym2", SCAN);
        std::fs::write(r.path("elsewhere.txt"), "do not overwrite").unwrap();
        symlink(
            r.path("elsewhere.txt"),
            r.path("code/specs/vos-fixture-scan.warrant"),
        )
        .unwrap();
        assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("is a symlink"));
        assert_eq!(
            std::fs::read_to_string(r.path("elsewhere.txt")).unwrap(),
            "do not overwrite"
        );
        assert!(nv(vos_link::check_doc(&r.pkg(SCAN), None)).contains("is a symlink"));
        // Even a dangling one.
        let r = Repo::with("sym2b", SCAN);
        symlink(
            r.path("nowhere"),
            r.path("code/specs/vos-fixture-scan.warrant"),
        )
        .unwrap();
        assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("is a symlink"));
        assert!(!r.path("nowhere").exists());
    }

    #[test]
    fn a_symlinked_specs_directory_fails() {
        let r = Repo::with("sym3", SCAN);
        std::fs::remove_dir(r.path("code/specs")).unwrap();
        std::fs::create_dir(r.path("outside")).unwrap();
        symlink(r.path("outside"), r.path("code/specs")).unwrap();
        assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("is a symlink"));
        assert!(std::fs::read_dir(r.path("outside"))
            .unwrap()
            .next()
            .is_none());
        assert!(nv(vos_link::check_doc(&r.pkg(SCAN), None)).contains("is a symlink"));
    }

    #[test]
    fn a_missing_specs_directory_is_no_verdict() {
        let r = Repo::with("sym3b", SCAN);
        std::fs::remove_dir(r.path("code/specs")).unwrap();
        assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("cannot stat"));
    }

    #[test]
    fn symlinked_inputs_are_refused() {
        for (label, rel) in [
            (
                "impl.part",
                "code/packages/rust/vos-fixture-scan/warrant/impl.part",
            ),
            ("vos.toml", "code/packages/rust/vos-fixture-scan/vos.toml"),
            (
                "contract.part",
                "code/packages/rust/vos-sched-contract/warrant/contract.part",
            ),
            (
                "contract.vos",
                "code/packages/rust/vos-sched-contract/contract.vos",
            ),
        ] {
            let r = Repo::with("sym4", SCAN);
            let target = r.path("copy.txt");
            std::fs::copy(r.path(rel), &target).unwrap();
            std::fs::remove_file(r.path(rel)).unwrap();
            symlink(&target, r.path(rel)).unwrap();
            assert!(
                nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("is a symlink"),
                "{label}"
            );
        }
        // A symlinked warrant directory and a symlinked contract package.
        let r = Repo::with("sym5", SCAN);
        let w = r.path("code/packages/rust/vos-fixture-scan/warrant");
        std::fs::rename(&w, r.path("moved")).unwrap();
        symlink(r.path("moved"), &w).unwrap();
        assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("is a symlink"));
        let r = Repo::with("sym6", SCAN);
        let c = r.pkg("vos-sched-contract");
        std::fs::rename(&c, r.path("moved")).unwrap();
        symlink(r.path("moved"), &c).unwrap();
        assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("is a symlink"));
        // code/packages/rust itself a symlink.
        let r = Repo::with("sym7", SCAN);
        std::fs::rename(r.path("code/packages/rust"), r.path("rustreal")).unwrap();
        symlink(r.path("rustreal"), r.path("code/packages/rust")).unwrap();
        assert!(vos_link::gen_doc(&r.path("code/packages/rust/vos-fixture-scan"), None).is_err());
    }

    #[test]
    fn a_fifo_or_directory_where_a_file_belongs_is_refused() {
        let r = Repo::with("fifo", SCAN);
        let p = r.path("code/packages/rust/vos-fixture-scan/warrant/impl.part");
        std::fs::remove_file(&p).unwrap();
        let st = std::process::Command::new("mkfifo")
            .arg(&p)
            .status()
            .unwrap();
        assert!(st.success());
        // A regression that opened the FIFO would block forever: bound it.
        let pkg = r.pkg(SCAN);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(vos_link::gen_doc(&pkg, None));
        });
        let res = rx
            .recv_timeout(std::time::Duration::from_secs(20))
            .expect("gen_doc hung on a FIFO");
        assert!(nv(res).contains("not a regular file"));
        let r = Repo::with("dirdoc", SCAN);
        std::fs::create_dir(r.path("code/specs/vos-fixture-scan.warrant")).unwrap();
        assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("not a regular file"));
        // A file where a directory belongs.
        let r = Repo::with("filedir", SCAN);
        std::fs::remove_dir_all(r.path("code/specs")).unwrap();
        std::fs::write(r.path("code/specs"), "x").unwrap();
        assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("not a directory"));
        let r = Repo::with("filedir2", SCAN);
        std::fs::remove_dir_all(r.path("code/packages/rust/vos-fixture-scan/warrant")).unwrap();
        std::fs::write(r.path("code/packages/rust/vos-fixture-scan/warrant"), "x").unwrap();
        assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("not a directory"));
        let r = Repo::with("filedir3", SCAN);
        std::fs::remove_dir_all(r.path("code/packages/rust/vos-sched-contract")).unwrap();
        std::fs::write(r.path("code/packages/rust/vos-sched-contract"), "x").unwrap();
        assert!(nv(vos_link::gen_doc(&r.pkg(SCAN), None)).contains("not a directory"));
    }

    #[test]
    fn a_stale_temporary_file_does_not_block_the_next_run() {
        // The temporary name is unique per process and call (S-L2): a file a crashed run
        // left behind, under the old fixed name or any other, is neither used nor in the way.
        let r = Repo::with("tmp1", SCAN);
        for stale in [
            ".vos-fixture-scan.warrant.vos-link.tmp",
            ".vos-fixture-scan.warrant.1.0.vos-link.tmp",
        ] {
            std::fs::write(r.path(&format!("code/specs/{stale}")), "stale").unwrap();
        }
        let g = vos_link::gen_doc(&r.pkg(SCAN), None).unwrap();
        assert!(g.findings.is_empty());
        assert_eq!(
            std::fs::read_to_string(r.path("code/specs/.vos-fixture-scan.warrant.vos-link.tmp"))
                .unwrap(),
            "stale",
            "the stale file is left alone"
        );
        assert!(r.check(SCAN).findings.is_empty());
    }

    fn is_root() -> bool {
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
            .unwrap_or(false)
    }

    #[test]
    fn an_unwritable_specs_directory_fails_the_write() {
        use std::os::unix::fs::PermissionsExt;
        if is_root() {
            eprintln!("SKIPPED: running as root, which ignores directory modes");
            return;
        }
        let r = Repo::with("tmp2", SCAN);
        std::fs::set_permissions(r.path("code/specs"), std::fs::Permissions::from_mode(0o555))
            .unwrap();
        let res = vos_link::gen_doc(&r.pkg(SCAN), None);
        std::fs::set_permissions(r.path("code/specs"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
        let e = res.expect_err("a read-only directory must fail the write");
        assert!(
            e.to_string().contains("cannot create the temporary file"),
            "{e}"
        );
    }

    #[test]
    fn write_replacing_replaces_a_regular_file_and_leaves_no_temporary() {
        let r = Repo::new("wr");
        let dir = r.path("code/specs");
        fsio::write_replacing(&dir, "a.txt", b"one").unwrap();
        fsio::write_replacing(&dir, "a.txt", b"two").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "two");
        let names = fsio::list_names(&dir).unwrap();
        assert_eq!(names, ["a.txt"]);
        assert_eq!(
            fsio::list_names(&r.path("nope")).unwrap(),
            Vec::<String>::new()
        );
        assert!(fsio::list_names(&dir.join("a.txt")).is_err());
        assert!(fsio::read_regular(&dir.join("zzz"), 10).is_err());
        assert!(fsio::read_regular(&dir.join("a.txt"), 2)
            .unwrap_err()
            .to_string()
            .contains("over the 2-byte cap"));
        assert_eq!(fsio::read_regular(&dir.join("a.txt"), 3).unwrap(), b"two");
    }
}

#[test]
fn check_doc_reports_input_findings_like_gen_doc() {
    let r = Repo::with("inputs-check", SCAN);
    r.write(
        "code/packages/rust/vos-fixture-scan/vos.toml",
        "kind = \"spine\"\n",
    );
    let c = vos_link::check_doc(&r.pkg(SCAN), None).unwrap();
    assert!(!c.findings.is_empty());
    assert!(c.findings.iter().all(|f| f.code == "GD05"));
}

#[test]
fn the_filesystem_root_is_not_a_package_directory() {
    assert!(nv(vos_link::gen_doc(Path::new("/"), None)).contains("not a package directory"));
}

#[test]
fn windows_path_text_is_compared_after_the_verbatim_prefix_is_dropped() {
    // S-L3: `canonicalize` on Windows returns `\\?\C:\...`; the cleaned path the user typed
    // is `C:\...`.  The helpers are pure string work, so they are tested on every platform.
    assert_eq!(fsio::strip_verbatim(r"\\?\C:\repo\code"), r"C:\repo\code");
    assert_eq!(
        fsio::strip_verbatim(r"\\?\UNC\host\share\x"),
        r"\\host\share\x"
    );
    assert_eq!(fsio::strip_verbatim("/usr/lib"), "/usr/lib");
    assert!(fsio::same_path_text(
        r"\\?\C:\Repo\code",
        r"c:\repo\code",
        true
    ));
    assert!(fsio::same_path_text(
        r"\\?\C:\repo\code",
        "C:/repo/code",
        true
    ));
    assert!(!fsio::same_path_text(r"\\?\C:\repo", r"C:\other", true));
    assert!(
        !fsio::same_path_text("/a/B", "/a/b", false),
        "Unix compares exactly"
    );
    assert!(fsio::same_path_text("/a/b", "/a/b", false));
}

#[cfg(target_os = "macos")]
#[test]
fn a_path_that_is_not_in_canonical_form_is_refused_not_trusted() {
    // On macOS the temporary directory is reached through the symlink /var -> /private/var.
    // The canonical root works (every other test); the same root typed through the symlink
    // is refused, which is the check that must not be defeated by a Windows-style prefix.
    let r = Repo::with("noncanon", SCAN);
    let canon = r.root.to_string_lossy().into_owned();
    if let Some(rest) = canon.strip_prefix("/private") {
        let typed = std::path::PathBuf::from(format!("{rest}/code/packages/rust/{SCAN}"));
        assert!(nv(vos_link::gen_doc(&typed, None)).contains("a component is a symlink"));
    }
}

#[test]
fn a_disagreeing_root_is_no_verdict_for_check_doc_too() {
    let r = Repo::with("root3", SCAN);
    let other = Repo::new("root4");
    assert!(nv(vos_link::check_doc(&r.pkg(SCAN), Some(&other.root)))
        .contains("is not the repository root"));
}

/// Windows regression: `canonicalize` returns a verbatim path (`\\?\C:\..`), and std does
/// not parse `..` in a verbatim path as `ParentDir`, so the component check alone let a
/// `..` through on Windows CI.  The text check must catch it for both separators, and it
/// is pure string work so this runs on every platform.
#[test]
fn dot_dot_text_is_found_for_both_separators_and_verbatim_paths() {
    let w = fsio::has_parent_component;
    assert!(w(r"\\?\C:\r\code\..\x", true));
    assert!(w(r"\\?\C:\r\code\packages/../x", true));
    assert!(w(r"C:\r/code/..", true));
    assert!(w("a/../b", true) && w("a/../b", false));
    assert!(w("..", false) && w("..", true));
    // `..` inside a name, a single dot, a `...` name: not a parent component
    assert!(!w(r"\\?\C:\r\..a\b..\...\.x", true));
    assert!(!w("a/./b/..c", false));
    // a backslash is an ordinary name character on Unix
    assert!(!w(r"a\..\b", false));
    assert!(w(r"a\..\b", true));
}
