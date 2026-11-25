//! # `meta`: what the results file says about the machine and the tree
//!
//! Every value here is *self-reported*: it is read from `git`, `rustc` and `uname` by the same
//! tool that writes the file, so nothing technical stops an edited tool from writing another
//! value.  That is why regeneration by a named person is a process control (VOS30 section 9.6).

use crate::results::{Meta, LEGACY_TREE_NAMES, TREE_NAMES};
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

/// The inputs whose git ids the results file records: `(name, repository path)`, in file order.
/// Each id comes from `git rev-parse <rev>:<path>`, which is a tree id for a directory and a blob
/// id for a file; both are hashes of content, so equal ids mean equal bytes.
///
/// # Why entries are inputs, not crates (VOS-D139, VOS-D140)
///
/// This list used to hold the five whole crate directories (the probe and its four solver
/// crates).  Those directories also hold `tests/`, `README.md`, `CHANGELOG.md` and `BUILD`, none
/// of which can change a recorded value.  Recording them made two things go wrong: a test inside
/// the probe asserted the "nothing recorded yet" state and so failed once a recording was
/// committed, and fixing it changed the probe's tree and staled the recording (D139); and after a
/// merge any README, CHANGELOG or test edit in those crates would stale the recording (D140).
///
/// So each entry is something that is compiled into, or read by, the binary that measures:
///
/// * `src/` of each of the five crates: the code that computes every recorded value.
/// * `Cargo.toml` of each crate: features, dependencies and their paths.
/// * the probe's `Cargo.lock`: the probe is its own cargo root, so its lockfile fixes every
///   dependency version of the solver crates it builds.  (The solver crates have no lockfile of
///   their own; they build through the probe's.)
/// * the four files the probe's `src/enumx.rs` pulls in at compile time with `include!` and
///   `include_bytes!`: the two generated warrant harnesses and the two `.warrant` documents.
/// * `src/` and `Cargo.toml` of EVERY path package in the probe's dependency closure (the E1
///   path: `vos-sched-fp-bitmap` with its `ghost` feature, `vos-sched-contract`,
///   `vos-probe-ready`, `warrant-grammar`; and `cpu-simulator`, `riscv-encoder`, `bounded-json`,
///   `sha256`, `test-bootstrap`, `zeroize`, `arithmetic`, `logic-gates`, `transistors`,
///   `grammar-tools`).  `Cargo.lock` pins registry crates but not the content of a path
///   dependency, so each path crate must be recorded itself.  The set is read from the probe's
///   `Cargo.lock` (a package with no `source`); `tests/tree_entries.rs` recomputes it and fails
///   when a path dependency is added without its entries.  Dev-dependency-only crates are
///   included too: over-coverage is safe, under-coverage is not.
/// * the two `#[path]` fixtures of `test-bootstrap` (its bins, not linked into the probe; included
///   for safety) and the repository-root `.cargo/config.toml` (rustflags or a linker there would
///   change the build; it is the only `.cargo/config*` on an ancestor path of the probe).
///
/// Left out, on purpose: `tests/` and `benches/` (never linked into the `run` binary; they assert
/// on values, they do not produce them), `README.md`, `CHANGELOG.md`, `BUILD`, `BUILD_windows`,
/// `required_capabilities.json` and `vos.toml` (documentation and build-tool metadata).  None of
/// the path crates has a `build.rs`; one that gains one must add it here.  What remains
/// uncovered: registry crates (pinned by the probe's `Cargo.lock`) and the Rust toolchain
/// (pinned by `mise.toml` and the workflow).
///
/// `code/scripts/vos_results_guard.py` mirrors this list, and a Python test parses it from this
/// file and compares, so the two cannot drift.
pub const TREE_ENTRIES: [(&str, &str); 46] = [
    (
        "vos-capacity-probe/src",
        "code/programs/rust/vos-capacity-probe/src",
    ),
    (
        "vos-capacity-probe/Cargo.toml",
        "code/programs/rust/vos-capacity-probe/Cargo.toml",
    ),
    (
        "vos-capacity-probe/Cargo.lock",
        "code/programs/rust/vos-capacity-probe/Cargo.lock",
    ),
    ("bit-blaster/src", "code/packages/rust/bit-blaster/src"),
    (
        "bit-blaster/Cargo.toml",
        "code/packages/rust/bit-blaster/Cargo.toml",
    ),
    ("cdcl-sat/src", "code/packages/rust/cdcl-sat/src"),
    (
        "cdcl-sat/Cargo.toml",
        "code/packages/rust/cdcl-sat/Cargo.toml",
    ),
    ("drat-check/src", "code/packages/rust/drat-check/src"),
    (
        "drat-check/Cargo.toml",
        "code/packages/rust/drat-check/Cargo.toml",
    ),
    (
        "riscv-simulator/src",
        "code/packages/rust/riscv-simulator/src",
    ),
    (
        "riscv-simulator/Cargo.toml",
        "code/packages/rust/riscv-simulator/Cargo.toml",
    ),
    (
        "vos-sched-fp-bitmap/tests/generated/warrant_evidence.rs",
        "code/packages/rust/vos-sched-fp-bitmap/tests/generated/warrant_evidence.rs",
    ),
    (
        "vos-probe-ready/tests/generated/warrant_evidence.rs",
        "code/packages/rust/vos-probe-ready/tests/generated/warrant_evidence.rs",
    ),
    (
        "specs/vos-sched-fp-bitmap.warrant",
        "code/specs/vos-sched-fp-bitmap.warrant",
    ),
    (
        "specs/vos-probe-ready.warrant",
        "code/specs/vos-probe-ready.warrant",
    ),
    ("arithmetic/src", "code/packages/rust/arithmetic/src"),
    (
        "arithmetic/Cargo.toml",
        "code/packages/rust/arithmetic/Cargo.toml",
    ),
    ("bounded-json/src", "code/packages/rust/bounded-json/src"),
    (
        "bounded-json/Cargo.toml",
        "code/packages/rust/bounded-json/Cargo.toml",
    ),
    ("sha256/src", "code/packages/rust/sha256/src"),
    ("sha256/Cargo.toml", "code/packages/rust/sha256/Cargo.toml"),
    (
        "test-bootstrap/src",
        "code/packages/rust/test-bootstrap/src",
    ),
    (
        "test-bootstrap/Cargo.toml",
        "code/packages/rust/test-bootstrap/Cargo.toml",
    ),
    (
        "warrant-grammar/src",
        "code/packages/rust/warrant-grammar/src",
    ),
    (
        "warrant-grammar/Cargo.toml",
        "code/packages/rust/warrant-grammar/Cargo.toml",
    ),
    ("zeroize/src", "code/packages/rust/zeroize/src"),
    (
        "zeroize/Cargo.toml",
        "code/packages/rust/zeroize/Cargo.toml",
    ),
    ("cpu-simulator/src", "code/packages/rust/cpu-simulator/src"),
    (
        "cpu-simulator/Cargo.toml",
        "code/packages/rust/cpu-simulator/Cargo.toml",
    ),
    ("grammar-tools/src", "code/packages/rust/grammar-tools/src"),
    (
        "grammar-tools/Cargo.toml",
        "code/packages/rust/grammar-tools/Cargo.toml",
    ),
    ("logic-gates/src", "code/packages/rust/logic-gates/src"),
    (
        "logic-gates/Cargo.toml",
        "code/packages/rust/logic-gates/Cargo.toml",
    ),
    ("riscv-encoder/src", "code/packages/rust/riscv-encoder/src"),
    (
        "riscv-encoder/Cargo.toml",
        "code/packages/rust/riscv-encoder/Cargo.toml",
    ),
    ("transistors/src", "code/packages/rust/transistors/src"),
    (
        "transistors/Cargo.toml",
        "code/packages/rust/transistors/Cargo.toml",
    ),
    (
        "vos-probe-ready/src",
        "code/packages/rust/vos-probe-ready/src",
    ),
    (
        "vos-probe-ready/Cargo.toml",
        "code/packages/rust/vos-probe-ready/Cargo.toml",
    ),
    (
        "vos-sched-contract/src",
        "code/packages/rust/vos-sched-contract/src",
    ),
    (
        "vos-sched-contract/Cargo.toml",
        "code/packages/rust/vos-sched-contract/Cargo.toml",
    ),
    (
        "vos-sched-fp-bitmap/src",
        "code/packages/rust/vos-sched-fp-bitmap/src",
    ),
    (
        "vos-sched-fp-bitmap/Cargo.toml",
        "code/packages/rust/vos-sched-fp-bitmap/Cargo.toml",
    ),
    (
        "test-bootstrap/tests/exit_fixture.rs",
        "code/packages/rust/test-bootstrap/tests/exit_fixture.rs",
    ),
    (
        "test-bootstrap/tests/self_corpus.rs",
        "code/packages/rust/test-bootstrap/tests/self_corpus.rs",
    ),
    ("repo/.cargo/config.toml", ".cargo/config.toml"),
];

