//! A throwaway repository for the tests.
//!
//! Each test builds `<tmp>/code/packages/rust/<package>` trees and `<tmp>/code/specs`, with
//! the **real** `vos-sched-contract` package copied in and a fixture implementation (or
//! several) from `tests/fixtures/impls`.  Nothing is written inside the real repository.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use vos_link::Outcome;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct Repo {
    pub root: PathBuf,
}

fn here() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}

impl Repo {
    /// An empty repository with the real scheduler contract package.
    pub fn new(label: &str) -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let base =
            std::env::temp_dir().join(format!("vos-link-test-{}-{n}-{label}", std::process::id()));
        // `create_dir` fails if the name exists: a test never reuses (or deletes) a
        // directory it did not make.  The name carries the pid, a counter and the clock.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let base = base.with_file_name(format!(
            "{}-{nanos}",
            base.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir(&base).expect("a fresh unique test directory");
        let root = base.canonicalize().unwrap();
        fs::create_dir_all(root.join("code/specs")).unwrap();
        let real = here().join("../../../packages/rust/vos-sched-contract");
        let dest = root.join("code/packages/rust/vos-sched-contract");
        fs::create_dir_all(dest.join("warrant")).unwrap();
        for f in ["vos.toml", "contract.vos"] {
            fs::copy(real.join(f), dest.join(f)).unwrap();
        }
        for entry in fs::read_dir(real.join("warrant")).unwrap() {
            let entry = entry.unwrap();
            fs::copy(entry.path(), dest.join("warrant").join(entry.file_name())).unwrap();
        }
        Repo { root }
    }

    /// A repository with one fixture implementation.
    pub fn with(label: &str, fixture: &str) -> Repo {
        let r = Repo::new(label);
        r.add_fixture(fixture);
        r
    }

    pub fn add_fixture(&self, name: &str) {
        copy_dir(
            &here().join("tests/fixtures/impls").join(name),
            &self.root.join("code/packages/rust").join(name),
        );
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    pub fn pkg(&self, name: &str) -> PathBuf {
        self.root.join("code/packages/rust").join(name)
    }

    pub fn write(&self, rel: &str, bytes: impl AsRef<[u8]>) {
        let p = self.path(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }

    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path(rel)).unwrap()
    }

    pub fn doc(&self, name: &str) -> String {
        self.read(&format!("code/specs/{name}.warrant"))
    }

    /// Replace `from` by `to` once in a file, asserting that it was there.
    pub fn edit(&self, rel: &str, from: &str, to: &str) {
        let s = self.read(rel);
        assert!(s.contains(from), "{rel} has no {from:?}");
        self.write(rel, s.replacen(from, to, 1));
    }

    pub fn gen(&self, name: &str) -> Outcome {
        vos_link::gen_doc(&self.pkg(name), None).expect("a verdict")
    }

    pub fn check(&self, name: &str) -> Outcome {
        vos_link::check_doc(&self.pkg(name), None).expect("a verdict")
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn codes(o: &Outcome) -> Vec<&'static str> {
    o.findings.iter().map(|f| f.code).collect()
}

pub fn show(o: &Outcome) -> String {
    o.findings
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The tier names carried, for assertions.
pub fn carried(o: &Outcome) -> Vec<&'static str> {
    o.carries.iter().map(|t| t.name()).collect()
}
