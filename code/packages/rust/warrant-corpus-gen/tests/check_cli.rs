//! `--check`, the atomic writer, the path guard and the command line (VER04r 13, slice S3c).
//!
//! Every negative control here seeds one wrong corpus on disk and requires `--check` to fail
//! with the location of the difference: a flipped byte, a missing chunk file, an extra file, a
//! reordered key, a CRLF, a stale profile.  Write mode must refuse every path that could leave
//! the output directory.  The process tests run the real `warrant-corpus-gen` binary.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use coding_adventures_warrant_corpus_gen::cli::{parse_binding_faults, run};
use coding_adventures_warrant_corpus_gen::fsio::{check_corpus, first_difference, guard, write_corpus};
use coding_adventures_warrant_corpus_gen::{code, generate, CorpusV2, GenInput};
use support::*;

const PROFILE: &str = include_str!("../../../../specs/fixtures/ct-compare-warrant-v2/corpus.profile");

fn ct() -> &'static CorpusV2 {
    static C: OnceLock<CorpusV2> = OnceLock::new();
    C.get_or_init(|| {
        with_file(CT_COMPARE, |f| {
            generate(&GenInput {
                file: f,
                spec_path: "code/specs/ct-compare.warrant",
                spec_bytes: CT_COMPARE.as_bytes(),
                profile_bytes: PROFILE.as_bytes(),
                binding_faults: None,
            })
            .unwrap()
        })
    })
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("wcg-s3c-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn written(name: &str) -> PathBuf {
    let d = tmp(name);
    write_corpus(&d, ct()).unwrap();
    d
}

fn drift(d: &Path) -> String {
    let e = check_corpus(d, ct()).expect_err("must drift");
    assert_eq!(e.code, code::DRIFT);
    e.message
}

// ---- write and check --------------------------------------------------------------------

#[test]
fn a_fresh_write_checks_clean_and_rewriting_is_idempotent() {
    let d = written("fresh");
    check_corpus(&d, ct()).unwrap();
    assert_eq!(fs::read_to_string(d.join("corpus.json")).unwrap(), ct().corpus_json);
    assert_eq!(fs::read_to_string(d.join("chunks/select_picks.txt")).unwrap(), ct().chunk_files.iter().find(|(n, _)| n == "select_picks.txt").unwrap().1);
    write_corpus(&d, ct()).unwrap();
    check_corpus(&d, ct()).unwrap();
    // no temporary file is left behind
    for dir in [d.clone(), d.join("chunks")] {
        for e in fs::read_dir(&dir).unwrap() {
            assert!(!e.unwrap().file_name().to_string_lossy().ends_with(".tmp"));
        }
    }
    // other authored files of a fixture directory are not the corpus's business
    fs::write(d.join("README.md"), "authored").unwrap();
    fs::write(d.join("corpus.profile"), PROFILE).unwrap();
    check_corpus(&d, ct()).unwrap();
    let _ = fs::remove_dir_all(d);
}

#[test]
fn a_flipped_byte_in_corpus_json_is_reported_with_its_location() {
    let d = written("flip");
    let mut b = fs::read(d.join("corpus.json")).unwrap();
    let at = b.iter().position(|&c| c == b'4').unwrap();
    b[at] = b'5';
    fs::write(d.join("corpus.json"), &b).unwrap();
    let m = drift(&d);
    let line = 1 + ct().corpus_json.as_bytes()[..at].iter().filter(|&&c| c == b'\n').count();
    assert!(m.starts_with("corpus.json: differs at line "), "{m}");
    assert!(m.contains(&format!("line {line},")) && m.contains(&format!("(byte {at})")), "{m}");
    let _ = fs::remove_dir_all(d);
}

