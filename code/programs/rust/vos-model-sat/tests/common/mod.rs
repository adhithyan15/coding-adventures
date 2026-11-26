//! Shared fixtures: the real `sched` contract crate, and the evaluator's answers about it,
//! computed once per test binary (the evaluator is an interpreter: about 25 s in release).

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use vos_model_sat::crosscheck::{gather, Evidence};
use vos_model_sat::oracle::{load, ContractFiles};

/// `code/packages/rust/vos-sched-contract`, from this crate's manifest directory
/// (`code/programs/rust/vos-model-sat`).  No `..` component: the loader refuses one.
pub fn contract_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let code = manifest.ancestors().nth(3).expect("code/ directory");
    code.join("packages")
        .join("rust")
        .join("vos-sched-contract")
}

pub fn files() -> &'static ContractFiles {
    static F: OnceLock<ContractFiles> = OnceLock::new();
    F.get_or_init(|| load(&contract_dir(), None).expect("the real contract crate loads"))
}

pub fn evidence() -> &'static Evidence {
    static E: OnceLock<Evidence> = OnceLock::new();
    E.get_or_init(|| gather(files()).expect("the evaluator runs on the real contract"))
}

/// The `failed` column of VOS31 6.1, in `lemmas.part` order.
pub const EXPECTED_MUTANT_FAILED: [u64; 10] =
    [65536, 63488, 63488, 448, 512, 1024, 1016, 128, 255, 1];

/// The system temp directory with every link resolved, spelled as an ordinary path.
///
/// The loader (`vos-link`) refuses any path whose canonical form differs from its spelling,
/// and that is a security property we do not weaken.  CI temp dirs are such paths: macOS
/// `/var/folders/..` is a symlink to `/private/var/folders/..`, and Windows runners give
/// `C:\Users\RUNNER~1\..`, an 8.3 short name for `C:\Users\runneradmin\..`.  So scratch
/// repositories must be built under the resolved directory.
pub fn real_temp_dir() -> PathBuf {
    let t = std::env::temp_dir();
    let c = std::fs::canonicalize(&t).unwrap_or(t);
    strip_verbatim(c)
}

/// `\\?\C:\x` becomes `C:\x`: drive-letter verbatim paths only; UNC `\\?\UNC\..` is left alone.
fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy().into_owned();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        let b = rest.as_bytes();
        if b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\' {
            return PathBuf::from(rest);
        }
    }
    p
}

/// Copy the real contract crate into a throwaway repository root and return
/// `(root, crate_dir)`.  The loader wants `<root>/code/packages/rust/<crate>`.
pub fn temp_contract(tag: &str) -> (PathBuf, PathBuf) {
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            let name = e.file_name();
            if name == "target" {
                continue;
            }
            let (src, dst) = (e.path(), to.join(&name));
            if e.file_type().unwrap().is_dir() {
                copy(&src, &dst);
            } else {
                std::fs::copy(&src, &dst).unwrap();
            }
        }
    }
    let root = real_temp_dir().join(format!("vos-model-sat-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let krate = root
        .join("code")
        .join("packages")
        .join("rust")
        .join("vos-sched-contract");
    copy(&contract_dir(), &krate);
    (root, krate)
}
