//! # `meta`: what the results file says about the machine and the tree
//!
//! Every value here is *self-reported*: it is read from `git`, `rustc` and `uname` by the same
//! tool that writes the file, so nothing technical stops an edited tool from writing another
//! value.  That is why regeneration by a named person is a process control (VOS30 section 9.6).

use crate::results::{Meta, TREE_NAMES};
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

/// The repository path of each tree [`TREE_NAMES`] names, in the same order: the probe and its
/// four solver crates (VOS30 section 9.12).
pub const TREE_PATHS: [&str; 5] = [
    "code/programs/rust/vos-capacity-probe",
    "code/packages/rust/bit-blaster",
    "code/packages/rust/cdcl-sat",
    "code/packages/rust/drat-check",
    "code/packages/rust/riscv-simulator",
];

/// The content-addressed tree ids (`git rev-parse HEAD:<path>`) of the probe and its four solver
/// crates; empty if git cannot say for any of them (a file without `trees` is then recorded, and
/// `check --full` refuses it).
pub fn head_trees() -> Vec<(String, String)> {
    let root = repo_root();
    let mut out = Vec::new();
    for (name, path) in TREE_NAMES.iter().zip(TREE_PATHS) {
        match run("git", &["rev-parse", &format!("HEAD:{path}")], Some(&root)) {
            Some(id) if !id.is_empty() => out.push((name.to_string(), id)),
            _ => return Vec::new(),
        }
    }
    out
}

/// The tree ids of the WORKING tree: like [`head_trees`], but a path with an uncommitted change
/// (or an untracked file) gets `+uncommitted` appended, so it equals no recorded id.
pub fn working_trees() -> Vec<(String, String)> {
    let root = repo_root();
    let mut out = head_trees();
    for ((_, id), path) in out.iter_mut().zip(TREE_PATHS) {
        let dirty = run("git", &["status", "--porcelain", "--", path], Some(&root))
            .is_none_or(|s| !s.is_empty());
        if dirty {
            id.push_str("+uncommitted");
        }
    }
    out
}

/// The `stale:` lines for a recorded `trees` object against the working tree's: one per name
/// whose recorded id differs.  A recorded object that is empty yields none (nothing is claimed),
/// and an empty `current` (git cannot say) yields one line.
pub fn stale_lines(
    what: &str,
    recorded: &[(String, String)],
    current: &[(String, String)],
) -> Vec<String> {
    if recorded.is_empty() {
        return Vec::new();
    }
    if current.is_empty() {
        return vec![format!(
            "stale: {what}: the working tree's ids cannot be read (is this a git checkout?)"
        )];
    }
    recorded
        .iter()
        .filter_map(|(k, rec)| {
            let now = current
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, x)| x.as_str());
            (now != Some(rec.as_str())).then(|| {
                format!(
                    "stale: {what}: {k} was recorded at tree {rec} but the working tree is {}",
                    now.unwrap_or("absent")
                )
            })
        })
        .collect()
}

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
        merges: Vec::new(),
        recording: None,
        trees: head_trees(),
    }
}

/// Is `rev` (a 40-hex commit id) HEAD or an ancestor of it?  `Err` if `git` cannot say.
pub fn is_ancestor_of_head(rev: &str) -> Result<bool, String> {
    if rev.len() != 40 || !rev.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("{rev:?} is not a 40-hex revision"));
    }
    let st = Command::new("git")
        .args(["merge-base", "--is-ancestor", rev, "HEAD"])
        .current_dir(repo_root())
        .status()
        .map_err(|e| format!("cannot run git: {e}"))?;
    match st.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!("git cannot place revision {rev}")),
    }
}

/// The current UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
pub fn utc_now() -> String {
    run("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], None).unwrap_or_else(|| "unknown".into())
}