#[test]
fn a_flipped_hex_digit_in_a_chunk_line_is_reported() {
    let d = written("chunkflip");
    let p = d.join("chunks/eq_agrees.txt");
    let mut b = fs::read(&p).unwrap();
    let n = b.len() - 3;
    b[n] = if b[n] == b'0' { b'1' } else { b'0' };
    fs::write(&p, b).unwrap();
    let m = drift(&d);
    assert!(m.starts_with("chunks/eq_agrees.txt: differs at line 2"), "{m}");
    let _ = fs::remove_dir_all(d);
}

#[test]
fn a_missing_chunk_file_an_extra_file_and_a_missing_corpus_are_reported() {
    let d = written("missing");
    fs::remove_file(d.join("chunks/select_picks.txt")).unwrap();
    assert!(drift(&d).contains("chunks/select_picks.txt: missing"));
    fs::write(d.join("chunks/stale.txt"), "x\n").unwrap();
    let m = drift(&d);
    assert!(m.contains("chunks/stale.txt: extra file"), "{m}");
    fs::create_dir(d.join("chunks/dir")).unwrap();
    assert!(drift(&d).contains("chunks/dir: extra"));
    fs::remove_dir_all(d.join("chunks")).unwrap();
    let m = drift(&d);
    assert_eq!(m.matches("missing or unreadable").count(), 9, "{m}");
    fs::remove_file(d.join("corpus.json")).unwrap();
    assert!(drift(&d).contains("corpus.json: missing"));
    let _ = fs::remove_dir_all(d);
}

#[test]
fn a_reordered_key_a_crlf_and_a_stale_profile_digest_fail_the_check() {
    let d = written("reorder");
    let text = ct().corpus_json.clone();
    // swap two adjacent keys of the canonical writer's output: valid JSON, wrong bytes
    let (a, b) = ("  \"budget\": 16777216,\n", "  \"chunk_size\": 65536,\n");
    assert!(text.contains(a) && text.contains(b));
    fs::write(d.join("corpus.json"), text.replace(a, "@@").replace(b, a).replace("@@", b)).unwrap();
    let m = drift(&d);
    assert!(m.contains("corpus.json: differs"), "{m}");
    fs::write(d.join("corpus.json"), text.replace('\n', "\r\n")).unwrap();
    assert!(drift(&d).contains("corpus.json: differs at line 1, column 2"));
    // stale spec/profile digest: regenerate from another profile and check against the old files
    fs::write(d.join("corpus.json"), &text).unwrap();
    let other = with_file(CT_COMPARE, |f| {
        generate(&GenInput {
            file: f,
            spec_path: "code/specs/ct-compare.warrant",
            spec_bytes: CT_COMPARE.as_bytes(),
            profile_bytes: format!("{PROFILE}slim_chunk eq_agrees 0\n").as_bytes(),
            binding_faults: None,
        })
        .unwrap()
    });
    let e = check_corpus(&d, &other).unwrap_err();
    assert_eq!(e.code, code::DRIFT);
    assert!(e.message.contains("corpus.json: differs"), "{}", e.message);
    let _ = fs::remove_dir_all(d);
}

#[test]
fn many_findings_are_capped_in_the_message() {
    let mut c = ct().clone();
    for i in 0..30 {
        c.chunk_files.push((format!("extra{i}.txt"), "x\n".into()));
    }
    let d = tmp("many");
    let e = check_corpus(&d, &c).unwrap_err();
    assert!(e.message.ends_with("and 21 more"), "{}", e.message);
    let _ = fs::remove_dir_all(d);
}

#[test]
fn first_difference_locates_bytes() {
    assert_eq!(first_difference(b"abc", b"abc"), None);
    assert_eq!(first_difference(b"ab\ncd", b"ab\ncX"), Some((4, 2, 2)));
    assert_eq!(first_difference(b"ab", b"abc"), Some((2, 1, 3)));
    assert_eq!(first_difference(b"abc", b"ab"), Some((2, 1, 3)));
    assert_eq!(first_difference(b"", b"x"), Some((0, 1, 1)));
}

// ---- the path guard ----------------------------------------------------------------------