/// The git ids (`git rev-parse HEAD:<path>`) of every [`TREE_ENTRIES`] input; empty if git cannot
/// say for any of them (a file without `trees` is then recorded, and `check --full` refuses it).
pub fn head_trees() -> Vec<(String, String)> {
    let root = repo_root();
    let mut out = Vec::new();
    for (name, path) in TREE_ENTRIES {
        match run("git", &["rev-parse", &format!("HEAD:{path}")], Some(&root)) {
            Some(id) if !id.is_empty() => out.push((name.to_string(), id)),
            _ => return Vec::new(),
        }
    }
    out
}

/// The ids of the WORKING tree: like [`head_trees`], but an entry whose path has an uncommitted
/// change (or an untracked file) gets `+uncommitted` appended, so it equals no recorded id.
pub fn working_trees() -> Vec<(String, String)> {
    let root = repo_root();
    let mut out = head_trees();
    for ((_, id), (_, path)) in out.iter_mut().zip(TREE_ENTRIES) {
        let dirty = run("git", &["status", "--porcelain", "--", path], Some(&root))
            .is_none_or(|s| !s.is_empty());
        if dirty {
            id.push_str("+uncommitted");
        }
    }
    out
}

/// Can a recorded `trees` object be compared with the working tree's ids at all?  Not when it is
/// empty (nothing is claimed), when git cannot say what the working tree is, or when it is the
/// legacy whole-crate record of VOS-D140.  Those are the cases [`stale_lines`] answers with one
/// line that is not "this input differs": `check --full --at-checkout` must still refuse them
/// (VOS-D141), because a verify of a file that cannot say which tree it describes would say
/// nothing, whereas an input that merely differs is exactly what a verify at the checkout is for.
pub fn is_comparable(recorded: &[(String, String)], current: &[(String, String)]) -> bool {
    !recorded.is_empty()
        && !current.is_empty()
        && !recorded
            .iter()
            .map(|(k, _)| k.as_str())
            .eq(LEGACY_TREE_NAMES)
}

