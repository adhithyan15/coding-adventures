//! # `meta`: what the results file says about the machine and the tree
//!
//! Every value here is *self-reported*: it is read from `git`, `rustc` and `uname` by the same
//! tool that writes the file, so nothing technical stops an edited tool from writing another
//! value.  That is why regeneration by a named person is a process control (VOS30 section 9.6).

use crate::results::Meta;
use coding_adventures_sha256::sha256_hex;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository root, from the location of this crate's manifest.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// The default results file.
pub fn default_results() -> PathBuf {
    repo_root().join("code/specs/data/vos30-capacity-results.json")
}

/// The default spec.
pub fn default_spec() -> PathBuf {
    repo_root().join("code/specs/VOS30-proof-route-and-capacity.md")
}

fn run(cmd: &str, args: &[&str], dir: Option<&Path>) -> Option<String> {
    let mut c = Command::new(cmd);
    c.args(args);
    if let Some(d) = dir {
        c.current_dir(d);
    }
    let out = c.output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The crates whose revisions the file records (section 9.6).
pub const REVISION_CRATES: [&str; 4] = ["bit-blaster", "cdcl-sat", "drat-check", "riscv-simulator"];

/// Gather the run metadata.
pub fn gather(exe: &Path) -> Meta {
    let root = repo_root();
    let host = run("rustc", &["-vV"], None).unwrap_or_default();
    let triple = host
        .lines()
        .find_map(|l| l.strip_prefix("host: "))
        .unwrap_or("unknown")
        .to_string();
    let mut revisions = Vec::new();
    for c in REVISION_CRATES {
        let path = format!("code/packages/rust/{c}");
        revisions.push((
            c.to_string(),
            run(
                "git",
                &["log", "-1", "--format=%H", "--", &path],
                Some(&root),
            )
            .unwrap_or_else(|| "unknown".into()),
        ));
    }
    revisions.push((
        "repo".to_string(),
        run("git", &["rev-parse", "HEAD"], Some(&root)).unwrap_or_else(|| "unknown".into()),
    ));
    let dirty = run("git", &["status", "--porcelain"], Some(&root))
        .map(|s| !s.is_empty())
        .unwrap_or(true);
    let binary = std::fs::read(exe)
        .map(|b| sha256_hex(&b))
        .unwrap_or_else(|_| "unavailable".into());
    Meta {
        name: "vos-capacity-probe".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        binary_sha256: binary,
        dirty_tree: dirty,
        triple,
        rustc: run("rustc", &["--version"], None).unwrap_or_default(),
        os: run("uname", &["-sr"], None).unwrap_or_default(),
        revisions,
        regeneration: None,
    }
}