#[test]
fn the_guard_accepts_only_the_corpus_files() {
    for ok in ["corpus.json", "cases.tfv", "chunks/eq_agrees.txt", "chunks/a.b.txt"] {
        assert!(guard(ok).is_ok(), "{ok}");
    }
    let long = format!("chunks/{}.txt", "a".repeat(130));
    for bad in [
        "", "../corpus.json", "chunks/../x.txt", "chunks/..txt", "/etc/passwd", "chunks//x.txt",
        "chunks/.hidden.txt", "chunks/a/b.txt", "chunks\\x.txt", "chunks/x.json", "chunks/x y.txt",
        "chunks/x\0.txt", "C:chunks/x.txt", "chunks/", "chunks", "corpus.json/", "Corpus.json",
        "other/x.txt", "x.txt", "chunks/é.txt", &long,
    ] {
        let e = guard(bad).unwrap_err();
        assert_eq!(e.code, code::IO, "{bad:?}");
    }
}

#[test]
fn write_refuses_every_traversal_and_writes_nothing_outside() {
    let root = tmp("trav");
    let out = root.join("out");
    for evil in ["../evil.txt", "..", "sub/../../evil.txt", "/tmp/wcg-evil.txt", ".x.txt", "a\\b.txt", ""] {
        let mut c = ct().clone();
        c.chunk_files[0].0 = evil.to_string();
        let e = write_corpus(&out, &c).unwrap_err();
        assert_eq!(e.code, code::IO, "{evil:?}");
        assert!(!out.exists(), "nothing is created before the paths are validated: {evil:?}");
        assert!(!root.join("evil.txt").exists() && !Path::new("/tmp/wcg-evil.txt").exists());
    }
    // a duplicated path is refused too
    let mut c = ct().clone();
    c.chunk_files[1].0 = c.chunk_files[0].0.clone();
    assert_eq!(write_corpus(&out, &c).unwrap_err().code, code::IO);
    let names: Vec<_> = fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert!(names.is_empty(), "{names:?}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn write_refuses_unowned_files_in_chunks_and_deletes_nothing() {
    let d = written("unowned");
    fs::write(d.join("chunks/mine.txt"), "keep me\n").unwrap();
    let e = write_corpus(&d, ct()).unwrap_err();
    assert_eq!(e.code, code::IO);
    assert!(e.message.contains("does not own") && e.message.contains("mine.txt"));
    assert_eq!(fs::read_to_string(d.join("chunks/mine.txt")).unwrap(), "keep me\n");
    let _ = fs::remove_dir_all(d);
}

#[cfg(unix)]
#[test]
fn write_and_check_refuse_symbolic_links() {
    use std::os::unix::fs::symlink;
    let root = tmp("links");
    let (out, target) = (root.join("out"), root.join("elsewhere"));
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(&out).unwrap();
    symlink(&target, out.join("chunks")).unwrap();
    assert_eq!(write_corpus(&out, ct()).unwrap_err().code, code::IO);
    assert_eq!(fs::read_dir(&target).unwrap().count(), 0, "nothing followed the link");
    fs::remove_file(out.join("chunks")).unwrap();
    // a linked target file
    fs::create_dir_all(out.join("chunks")).unwrap();
    let secret = root.join("secret");
    fs::write(&secret, "do not touch").unwrap();
    symlink(&secret, out.join("cases.tfv")).unwrap();
    let e = write_corpus(&out, ct()).unwrap_err();
    assert!(e.message.contains("symbolic link"), "{e}");
    assert_eq!(fs::read_to_string(&secret).unwrap(), "do not touch");
    assert!(drift(&out).contains("cases.tfv: is a symbolic link"));
    fs::remove_file(out.join("cases.tfv")).unwrap();
    // a link planted at the temporary name is unlinked, never written through
    symlink(&secret, out.join("chunks/.eq_agrees.txt.tmp")).unwrap();
    write_corpus(&out, ct()).unwrap();
    assert_eq!(fs::read_to_string(&secret).unwrap(), "do not touch");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_failed_write_leaves_no_temporary_file() {
    // an output path that is a regular file: create_dir_all fails
    let d = tmp("failed");
    let f = d.join("file");
    fs::write(&f, "x").unwrap();
    assert_eq!(write_corpus(&f, ct()).unwrap_err().code, code::IO);
    // a read-only chunks directory fails at the temporary file (unix, not root: skipped as root)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let ro = d.join("ro");
        fs::create_dir_all(ro.join("chunks")).unwrap();
        fs::set_permissions(ro.join("chunks"), fs::Permissions::from_mode(0o555)).unwrap();
        let r = write_corpus(&ro, ct());
        fs::set_permissions(ro.join("chunks"), fs::Permissions::from_mode(0o755)).unwrap();
        if let Err(e) = r {
            assert_eq!(e.code, code::IO);
            assert_eq!(fs::read_dir(ro.join("chunks")).unwrap().count(), 0);
        }
    }
    let _ = fs::remove_dir_all(d);
}

// ---- the command line --------------------------------------------------------------------

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn cli(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let code = run(&args, &mut o, &mut e);
    (code, String::from_utf8(o).unwrap(), String::from_utf8(e).unwrap())
}

fn ct_args(out: &str) -> Vec<String> {
    let p = root().join("code/specs/fixtures/ct-compare-warrant-v2/corpus.profile");
    let b = root().join("code/packages/rust/ct-compare/evidence.bindings");
    [
        "--warrant", "code/specs/ct-compare.warrant", "--profile", p.to_str().unwrap(),
        "--bindings", b.to_str().unwrap(), "--out", out,
    ]
    .map(String::from)
    .to_vec()
}

#[test]
fn the_binary_writes_checks_and_fails_on_drift() {
    let bin = env!("CARGO_BIN_EXE_warrant-corpus-gen");
    let d = tmp("bin");
    let out = d.join("fixture");
    let go = |extra: &[&str]| {
        let mut a = ct_args(out.to_str().unwrap());
        a.extend(extra.iter().map(|s| s.to_string()));
        Command::new(bin).current_dir(root()).args(a).output().unwrap()
    };
    let w = go(&[]);
    assert!(w.status.success(), "{}", String::from_utf8_lossy(&w.stderr));
    assert!(String::from_utf8_lossy(&w.stdout).starts_with("wrote 11 files"));
    assert_eq!(fs::read_to_string(out.join("corpus.json")).unwrap(), ct().corpus_json);
    let ok = go(&["--check"]);
    assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stderr));
    assert!(String::from_utf8_lossy(&ok.stdout).starts_with("ok:"));
    // sabotage: the gate must fail (the BUILD line does the same with sed)
    let mut b = fs::read(out.join("chunks/eq_agrees.txt")).unwrap();
    b[0] = b'9';
    fs::write(out.join("chunks/eq_agrees.txt"), b).unwrap();
    let bad = go(&["--check"]);
    assert_eq!(bad.status.code(), Some(1), "drift is exit 1");
    let msg = String::from_utf8_lossy(&bad.stderr);
    assert!(msg.contains("drift") && msg.contains("chunks/eq_agrees.txt: differs at line 1, column 1"), "{msg}");
    // a directory is not a regular file
    let dir = Command::new(bin)
        .current_dir(root())
        .args(["--warrant", "code/specs", "--profile", "x", "--out", "y"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&dir.stderr).contains("regular file"), "{}", String::from_utf8_lossy(&dir.stderr));
    assert_eq!(dir.status.code(), Some(2), "an unreadable input is a refusal");
    let usage = Command::new(bin).output().unwrap();
    assert_eq!(usage.status.code(), Some(2), "a usage error is exit 2, through the real binary");
    let _ = fs::remove_dir_all(d);
}