/// Why `trees` cannot be the ids of a COMMITTED checkout, as [`head_trees`] reads one (VOS-D141).
/// A clean checkout yields exactly one `git rev-parse HEAD:<path>` per [`TREE_NAMES`] entry, in
/// order, each a 40- or 64-digit hex object id; there is no sentinel for an absent path
/// ([`head_trees`] returns nothing at all instead).  So anything else is refused: empty, the
/// legacy whole-crate names, a `+uncommitted` id ([`working_trees`]' mark of an edited input), or
/// a malformed id.  `check --full --at-checkout` refuses to produce such a file; `decision-diff`
/// uses this to refuse to accept one, so producer and consumer cannot disagree.
pub fn committed_tree_refusals(trees: &[(String, String)]) -> Vec<String> {
    let mut v = Vec::new();
    if trees.is_empty() {
        v.push("it records no tree ids".to_string());
        return v;
    }
    let names: Vec<&str> = trees.iter().map(|(k, _)| k.as_str()).collect();
    if names == LEGACY_TREE_NAMES {
        v.push("it records the legacy whole-crate tree names".to_string());
    } else if names != TREE_NAMES {
        v.push("its tree ids do not name exactly the recorded inputs".to_string());
    }
    for (k, id) in trees {
        if id.ends_with("+uncommitted") {
            v.push(format!("the id of {k} is marked +uncommitted"));
        } else if !((id.len() == 40 || id.len() == 64) && id.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            v.push(format!("the id of {k} is not a hex object id"));
        }
    }
    v
}

/// The names of the recorded inputs whose id differs from the working tree's (nothing, when the
/// record is not [`is_comparable`]).  What `check --full --at-checkout` names in its ok line.
pub fn differing_names(recorded: &[(String, String)], current: &[(String, String)]) -> Vec<String> {
    if !is_comparable(recorded, current) {
        return Vec::new();
    }
    recorded
        .iter()
        .filter(|(k, rec)| {
            current
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, x)| x.as_str())
                != Some(rec.as_str())
        })
        .map(|(k, _)| k.clone())
        .collect()
}

