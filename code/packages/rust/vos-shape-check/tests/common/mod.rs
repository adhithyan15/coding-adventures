//! Shared test helpers: load a fixture crate, build in-memory crates, mutate
//! text.  Fixture manifests are stored as `Cargo.toml.fixture` so that no
//! nested `Cargo.toml` exists under this package (repository tooling treats
//! every `Cargo.toml` as a Cargo root); the helpers rename on the way in.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use vos_shape_check::{check_crate, CrateInput, Policy, Report, SourceFile};

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn walk(dir: &Path, rel: &str, out: &mut Vec<SourceFile>) {
    let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap()).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        let child = format!("{rel}/{name}");
        if path.is_dir() {
            walk(&path, &child, out);
        } else if name.ends_with(".rs") {
            out.push(SourceFile {
                path: child,
                text: fs::read_to_string(&path).unwrap(),
            });
        }
    }
}

/// Load `tests/fixtures/<name>` as an in-memory crate.
pub fn load(name: &str) -> CrateInput {
    let dir = fixtures_dir().join(name);
    let mut root_entries: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .replace("Cargo.toml.fixture", "Cargo.toml")
        })
        .collect();
    root_entries.sort();
    let cargo_toml = fs::read_to_string(dir.join("Cargo.toml.fixture")).ok();
    let vos_toml = fs::read_to_string(dir.join("vos.toml")).ok();
    let mut sources = Vec::new();
    if dir.join("src").is_dir() {
        walk(&dir.join("src"), "src", &mut sources);
    }
    CrateInput {
        cargo_toml,
        vos_toml,
        root_entries,
        sources,
    }
}

/// A temporary crate tree that deletes itself when dropped (also on panic),
/// restoring permissions first so a `chmod 000` test cannot strand it.
pub struct Tmp(PathBuf);

impl std::ops::Deref for Tmp {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

fn make_writable(p: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(m) = fs::symlink_metadata(p) {
            if !m.file_type().is_symlink() {
                let _ = fs::set_permissions(p, fs::Permissions::from_mode(0o755));
            }
            if m.is_dir() {
                if let Ok(rd) = fs::read_dir(p) {
                    for e in rd.flatten() {
                        make_writable(&e.path());
                    }
                }
            }
        }
    }
    #[cfg(not(unix))]
    let _ = p;
}

impl Drop for Tmp {
    fn drop(&mut self) {
        make_writable(&self.0);
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Copy a fixture to a fresh temp directory laid out as a real crate.
pub fn materialize(name: &str, tag: &str) -> Tmp {
    let src = fixtures_dir().join(name);
    let dst = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("vos-shape-check-{}-{tag}", std::process::id()));
    make_writable(&dst);
    let _ = fs::remove_dir_all(&dst);
    copy_dir(&src, &dst);
    Tmp(dst)
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let name = e.file_name().to_string_lossy().into_owned();
        let to = dst.join(if name == "Cargo.toml.fixture" {
            "Cargo.toml"
        } else {
            &name
        });
        if e.path().is_dir() {
            copy_dir(&e.path(), &to);
        } else {
            fs::copy(e.path(), &to).unwrap();
        }
    }
}

/// A one-file crate with the given `src/lib.rs`, a clean manifest and no vos.toml.
pub fn lib(src: &str) -> CrateInput {
    CrateInput {
        cargo_toml: Some(MANIFEST.to_string()),
        vos_toml: None,
        root_entries: vec!["Cargo.toml".into(), "src".into()],
        sources: vec![SourceFile {
            path: "src/lib.rs".into(),
            text: src.to_string(),
        }],
    }
}

pub const MANIFEST: &str =
    "[package]\nname = \"vos-demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
pub const HEADER: &str = "#![no_std]\n#![forbid(unsafe_code)]\n";

/// `lib` with the standard `no_std` / `forbid(unsafe_code)` header prepended.
pub fn body(src: &str) -> CrateInput {
    lib(&format!("{HEADER}{src}"))
}

pub fn run(input: &CrateInput) -> Report {
    check_crate(input, &Policy::default())
}