#[test]
fn cli_usage_and_refusals() {
    assert_eq!(cli(&[]).0, 2);
    let (c, _, e) = cli(&["--bogus"]);
    assert_eq!((c, e.contains("unknown argument")), (2, true));
    let (c, _, e) = cli(&["--warrant"]);
    assert_eq!((c, e.contains("needs a value")), (2, true));
    let (c, _, e) = cli(&["--warrant", "x"]);
    assert_eq!((c, e.contains("--profile is required")), (2, true));
    // the spec path is recorded verbatim, so it must be repo-relative
    for bad in ["/abs/x.warrant", "../x.warrant", "a/../x", "a\\b", "C:x", "./x", "a//b", ""] {
        let (c, _, e) = cli(&["--warrant", bad, "--profile", "p", "--out", "o"]);
        assert_eq!(c, 2, "{bad:?}");
        assert!(e.contains("repo-relative"), "{e}");
    }
    let (c, _, e) = cli(&["--warrant", "no/such.warrant", "--profile", "p", "--out", "o"]);
    assert_eq!((c, e.contains("cannot be read")), (2, true));
    // a directory is not a regular file
    let (c, _, e) = cli(&["--warrant", "code/specs", "--profile", "p", "--out", "o"]);
    assert_eq!(c, 2);
    assert!(e.contains("cannot be read") || e.contains("regular file"), "{e}");
}

