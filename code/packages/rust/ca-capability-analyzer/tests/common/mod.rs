//! Shared helpers for the integration tests: temp packages and manifests.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A temp directory removed on drop.
pub struct Pkg {
    pub dir: PathBuf,
}

impl Drop for Pkg {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.dir).ok();
    }
}

impl Pkg {
    /// Create an empty package directory.
    pub fn empty(tag: &str) -> Pkg {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("ca_hard_{tag}_{}_{id}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        Pkg { dir }
    }

    /// Create a package from `(relative path, contents)` pairs.
    pub fn with(tag: &str, files: &[(&str, &str)]) -> Pkg {
        let p = Pkg::empty(tag);
        for (rel, content) in files {
            p.write(rel, content);
        }
        p
    }

    pub fn write(&self, rel: &str, content: &str) {
        let path = self.dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.dir.join(rel)).unwrap()
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

/// A valid pure manifest for `rust/<name>`.
pub fn pure_manifest(name: &str) -> String {
    format!(
        r#"{{"version": 1, "package": "rust/{name}", "capabilities": [], "justification": "Pure test package for hardening."}}"#
    )
}

/// A manifest declaring the given `category:action` pairs (target `*`) and exceptions.
pub fn manifest(name: &str, caps: &[(&str, &str)], exceptions: &[&str]) -> String {
    let caps_json: Vec<String> = caps
        .iter()
        .map(|(c, a)| {
            format!(
                r#"{{"category": "{c}", "action": "{a}", "target": "*", "justification": "Test needs {c}:{a} access."}}"#
            )
        })
        .collect();
    let ex_json: Vec<String> = exceptions
        .iter()
        .map(|c| {
            format!(
                r#"{{"construct": "{c}", "language": "rust", "justification": "Audited test exception for {c}."}}"#
            )
        })
        .collect();
    format!(
        r#"{{"version": 1, "package": "rust/{name}", "capabilities": [{}], "justification": "Test package manifest.", "banned_construct_exceptions": [{}]}}"#,
        caps_json.join(","),
        ex_json.join(",")
    )
}

/// A package with `src/lib.rs`, a manifest, and the generated capabilities module.
pub fn package(tag: &str, lib_rs: &str, manifest_json: &str) -> Pkg {
    let p = Pkg::with(
        tag,
        &[("src/lib.rs", lib_rs), ("required_capabilities.json", manifest_json)],
    );
    coding_adventures_ca_capability_analyzer::write_module_to_disk(p.dir()).unwrap();
    p
}