/// The names of the working tree's inputs with an uncommitted change (their id carries the
/// `+uncommitted` suffix [`working_trees`] appends).
pub fn uncommitted_inputs(current: &[(String, String)]) -> Vec<&str> {
    current
        .iter()
        .filter(|(_, id)| id.ends_with("+uncommitted"))
        .map(|(n, _)| n.as_str())
        .collect()
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
    // A record made before the input-level entries (VOS-D140) names five whole crate directories.
    // Its ids are historical and cannot be compared with any entry now, so it gets one line, not
    // five "absent" ones; `check --full` turns that line into a refusal, as a stale id always was.
    if recorded
        .iter()
        .map(|(k, _)| k.as_str())
        .eq(LEGACY_TREE_NAMES)
    {
        return vec![format!(
            "stale: {what}: recorded at whole-crate tree ids (before the input-level entries, \
             VOS-D140); they cannot be compared with the working tree"
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
    let st = clean_git(&["merge-base", "--is-ancestor", rev, "HEAD"])
        .status()
        .map_err(|e| format!("cannot run git: {e}"))?;
    match st.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!("git cannot place revision {rev}")),
    }
}

/// A `git` command aimed at THIS checkout.  An inherited `GIT_DIR` (a hook, a worktree wrapper)
/// would point git at a repository other than the one the reviewer is standing in, so the
/// variables that choose one are cleared.  Only on the calls that judge a revision: the other git
/// calls keep their environment.
fn clean_git(args: &[&str]) -> Command {
    // Every variable below can point git at a different repository, a different object store, a
    // different set of refs or different configuration than the checkout the reviewer is looking
    // at.  The sign-off rests on "the trees at this commit are the recorded ones", so the lookup
    // must read this checkout's own objects and nothing else.  `GIT_NO_REPLACE_OBJECTS` makes git
    // ignore `refs/replace/*`, which would otherwise let a local replace ref substitute another
    // commit's trees for the one named (security review of VOS-028c3c-2d).
    let mut c = Command::new("git");
    c.args(args);
    for var in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_REPLACE_REF_BASE",
        "GIT_NAMESPACE",
        "GIT_GRAFT_FILE",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CONFIG_COUNT",
    ] {
        c.env_remove(var);
    }
    c.env("GIT_NO_REPLACE_OBJECTS", "1")
        .current_dir(repo_root());
    c
}

/// The ids of every [`TREE_ENTRIES`] input AT the commit `rev` (a 40-hex id): the same names and
/// ids [`head_trees`] would have recorded had HEAD been `rev` (VOS-D137).
///
/// Why this and not ancestry: main is squash-merged, so the commit a CI recording ran at is never
/// in main's history, but the trees it recorded are, byte for byte, as long as nothing under
/// those inputs changed since.  A tree id is a hash of the content, so equality of ids is
/// equality of content.
///
/// `Err` when `rev` is not 40-hex, when the commit is not in this checkout's object store (the
/// message names `git fetch origin <rev>`; GitHub keeps a pull request's head commits), or when
/// git cannot read a path at it.
pub fn trees_at(rev: &str) -> Result<Vec<(String, String)>, String> {
    if rev.len() != 40 || !rev.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("{rev:?} is not a 40-hex revision"));
    }
    let have = clean_git(&["cat-file", "-e", &format!("{rev}^{{commit}}")])
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !have.status.success() {
        return Err(format!(
            "commit {rev} is not in this checkout (run `git fetch origin {rev}` and sign again)"
        ));
    }
    let mut out = Vec::new();
    for (name, path) in TREE_ENTRIES {
        // No `^{tree}` peel here: in `<rev>:<path>` everything after the colon is the path, so a
        // suffix would be read as part of it.  The id is only ever compared for equality with the
        // recorded one, which `head_trees` produced the same way, so a blob id could not match.
        let r = clean_git(&["rev-parse", &format!("{rev}:{path}")])
            .output()
            .map_err(|e| format!("cannot run git: {e}"))?;
        let id = String::from_utf8_lossy(&r.stdout).trim().to_string();
        // A path missing at `rev` makes git print the spec back and fail; an id is exactly 40
        // hex digits, so anything else is refused here rather than compared.
        if !r.status.success() || id.len() != 40 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("git cannot read {path} at commit {rev}"));
        }
        out.push((name.to_string(), id));
    }
    Ok(out)
}

/// The current UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
pub fn utc_now() -> String {
    run("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], None).unwrap_or_else(|| "unknown".into())
}