#[test]
fn cli_write_check_and_refusals_on_a_small_document() {
    // enum_probe is a small synthetic document, named relative to the crate directory
    let bin = env!("CARGO_BIN_EXE_warrant-corpus-gen");
    let d = tmp("small");
    let out = d.join("o");
    let prof = d.join("p");
    fs::write(&prof, "chunk_size 1024\n").unwrap();
    let go = |extra: &[&str]| {
        let r = Command::new(bin)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args(["--warrant", "tests/fixtures/enum_probe.warrant", "--profile", prof.to_str().unwrap(), "--out", out.to_str().unwrap()])
            .args(extra)
            .output()
            .unwrap();
        (r.status.code().unwrap(), String::from_utf8_lossy(&r.stdout).into_owned(), String::from_utf8_lossy(&r.stderr).into_owned())
    };
    let (c, o, e) = go(&[]);
    assert_eq!(c, 0, "{e}");
    assert!(e.contains("--bindings not given"), "{e}");
    assert!(o.starts_with("wrote "));
    assert_eq!(go(&["--check"]).0, 0);
    // another profile: the digest in corpus.json is stale
    fs::write(&prof, "chunk_size 2048\n").unwrap();
    let (c, _, e) = go(&["--check"]);
    assert_eq!(c, 1, "drift");
    assert!(e.contains("drift") && e.contains("corpus.json: differs"), "{e}");
    // a profile the reader refuses
    fs::write(&prof, "chunk_size 3000\n").unwrap();
    let (c, _, e) = go(&[]);
    assert_eq!(c, 2, "a profile error is a refusal");
    assert!(e.contains("profile"), "{e}");
    let _ = fs::remove_dir_all(d);
}

#[test]
fn the_binary_refuses_drifted_bindings_and_documents_the_checker_refuses() {
    let bin = env!("CARGO_BIN_EXE_warrant-corpus-gen");
    let d = tmp("refuse");
    let bad = d.join("bindings");
    fs::write(&bad, "op eq export=ct_eq faults=invert\n").unwrap();
    let mut a = ct_args(d.join("o2").to_str().unwrap());
    let i = a.iter().position(|x| x == "--bindings").unwrap();
    a[i + 1] = bad.to_str().unwrap().to_string();
    let r = Command::new(bin).current_dir(root()).args(&a).output().unwrap();
    // the `drift` *code* of the bindings cross-check is still a refusal: nothing is compared
    assert_eq!(r.status.code(), Some(2));
    let e = String::from_utf8_lossy(&r.stderr);
    assert!(e.contains("drift") && e.contains("faults= lists"), "{e}");
    assert!(!d.join("o2").exists(), "nothing is written when the lists drift");
    // a document the checker refuses (cwd is the temp dir so the path is relative)
    fs::write(d.join("bad.warrant"), "warrant x\n  nonsense\n").unwrap();
    fs::write(d.join("p"), "").unwrap();
    let r = Command::new(bin)
        .current_dir(&d)
        .args(["--warrant", "bad.warrant", "--profile", "p", "--out", "o3"])
        .output()
        .unwrap();
    assert_eq!(r.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&r.stderr).contains("check"), "{}", String::from_utf8_lossy(&r.stderr));
    assert!(!d.join("o3").exists());
    let _ = fs::remove_dir_all(d);
}