/// The rule ids of a report, in report order, as a vector of strings.
pub fn rules(r: &Report) -> Vec<&'static str> {
    r.findings.iter().map(|f| f.rule.id()).collect()
}

/// Apply a text replacement to `src/lib.rs` of a loaded fixture; panics if the
/// pattern is absent (so a stale mutation cannot pass silently).
pub fn mutate(mut input: CrateInput, from: &str, to: &str) -> CrateInput {
    let f = input
        .sources
        .iter_mut()
        .find(|s| s.path == "src/lib.rs")
        .unwrap();
    assert!(f.text.contains(from), "mutation pattern not found: {from}");
    f.text = f.text.replacen(from, to, 1);
    input
}

impl AsRef<Path> for Tmp {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Batched checks
//
// Building the grammar lexer costs about 0.7 s in a debug build (RS01 section
// 4.1: 175 ms in release) and cannot be reused for a second source, so a test
// that checks 30 snippets one by one would cost 20 s.  `check_crates` lexes many
// crates with ONE lexer.  A test that starts a `Batch` queues its `fires`,
// `clean` and `silent` helper calls and the batch runs them together when the
// test function ends (in `Drop`, unless the test is already panicking).
// ---------------------------------------------------------------------------

use std::cell::RefCell;
use vos_shape_check::check_crates;

pub enum Expect {
    Fires(String),
    Clean,
    Silent(String),
    Msg(String, String),
}

struct Queued {
    input: CrateInput,
    expect: Expect,
    label: String,
}

thread_local! {
    static QUEUE: RefCell<Option<Vec<Queued>>> = const { RefCell::new(None) };
}

pub struct Batch;

impl Batch {
    pub fn begin() -> Batch {
        QUEUE.with(|q| *q.borrow_mut() = Some(Vec::new()));
        Batch
    }
}

fn verify(q: &Queued, r: &Report) -> Option<String> {
    let ids = rules(r);
    match &q.expect {
        Expect::Fires(id) if !ids.contains(&id.as_str()) => Some(format!("expected {id}")),
        Expect::Clean if !r.findings.is_empty() => Some("expected clean".to_string()),
        Expect::Silent(id) if ids.contains(&id.as_str()) => Some(format!("did not expect {id}")),
        Expect::Msg(id, needle)
            if !r
                .findings
                .iter()
                .any(|f| f.rule.id() == id && f.message.contains(needle.as_str())) =>
        {
            Some(format!("expected {id} mentioning {needle:?}"))
        }
        _ => None,
    }
}

/// Queue a check (run when the test's `Batch` is dropped) or run it now.
pub fn check(input: CrateInput, expect: Expect, label: &str) {
    let q = Queued {
        input,
        expect,
        label: label.to_string(),
    };
    let now = QUEUE.with(|cell| match cell.borrow_mut().as_mut() {
        Some(v) => {
            v.push(q);
            None
        }
        None => Some(q),
    });
    if let Some(q) = now {
        let r = run(&q.input);
        if let Some(why) = verify(&q, &r) {
            panic!("{why}\n{}\n{}", q.label, r.render());
        }
    }
}

impl Drop for Batch {
    fn drop(&mut self) {
        let items = QUEUE.with(|q| q.borrow_mut().take()).unwrap_or_default();
        if std::thread::panicking() || items.is_empty() {
            return;
        }
        let inputs: Vec<CrateInput> = items.iter().map(|i| i.input.clone()).collect();
        let reports = check_crates(&inputs, &Policy::default());
        let mut failures = Vec::new();
        for (q, r) in items.iter().zip(&reports) {
            if let Some(why) = verify(q, r) {
                failures.push(format!("{why}\n  input: {}\n{}", q.label, r.render()));
            }
        }
        if !failures.is_empty() {
            panic!(
                "{} of {} batched checks failed:\n{}",
                failures.len(),
                items.len(),
                failures.join("\n---\n")
            );
        }
    }
}

/// Check many crates with one grammar lexer (see the module notes above).
pub fn run_all(inputs: &[CrateInput]) -> Vec<Report> {
    check_crates(inputs, &Policy::default())
}
