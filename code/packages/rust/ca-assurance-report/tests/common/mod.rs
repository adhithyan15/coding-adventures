//! Test support: a throw-away copy of the `rust/ct-compare` evidence layout so
//! falsification tests can corrupt manifests, certificates, the spec or the
//! source without touching tracked files.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

pub struct Sandbox {
    pub root: PathBuf,
    /// `<root>/code/packages/rust/ct-compare`, the package directory.
    pub pkg: PathBuf,
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

impl Sandbox {
    pub fn new(tag: &str) -> Sandbox {
        Sandbox::for_package(tag, "ct-compare")
    }

    /// The same throw-away layout for any adopter (`lzw`, `arithmetic`, ...).
    pub fn for_package(tag: &str, name: &str) -> Sandbox {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let real = manifest.join("..").join(name);
        let root = std::env::temp_dir().join(format!("ver04h-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let pkg = root.join("code/packages/rust").join(name);
        copy_dir(&real.join("evidence"), &pkg.join("evidence"));
        copy_dir(&real.join("src"), &pkg.join("src"));
        fs::copy(real.join("effect_certificate.json"), pkg.join("effect_certificate.json")).unwrap();
        // The reviewed bindings file: it is where a package declares an
        // operation's proofs `not-live`.
        fs::copy(real.join("evidence.bindings"), pkg.join("evidence.bindings")).unwrap();
        // `--generate` names the report after the package directory and reads
        // its version from Cargo.toml.
        fs::copy(real.join("Cargo.toml"), pkg.join("Cargo.toml")).unwrap();
        // The audit profile is the package's own data file (VER04h section 9 item 22).
        fs::copy(real.join("assurance-profile.json"), pkg.join("assurance-profile.json")).unwrap();
        fs::create_dir_all(root.join("code/specs")).unwrap();
        fs::copy(
            manifest.join("../../../specs").join(format!("{name}.warrant")),
            root.join("code/specs").join(format!("{name}.warrant")),
        )
        .unwrap();
        // The VER13 catalogue is found above the package, like the spec.
        copy_dir(&manifest.join("../../../assumptions"), &root.join("code/assumptions"));
        Sandbox { root, pkg }
    }

    /// The `source_sha256` of this sandbox's `src/**/*.rs` (shared framing).
    pub fn src_digest(&self) -> String {
        let framed = coding_adventures_test_bootstrap::manifest::source_tree_framing(&self.pkg.join("src")).unwrap();
        coding_adventures_sha256::sha256_hex(&framed)
    }

    /// Path of a file under the sandbox's `code/assumptions/`.
    pub fn assumptions(&self, rel: &str) -> PathBuf {
        self.root.join("code/assumptions").join(rel)
    }

    pub fn spec(&self) -> PathBuf {
        self.root.join("code/specs/ct-compare.warrant")
    }

    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.pkg.join(rel)).unwrap()
    }

    pub fn write(&self, rel: &str, text: &str) {
        fs::write(self.pkg.join(rel), text).unwrap();
    }

    /// Replace `from` with `to` in a package file; the text must be present.
    pub fn edit(&self, rel: &str, from: &str, to: &str) {
        let s = self.read(rel);
        assert!(s.contains(from), "{rel} must contain {from:?}");
        self.write(rel, &s.replacen(from, to, 1));
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