#[test]
fn binding_faults_are_read_from_evidence_bindings() {
    let text = "# comment\nop eq export=ct_eq faults=invert,ignore-last-byte\nop plain export=p\nop sb export=s fresh faults=swap-choice proofs_not_live=x\nrevision 1\nop\n";
    assert_eq!(
        parse_binding_faults(text),
        vec![
            ("eq".to_string(), vec!["invert".to_string(), "ignore-last-byte".to_string()]),
            ("plain".to_string(), vec![]),
            ("sb".to_string(), vec!["swap-choice".to_string()]),
        ]
    );
    let real = fs::read_to_string(root().join("code/packages/rust/ct-compare/evidence.bindings")).unwrap();
    assert_eq!(parse_binding_faults(&real).len(), 4);
}

// ---- review fixes: hard links, FIFOs, oversize files, hostile names -------------------------

/// Run `f` on a thread and fail the test if it does not finish (a FIFO must never hang us).
#[cfg(unix)]
fn within<T: Send + 'static>(secs: u64, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(std::time::Duration::from_secs(secs))
        .expect("the operation hung (a FIFO was opened?)")
}

#[cfg(unix)]
fn mkfifo(p: &Path) -> bool {
    Command::new("mkfifo").arg(p).status().is_ok_and(|s| s.success())
}

#[cfg(unix)]
#[test]
fn a_hard_link_planted_at_the_temporary_name_never_clobbers_its_victim() {
    let root = tmp("hardlink");
    let out = root.join("out");
    fs::create_dir_all(out.join("chunks")).unwrap();
    let victim = root.join("victim");
    fs::write(&victim, "precious").unwrap();
    // the temp names of corpus.json and of a chunk file
    fs::hard_link(&victim, out.join(".corpus.json.tmp")).unwrap();
    fs::hard_link(&victim, out.join("chunks/.eq_agrees.txt.tmp")).unwrap();
    write_corpus(&out, ct()).unwrap();
    assert_eq!(fs::read_to_string(&victim).unwrap(), "precious");
    check_corpus(&out, ct()).unwrap();
    // O_EXCL on a fresh inode: the written file is not the victim's inode
    assert_eq!(fs::read_to_string(out.join("corpus.json")).unwrap(), ct().corpus_json);
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn a_fifo_at_the_temporary_name_or_at_a_corpus_file_never_hangs() {
    let root = tmp("fifo");
    let out = root.join("out");
    fs::create_dir_all(out.join("chunks")).unwrap();
    if !mkfifo(&out.join(".cases.tfv.tmp")) {
        let _ = fs::remove_dir_all(root);
        return; // no mkfifo on this machine
    }
    let (o2, c2) = (out.clone(), ct().clone());
    within(60, move || write_corpus(&o2, &c2)).unwrap();
    check_corpus(&out, ct()).unwrap();
    // a FIFO where corpus.json should be: --check reports it and does not open it
    fs::remove_file(out.join("corpus.json")).unwrap();
    assert!(mkfifo(&out.join("corpus.json")));
    let (o3, c3) = (out.clone(), ct().clone());
    let e = within(60, move || check_corpus(&o3, &c3)).unwrap_err();
    assert!(e.message.contains("corpus.json: is not a regular file"), "{e}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn an_oversize_file_is_a_finding_and_is_not_read_whole() {
    let d = written("oversize");
    let big = vec![b'x'; 6 * 1024 * 1024];
    fs::write(d.join("chunks/eq_agrees.txt"), &big).unwrap();
    let m = drift(&d);
    assert!(m.contains("chunks/eq_agrees.txt: differs at line 1, column 1"), "{m}");
    assert!(m.contains(&format!("found {}", big.len())), "{m}");
    let _ = fs::remove_dir_all(d);
}

#[test]
fn cli_inputs_are_capped_and_irregular_files_are_refused() {
    let bin = env!("CARGO_BIN_EXE_warrant-corpus-gen");
    let d = tmp("inputs");
    fs::write(d.join("big.warrant"), vec![b'x'; 5 * 1024 * 1024]).unwrap();
    fs::write(d.join("p"), "").unwrap();
    let run = |w: &str| {
        let r = Command::new(bin)
            .current_dir(&d)
            .args(["--warrant", w, "--profile", "p", "--out", "o"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&r.stderr).into_owned()
    };
    assert!(run("big.warrant").contains("size cap"));
    #[cfg(unix)]
    if mkfifo(&d.join("pipe.warrant")) {
        let dd = d.clone();
        let e = within(60, move || {
            let r = Command::new(bin)
                .current_dir(&dd)
                .args(["--warrant", "pipe.warrant", "--profile", "p", "--out", "o"])
                .output()
                .unwrap();
            String::from_utf8_lossy(&r.stderr).into_owned()
        });
        assert!(e.contains("not a regular file"), "{e}");
    }
    // without --bindings the tool says so
    let e = run("big.warrant");
    assert!(e.contains("--bindings not given"), "{e}");
    let _ = fs::remove_dir_all(d);
}

#[cfg(unix)]
#[test]
fn check_reports_a_symlinked_chunks_directory_and_escapes_hostile_names() {
    use std::os::unix::fs::symlink;
    let d = written("l1");
    // L1: chunks replaced by a link to a copy: write refuses, --check reports it
    let copy = d.join("copy");
    fs::rename(d.join("chunks"), &copy).unwrap();
    symlink(&copy, d.join("chunks")).unwrap();
    assert!(drift(&d).contains("chunks: is a symbolic link"));
    assert_eq!(write_corpus(&d, ct()).unwrap_err().code, code::IO);
    fs::remove_file(d.join("chunks")).unwrap();
    fs::rename(&copy, d.join("chunks")).unwrap();
    // L2: a hostile file name is escaped in both reports
    let evil = "x\n::error::pwned\u{1b}[31m.txt";
    fs::write(d.join("chunks").join(evil), "x").unwrap();
    let m = drift(&d);
    assert!(!m.contains('\n') && !m.contains('\u{1b}'), "{m:?}");
    assert!(m.contains("x\\n::error::pwned\\u{1b}[31m.txt"), "{m}");
    let e = write_corpus(&d, ct()).unwrap_err();
    assert!(!e.message.contains('\n') && !e.message.contains('\u{1b}'), "{:?}", e.message);
    // a symlinked --out itself is the caller's choice: accepted in both modes
    let alias = d.parent().unwrap().join(format!("{}-alias", d.file_name().unwrap().to_string_lossy()));
    fs::remove_file(d.join("chunks").join(evil)).unwrap();
    symlink(&d, &alias).unwrap();
    write_corpus(&alias, ct()).unwrap();
    check_corpus(&alias, ct()).unwrap();
    let _ = fs::remove_file(alias);
    let _ = fs::remove_dir_all(d);
}

#[test]
fn the_name_limit_is_the_spec_maximum() {
    // two 64-character claim names joined by `/` become `<64>.<64>.txt`
    let ok = format!("chunks/{}.{}.txt", "a".repeat(64), "b".repeat(64));
    assert_eq!(ok.len() - "chunks/".len(), 133);
    assert!(guard(&ok).is_ok());
    let too_long = format!("chunks/{}.{}x.txt", "a".repeat(64), "b".repeat(64));
    assert!(guard(&too_long).is_err());
}

#[test]
fn a_temporary_name_that_cannot_be_replaced_fails_the_write_without_damage() {
    // a non-empty directory sits at the reserved temp name: it cannot be unlinked as a file, so
    // the exclusive create fails and the write is refused (nothing is written through it)
    let d = tmp("tmpdir");
    fs::create_dir_all(d.join(".cases.tfv.tmp/inner")).unwrap();
    let e = write_corpus(&d, ct()).unwrap_err();
    assert_eq!(e.code, code::IO);
    assert!(e.message.contains("temporary file cannot be created"), "{e}");
    assert!(d.join(".cases.tfv.tmp/inner").exists());
    assert!(!d.join("corpus.json").exists(), "corpus.json, the commit point, is written last");
    let _ = fs::remove_dir_all(d);
}

/// The exit-status table of the binary (the oracle's convention: 1 = differs, 2 = refused or
/// error), one assertion per category.  The real binary is run with an explicit working
/// directory, so no test changes the process-wide one.
#[test]
fn exit_status_table_success_drift_refusal_usage() {
    let bin = env!("CARGO_BIN_EXE_warrant-corpus-gen");
    let d = tmp("table");
    let out = d.join("o");
    let prof = d.join("p");
    fs::write(&prof, "chunk_size 1024\n").unwrap();
    let run_in = |cwd: &Path, warrant: &str, out: &Path, extra: &[&str]| -> (i32, String) {
        let r = Command::new(bin)
            .current_dir(cwd)
            .args(["--warrant", warrant, "--profile", prof.to_str().unwrap(), "--out", out.to_str().unwrap()])
            .args(extra)
            .output()
            .unwrap();
        (r.status.code().unwrap(), String::from_utf8_lossy(&r.stderr).into_owned())
    };
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ok = "tests/fixtures/enum_probe.warrant";
    // success: write, then a clean --check
    assert_eq!(run_in(here, ok, &out, &[]).0, 0);
    assert_eq!(run_in(here, ok, &out, &["--check"]).0, 0);
    // drift: a differing byte, then a missing file
    let corpus = out.join("corpus.json");
    let good = fs::read(&corpus).unwrap();
    let mut bad = good.clone();
    bad[0] = b'X';
    fs::write(&corpus, &bad).unwrap();
    assert_eq!(run_in(here, ok, &out, &["--check"]).0, 1, "a differing byte");
    fs::write(&corpus, &good).unwrap();
    fs::remove_file(out.join("cases.tfv")).unwrap();
    assert_eq!(run_in(here, ok, &out, &["--check"]).0, 1, "a missing file is listed as drift");
    // refusal: a bound the document breaks (arithmetic.warrant: a claim with more than 8 binders, which
    // surfaces now that `ir.data` is generated, S11c-2), write and --check
    let fixed = "code/specs/arithmetic.warrant";
    let o9 = d.join("o9");
    for extra in [&[][..], &["--check"][..]] {
        let (c, e) = run_in(&root(), fixed, &o9, extra);
        assert_eq!(c, 2, "a refusal {extra:?}: {e}");
        assert!(e.contains("limit"), "{e}");
    }
    assert!(!o9.exists(), "nothing is written for a refusal");
    // refusal: a profile error, in write and in --check mode (a refusal, never a drift)
    fs::write(&prof, "chunk_size 3000\n").unwrap();
    assert_eq!(run_in(here, ok, &out, &[]).0, 2);
    assert_eq!(run_in(here, ok, &out, &["--check"]).0, 2);
    // refusal: an I/O error writing the output (--out is a regular file, not a directory)
    fs::write(&prof, "chunk_size 1024\n").unwrap();
    let blocked = d.join("blocked");
    fs::write(&blocked, "x").unwrap();
    let (c, e) = run_in(here, ok, &blocked, &[]);
    assert_eq!(c, 2, "{e}");
    // usage
    let r = Command::new(bin).arg("--bogus").output().unwrap();
    assert_eq!(r.status.code(), Some(2));
    let _ = fs::remove_dir_all(d);
}
