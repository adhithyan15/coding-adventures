//! # The safe lane artifact loader (WD-004b1, VER04i section 8.2)
//!
//! [`crate::lane_evidence`] (WD-004a) parses *bytes*.  This module is the first
//! code that *opens files*, so it is where the hostile-checkout questions get
//! answered:
//!
//! 1. **Can a registry path make us read something outside the repository?**
//!    No: [`read_repo_file`] joins a lexically-validated relative path to the
//!    repository root, re-checks containment with the lane runners' shared
//!    rule ([`lexical_containment`]), refuses every symbolic link beneath the
//!    root, and insists on a regular file.
//! 2. **Can a file be so large that reading it is itself the attack?**  No:
//!    every read has a cap that is checked against the file's size *and* again
//!    against the bytes actually read.
//! 3. **Do the recorded digests describe the bytes on disk?**  Each raw digest
//!    is a SHA-256 of exactly the bytes this loader read (VER04i 8.2).
//! 4. **Is the evidence honest about what it does not know?**  A successful
//!    load carries the *names* of the static checks that passed and a list of
//!    what is **not** claimed.  Nothing here says the sources are fresh, the
//!    build ran, or the toolchain is genuine.
//!
//! ## What this module is not
//!
//! * It is not a report generator and has no CLI: the report rungs live in
//!   [`crate::lane_report`] and the `--check-lane` command in `main.rs`
//!   (WD-004b2), both of which call [`load_lane`].
//! * It never touches the legacy Rust loader in [`crate::evidence`]: a Rust
//!   package is not in the consumer registry, so [`load_lane`] refuses it with
//!   `not-registered`, and a Rust `warrant-test-evidence-v1` manifest is never
//!   read as a lane manifest.
//! * It makes no claim that a manifest was *produced* by running the source
//!   whose digest it records: `source_sha256` and `runner.src_sha256` are
//!   recomputed from the tree on disk (WD-004b2) and compared, which shows the
//!   manifest is not stale, not that it was earned.
//!
//! ## Reading order of one [`load_lane`]
//!
//! ```text
//!   registry ──► row for <package> ──► status must be "active"
//!      │
//!      ├─ spec, corpus.json, cases.tfv   (bounded, contained, raw bytes)
//!      ├─ manifest ──► strict parse ──► identity equals the registry row
//!      ├─ raw digests: spec / corpus / cases against the manifest
//!      ├─ WD-004a join: ids, counts, stream digests, controls
//!      ├─ binding file ──► sha256 equals manifest.binding_sha256
//!      ├─ runner identity (name + lane)
//!      ├─ labels sidecar ──► closed schema, labels re-derived, digest bound
//!      ├─ runner entry + build files ──► exist, contained, build routing text
//!      └─ source root + runner root ──► recomputed `warrant-src-v1` digests
//! ```
//!
//! Any failure is an `Err`; there is no partial result.

#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::Path;

use coding_adventures_sha256::sha256_hex;

use crate::evidence::MAX_MANIFEST_BYTES;
use crate::json::JsonValue;
use crate::lane_evidence::{
    parse_consumer_registry, parse_lane_manifest, validate_integer_tokens, validate_lane_evidence,
    validate_repo_path, ExhaustiveEntry, LaneConsumer, LaneEvidenceError, LaneEvidenceManifest,
    LaneTestEntry, CORPUS_FILE, SPEC_FILE,
};

// ---------------------------------------------------------------------------
// Caps.  Registry, manifest and sidecar share the WD-004a manifest cap; the
// spec, corpus and TFV caps are the WD-004a lane-profile caps.
// ---------------------------------------------------------------------------

const MAX_SPEC_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CORPUS_BYTES: u64 = 32 * 1024 * 1024;
const MAX_BINDING_BYTES: u64 = 1024 * 1024;
const MAX_RUNNER_ENTRY_BYTES: u64 = 4 * 1024 * 1024;
const MAX_BUILD_FILE_BYTES: u64 = 256 * 1024;

/// The runner every lane manifest must name (VER04i 8: `runner.name`).
pub const RUNNER_NAME: &str = "warrant-corpus";

/// The static readiness checks, in the order a successful load reports them.
/// The names are the ones VER04i 8.2 lists; a test pins the two together.
pub const READINESS_CHECKS: [&str; 10] = [
    "paths-contained",
    "registry-active",
    "corpus-digests",
    "corpus-join",
    "binding-digest",
    "runner-identity",
    "labels-bound",
    "build-routing",
    "source-fresh",
    "runner-source-fresh",
];

/// What a successful load deliberately does **not** establish.  The output of
/// [`LoadedLane::render_readiness`] always prints this list.
pub const UNCLAIMED: [&str; 5] = [
    "toolchain authenticity",
    "build execution",
    "sandbox",
    "go.mod go directive version",
    "contents of directories the Go tool ignores",
];

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// A failed load.  `code` is a stable machine-readable id (tests and WD-004b2
/// match on it); `message` is for humans and never echoes whole untrusted
/// files.
///
/// Codes: `path`, `symlink`, `not-regular`, `too-large`, `io`, `registry`,
/// `not-registered`, `not-active`, `manifest`, `digest`, `join`,
/// `runner-identity`, `labels`, `build-routing`, `source-tree`,
/// `uncovered-source`, `go-mod`, `stale-source`, `stale-runner`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneLoadError {
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for LaneLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lane load error [{}]: {}", self.code, self.message)
    }
}

impl std::error::Error for LaneLoadError {}

fn err<T>(code: &'static str, message: impl Into<String>) -> Result<T, LaneLoadError> {
    Err(LaneLoadError {
        code,
        message: message.into(),
    })
}

fn lift<T>(code: &'static str, result: Result<T, LaneEvidenceError>) -> Result<T, LaneLoadError> {
    result.map_err(|LaneEvidenceError(message)| LaneLoadError { code, message })
}

// ---------------------------------------------------------------------------
// Lexical containment (the rule every lane runner shares)
// ---------------------------------------------------------------------------

/// Which path grammar a containment question is asked in.  Production code
/// always uses [`PathFlavor::host`]; both flavours exist so the rules for the
/// *other* platform stay testable on this one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathFlavor {
    Posix,
    Win32,
}

impl PathFlavor {
    /// The flavour of the platform this binary runs on.
    pub fn host() -> PathFlavor {
        if cfg!(target_os = "windows") {
            PathFlavor::Win32
        } else {
            PathFlavor::Posix
        }
    }
}

/// The answer to "is `child` inside `root`?".  `same` means the child *is* the
/// root, in which case `inside` is also true.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Containment {
    pub inside: bool,
    pub same: bool,
}

const NEVER: Containment = Containment {
    inside: false,
    same: false,
};

/// A path reduced to `(prefix, components)`: two paths can only be related if
/// their prefixes (root, drive or UNC share) are identical.
struct Normal {
    prefix: String,
    parts: Vec<String>,
}

/// Split `rest` on `/`, dropping empty and `.` components and letting `..`
/// pop (a `..` at the root stays at the root).  `fold` lower-cases **ASCII
/// only** for the case-insensitive Windows flavour (VER04i 9.1, WD-100b):
/// `str::to_lowercase` is full Unicode and would equate the Kelvin sign with
/// `k`, so a non-ASCII case variant must stay a different name (fail closed).
fn components(rest: &str, fold: bool) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    for part in rest.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(if fold {
                other.to_ascii_lowercase()
            } else {
                other.to_owned()
            }),
        }
    }
    parts
}

/// `None` means "unusable as either side of a containment question": relative,
/// drive-relative, a device path, or a POSIX `//x` path, all of which the
/// conservative rule says are never inside anything.
fn normalize(flavor: PathFlavor, path: &str) -> Option<Normal> {
    match flavor {
        PathFlavor::Posix => {
            if !path.starts_with('/') {
                return None;
            }
            // Exactly two leading slashes is implementation-defined by POSIX;
            // three or more collapse to one.
            let leading = path.bytes().take_while(|b| *b == b'/').count();
            if leading == 2 {
                return None;
            }
            Some(Normal {
                prefix: "/".into(),
                parts: components(path, false),
            })
        }
        PathFlavor::Win32 => {
            // Separators are normalised *before* any prefix rule, so
            // `\/?\D:\r`, `/\?/D:/r` and `\\?/D:\r` are all device paths.
            let p = path.replace('\\', "/");
            if let Some(rest) = p.strip_prefix("//") {
                let head = rest.split('/').next().unwrap_or("");
                if head == "?" || head == "." {
                    return None; // device namespace, including `\\?\UNC\...`
                }
                // UNC: `//server/share/...`.  The server and share are the
                // prefix; fewer than two names is not a path we can reason about.
                if rest.starts_with('/') {
                    return None;
                }
                let mut names = rest.split('/').filter(|s| !s.is_empty());
                let server = names.next()?;
                let share = names.next()?;
                let tail: Vec<&str> = names.collect();
                return Some(Normal {
                    prefix: format!(
                        "//{}/{}",
                        server.to_ascii_lowercase(),
                        share.to_ascii_lowercase()
                    ),
                    parts: components(&tail.join("/"), true),
                });
            }
            let bytes = p.as_bytes();
            if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
                // `C:foo` is drive-relative, not absolute.
                if bytes.len() < 3 || bytes[2] != b'/' {
                    return None;
                }
                return Some(Normal {
                    prefix: format!("{}:", (bytes[0] as char).to_ascii_lowercase()),
                    parts: components(&p[2..], true),
                });
            }
            if p.starts_with('/') {
                return Some(Normal {
                    prefix: "/".into(),
                    parts: components(&p, true),
                });
            }
            None
        }
    }
}

/// Is this a Windows path component that must be refused (VER04i 9.1)?  Any
/// of: a colon (an alternate data stream or device suffix: `CON:`,
/// `nul::$DATA`, `file.txt:NUL`); a trailing dot or space (Windows strips
/// them, so two spellings would name one file); or a *stem* that is a device
/// name.  The stem is the part before the first `.` with trailing spaces
/// removed (`CON.txt`, `com1.tar.gz` and `CON .txt` are devices).  The set is
/// `CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`, `CONIN$`,
/// `CONOUT$` and `COM`/`LPT` plus a superscript 1, 2 or 3; only ASCII folds,
/// so fullwidth `CON` and `CLOCK$` stay ordinary, as do `console.txt`,
/// `COM10`, `LPT0` and `CONx`.
fn refused_component(name: &str) -> bool {
    if name.contains(':') || name.ends_with('.') || name.ends_with(' ') {
        return true;
    }
    let stem = name.split('.').next().unwrap_or("").trim_end_matches(' ');
    let stem = stem.to_ascii_uppercase();
    match stem.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" => true,
        _ => {
            let digit = |rest: &str| {
                matches!(
                    rest,
                    "1" | "2"
                        | "3"
                        | "4"
                        | "5"
                        | "6"
                        | "7"
                        | "8"
                        | "9"
                        | "\u{b9}"
                        | "\u{b2}"
                        | "\u{b3}"
                )
            };
            stem.strip_prefix("COM").is_some_and(digit)
                || stem.strip_prefix("LPT").is_some_and(digit)
        }
    }
}

/// Is `child` inside `root`?  Purely lexical: both inputs must be absolute and
/// nothing is looked up on disk, so symbolic links are the caller's concern
/// ([`read_repo_file`] refuses them).  Whole components are compared, never
/// string prefixes (`/rr` is not inside `/r`), and paths on different drives or
/// shares have no common root, whatever a runtime's `relative()` would say.
pub fn lexical_containment(flavor: PathFlavor, root: &str, child: &str) -> Containment {
    let (Some(root), Some(child)) = (normalize(flavor, root), normalize(flavor, child)) else {
        return NEVER;
    };
    if root.prefix != child.prefix || child.parts.len() < root.parts.len() {
        return NEVER;
    }
    if child.parts[..root.parts.len()] != root.parts[..] {
        return NEVER;
    }
    // Under win32 a child with a reserved device name or a trailing dot/space
    // component is never inside anything (WD-100b).  The parts are already
    // ASCII-lowercased, which the case-insensitive check does not mind.
    if flavor == PathFlavor::Win32 && child.parts.iter().any(|part| refused_component(part)) {
        return NEVER;
    }
    Containment {
        inside: true,
        same: child.parts.len() == root.parts.len(),
    }
}

// ---------------------------------------------------------------------------
// Bounded, contained, symlink-refusing reads
// ---------------------------------------------------------------------------

/// Is this metadata a symbolic link or, on Windows, any reparse point (which
/// covers junctions and mount points as well as symlinks)?
fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}

/// Turn the extended-length spelling the Windows API hands back for a path
/// (`\\?\D:\a\repo`, `\\?\UNC\server\share\x`) into the plain one
/// (`D:\a\repo`, `\\server\share\x`).  It is for paths the *operating
/// system* produced (a canonicalised repository root), never for caller input:
/// [`lexical_containment`] keeps refusing `\\?\` as a device path.  Only the
/// two exact verbatim forms are converted (a verbatim path is not normalised
/// by Windows, so any `.`, `..` or `/` inside would change meaning, and such
/// text is left alone to be refused).  The POSIX flavour never changes.
pub fn plain_root_text(flavor: PathFlavor, text: &str) -> String {
    if flavor != PathFlavor::Win32 {
        return text.to_owned();
    }
    let Some(rest) = text.strip_prefix("\\\\?\\") else {
        return text.to_owned();
    };
    let clean = |s: &str| !s.contains('/') && !s.split('\\').any(|c| c == "." || c == "..");
    if !clean(rest) {
        return text.to_owned();
    }
    if let Some(unc) = rest.strip_prefix("UNC\\") {
        return format!("\\\\{unc}");
    }
    let b = rest.as_bytes();
    if b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\' {
        return rest.to_owned();
    }
    text.to_owned()
}

/// What the last component of a contained path must be.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    File,
    Dir,
}

/// Steps 1 to 3 of every read: the registry's lexical rules, the shared
/// containment rule in the host flavour, and a component-by-component walk that
/// refuses links.  Returns the path to open and its absolute text (the base
/// for the containment questions a directory walk asks later).
fn contained_path(
    repo_root: &Path,
    rel: &str,
    kind: Kind,
) -> Result<(std::path::PathBuf, String), LaneLoadError> {
    // Step 1: the registry's lexical rules (ASCII, slash-separated, no empty,
    // `.` or `..` component, portable characters).  Cheap, and it runs before
    // any filesystem call.
    if let Err(LaneEvidenceError(message)) = validate_repo_path(rel) {
        return err("path", format!("{rel:?}: {message}"));
    }
    // Step 2: the shared containment rule, in the host's path flavour.  With a
    // validated `rel` it cannot fail, but a production path must go through
    // the same helper the fixture drives, so it is asserted, not assumed.
    let root_abs = std::path::absolute(repo_root).map_err(|e| LaneLoadError {
        code: "io",
        message: format!("repository root: {e}"),
    })?;
    let root_text = root_abs.to_str().ok_or_else(|| LaneLoadError {
        code: "path",
        message: "repository root is not UTF-8".into(),
    })?;
    // The root usually arrives from `Path::canonicalize`, which on Windows
    // returns an extended-length `\\?\D:\...` path.  The conformance rule
    // refuses every `\\?\` path as a device path, so the OS's own spelling of
    // the *root* is turned into the plain form first.  `rel` is never touched.
    let root_plain = plain_root_text(PathFlavor::host(), root_text);
    let root_text = root_plain.as_str();
    let joined = format!("{}/{rel}", root_text.trim_end_matches(['/', '\\']));
    let verdict = lexical_containment(PathFlavor::host(), root_text, &joined);
    if !verdict.inside || verdict.same {
        return err(
            "path",
            format!("{rel}: resolves outside the repository root"),
        );
    }
    // Step 3: walk the components, refusing links and wrong-kind targets.
    let parts: Vec<&str> = rel.split('/').collect();
    let mut current = repo_root.to_path_buf();
    let mut shown = String::new();
    for (index, part) in parts.iter().enumerate() {
        current.push(part);
        if !shown.is_empty() {
            shown.push('/');
        }
        shown.push_str(part);
        let meta = fs::symlink_metadata(&current).map_err(|e| LaneLoadError {
            code: "io",
            message: format!("{rel}: {e}"),
        })?;
        if is_link(&meta) {
            return err(
                "symlink",
                format!("{shown} is a symbolic link; links are refused"),
            );
        }
        let last = index + 1 == parts.len();
        if last && kind == Kind::File && !meta.is_file() {
            return err("not-regular", format!("{rel} is not a regular file"));
        }
        if last && kind == Kind::Dir && !meta.is_dir() {
            return err("not-regular", format!("{rel} is not a directory"));
        }
        if !last && !meta.is_dir() {
            return err("not-regular", format!("{shown} is not a directory"));
        }
    }
    Ok((current, joined))
}

/// Read the repository-relative file `rel` under `repo_root`, refusing anything
/// but a bounded regular file reached without a symbolic link.
///
/// The repository root itself is trusted (it may be a symlink); every
/// component *beneath* it is inspected with `symlink_metadata`.  This is a
/// defence against a hostile checkout, not against a process that can rewrite
/// the tree while the load runs (the check-to-open window of VER04i 8.2, still
/// open).
pub fn read_repo_file(repo_root: &Path, rel: &str, cap: u64) -> Result<Vec<u8>, LaneLoadError> {
    let (current, _) = contained_path(repo_root, rel, Kind::File)?;
    // Step 4: bounded read.  The size check rejects the common case without
    // reading; `take` makes the cap hold even if the file grows meanwhile.
    let file = fs::File::open(&current).map_err(|e| LaneLoadError {
        code: "io",
        message: format!("{rel}: {e}"),
    })?;
    let size = file
        .metadata()
        .map_err(|e| LaneLoadError {
            code: "io",
            message: format!("{rel}: {e}"),
        })?
        .len();
    if size > cap {
        return err(
            "too-large",
            format!("{rel} is {size} bytes, over the {cap}-byte limit"),
        );
    }
    let mut bytes = Vec::with_capacity(size as usize);
    file.take(cap + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| LaneLoadError {
            code: "io",
            message: format!("{rel}: {e}"),
        })?;
    if bytes.len() as u64 > cap {
        return err(
            "too-large",
            format!("{rel} grew past the {cap}-byte limit while reading"),
        );
    }
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Source digests (WD-004b2, VER04i 8.2 "Source digests")
// ---------------------------------------------------------------------------
//
// `source_sha256` is the `warrant-src-v1` digest of a lane's production source
// and `runner.src_sha256` that of the runner's own.  Both are recomputed here
// from the tree on disk.  The framing is `test_bootstrap`'s, shared with the
// Rust producer; the *selection* of files is each lane's rule, which has to
// match the runner that wrote the manifest byte for byte.
//
//   directory ──► every entry: symlink_metadata first (link ⇒ refuse)
//                   ├─ directory ──► skipped by name?  no ⇒ descend
//                   ├─ regular file selected by the rule? ⇒ contained, bounded read
//                   └─ anything else (FIFO, socket, device) ⇒ stepped over, never opened
//
// The link check comes *before* the name rule, as in the runners, so a link
// named `notes.txt` or `__pycache__` is still refused.

/// A runner's per-file limit; the loader adds the two totals below.
const MAX_SOURCE_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SOURCE_DEPTH: usize = 32;
const MAX_SOURCE_FILES: usize = 4096;
/// Entries the package-wide bytecode scan will look at before giving up.
const MAX_PACKAGE_ENTRIES: usize = 100_000;
const MAX_SOURCE_TOTAL_BYTES: u64 = 64 * 1024 * 1024;

/// Which of a lane's two digests a walk computes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    /// `source_sha256`: the package's production source.
    Production,
    /// `runner.src_sha256`: the `warrant-corpus` runner's own source.
    Runner,
}

/// The runner's own source root, relative to the repository (VER04i 8.2).  The
/// registry carries no runner root, so it is fixed per lane.  `None` for a lane
/// with no rule.
pub fn runner_root(lane: &str) -> Option<&'static str> {
    match lane {
        "python" => {
            Some("code/packages/python/warrant-corpus/src/coding_adventures_warrant_corpus")
        }
        "typescript" => Some("code/packages/typescript/warrant-corpus/src"),
        "go" => Some("code/packages/go/warrant-corpus"),
        _ => None,
    }
}

/// The package that holds a lane's runner root: the whole package, whose
/// `tests/` the runner-root digest never reads (WD-004b3).
fn runner_package_root(lane: &str) -> Option<&'static str> {
    match lane {
        "python" => Some("code/packages/python/warrant-corpus"),
        _ => None,
    }
}

/// How one lane selects its files.
struct SourceRule {
    select: fn(&str) -> bool,
    skip_dir: fn(&str) -> bool,
    /// `false` for the Go runner, which embeds only its own directory's `*.go`.
    recursive: bool,
    /// The lane's own part of the ignore set: a file that is not selected but is
    /// known not to change behaviour (tests).  Arguments: lower-cased name, and
    /// whether the file is directly in the root.
    ignore_file: fn(&str, bool) -> bool,
    /// Directories at the root that are listed as not source (Go package data).
    ignore_root_dir: fn(&str) -> bool,
    /// Directories that may not exist at all, at any depth: they can shadow or
    /// replace what the digest covers (Go `vendor`, TypeScript `node_modules`
    /// and `dist`).  Refused, never skipped.
    refuse_dir: fn(&str) -> bool,
    /// Python: `__pycache__` may exist but must hold nothing.  Python runs a
    /// cached `.pyc` instead of its source, and no header check can tell a real
    /// cache from a forged one (a hash-based pyc is never revalidated; a
    /// timestamp pyc can be made to match the source's mtime and size).
    pycache: bool,
}

/// The ignore set every lane shares (VER04i 8.1): documentation and editor
/// litter, compared case-insensitively.  Anything else a rule does not select
/// is an `uncovered-source` failure.
fn common_ignore(lower: &str) -> bool {
    matches!(
        lower,
        "readme"
            | "readme.md"
            | "changelog.md"
            | "license"
            | "license.md"
            | ".ds_store"
            | ".gitkeep"
    )
}

fn source_rule(lane: &str, kind: SourceKind) -> Option<SourceRule> {
    match lane {
        "python" => Some(SourceRule {
            select: |name| name.ends_with(".py") || name.ends_with(".pyi") || name == "py.typed",
            skip_dir: |name| name == "__pycache__",
            recursive: true,
            ignore_file: |_, _| false,
            ignore_root_dir: |_| false,
            refuse_dir: |_| false,
            pycache: true,
        }),
        "typescript" => Some(SourceRule {
            select: |name| name.ends_with(".ts") && !name.ends_with(".test.ts"),
            skip_dir: |_| false,
            recursive: true,
            ignore_file: |lower, _| lower.ends_with(".test.ts"),
            ignore_root_dir: |_| false,
            refuse_dir: |name| name == "dist" || name == "node_modules",
            pycache: false,
        }),
        "go" => Some(SourceRule {
            select: |name| name.ends_with(".go") && !name.ends_with("_test.go"),
            skip_dir: |name| name == "testdata" || name.starts_with('.') || name.starts_with('_'),
            recursive: kind == SourceKind::Production,
            // Tests, and the two dependency files at the root: `go.mod` and `go.sum`
            // pin dependencies and are NOT digested (a listed residual, VER04i 8.1);
            // a go.mod anywhere else starts a new module and is refused.
            ignore_file: |lower, at_root| {
                lower.ends_with("_test.go")
                    || (at_root
                        && matches!(
                            lower,
                            "go.mod"
                                | "build"
                                | "build_windows"
                                | "required_capabilities.json"
                                | "evidence.bindings"
                        ))
            },
            ignore_root_dir: |name| name == "evidence" || name == "CHANGELOG.d",
            refuse_dir: |name| name == "vendor",
            pycache: false,
        }),
        _ => None,
    }
}

fn source_fail<T>(message: impl Into<String>) -> Result<T, LaneLoadError> {
    err("source-tree", message)
}

/// The `source` line of a binding file: `.` (the package root) or a relative,
/// slash-separated path under it.  Exactly one line, exactly one value, under
/// the registry's path rules, so the joined path cannot leave the package.
pub fn binding_source(bytes: &[u8]) -> Result<String, LaneLoadError> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return source_fail("binding file is not UTF-8");
    };
    let mut found: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut tokens = line.split_whitespace();
        if tokens.next() != Some("source") {
            continue;
        }
        let (Some(value), None) = (tokens.next(), tokens.next()) else {
            return source_fail("binding `source` line must have exactly one value");
        };
        if found.is_some() {
            return source_fail("binding has more than one `source` line");
        }
        if value != "." {
            if let Err(LaneEvidenceError(message)) = validate_repo_path(value) {
                return source_fail(format!("binding source {value:?}: {message}"));
            }
        }
        found = Some(value.to_owned());
    }
    match found {
        Some(value) => Ok(value),
        None => source_fail("binding has no `source` line"),
    }
}

struct Walk {
    rule: SourceRule,
    /// Absolute text of the walk's root, the base of every containment question.
    root_text: String,
    files: Vec<(String, Vec<u8>)>,
    total: u64,
}

impl Walk {
    fn directory(&mut self, dir: &Path, rel: &str, depth: usize) -> Result<(), LaneLoadError> {
        if depth > MAX_SOURCE_DEPTH {
            return source_fail("source tree is nested too deeply");
        }
        let listing = fs::read_dir(dir).map_err(|e| LaneLoadError {
            code: "io",
            message: format!(
                "cannot list {}: {e}",
                if rel.is_empty() {
                    "the source root"
                } else {
                    rel
                }
            ),
        })?;
        let mut entries: Vec<(String, std::path::PathBuf)> = Vec::new();
        for entry in listing {
            let entry = entry.map_err(|e| LaneLoadError {
                code: "io",
                message: format!("cannot list the source tree: {e}"),
            })?;
            let Ok(name) = entry.file_name().into_string() else {
                return source_fail("non-UTF-8 file name in the source tree");
            };
            entries.push((name, entry.path()));
        }
        // Byte order, like the runners; the final sort is by relative path.
        entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        for (name, path) in entries {
            let shown = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            let meta = fs::symlink_metadata(&path).map_err(|e| LaneLoadError {
                code: "io",
                message: format!("{shown}: {e}"),
            })?;
            if is_link(&meta) {
                return err(
                    "symlink",
                    format!("{shown} is a symbolic link; links are refused"),
                );
            }
            let at_root = rel.is_empty();
            if meta.is_dir() {
                if (self.rule.refuse_dir)(&name) {
                    return err(
                        "uncovered-source",
                        format!("{shown} may not exist in a source root: it can replace what the digest covers (VER04i 8.1)"),
                    );
                } else if self.rule.pycache && name == "__pycache__" {
                    self.pycache(&path, &shown)?;
                } else if at_root && (self.rule.ignore_root_dir)(&name) {
                    // listed as not source
                } else if self.rule.recursive && !(self.rule.skip_dir)(&name) {
                    self.directory(&path, &shown, depth + 1)?;
                }
            } else if meta.is_file() {
                if (self.rule.select)(&name) {
                    self.file(&path, &shown, meta.len())?;
                } else {
                    let lower = name.to_ascii_lowercase();
                    if !common_ignore(&lower) && !(self.rule.ignore_file)(&lower, at_root) {
                        return err(
                            "uncovered-source",
                            format!(
                                "{shown} is in the source root but no digest rule covers it and it is not in the ignore set (VER04i 8.1)"
                            ),
                        );
                    }
                }
            }
            // A FIFO, socket or device is neither: stepped over, never opened.
        }
        Ok(())
    }

    /// `__pycache__` must be empty: any entry is bytecode (or something else)
    /// the interpreter could run in place of the source.
    fn pycache(&self, cache: &Path, shown: &str) -> Result<(), LaneLoadError> {
        let mut listing = fs::read_dir(cache).map_err(|e| LaneLoadError {
            code: "io",
            message: format!("cannot list {shown}: {e}"),
        })?;
        if let Some(entry) = listing.next() {
            let entry = entry.map_err(|e| LaneLoadError {
                code: "io",
                message: format!("cannot list {shown}: {e}"),
            })?;
            return err(
                "uncovered-source",
                format!(
                    "{shown}/{} is bytecode or other content in __pycache__: Python can run it instead of the source, so none is accepted (VER04i 8.1)",
                    entry.file_name().to_string_lossy()
                ),
            );
        }
        Ok(())
    }

    fn file(&mut self, path: &Path, shown: &str, size: u64) -> Result<(), LaneLoadError> {
        let child = format!("{}/{shown}", self.root_text);
        let verdict = lexical_containment(PathFlavor::host(), &self.root_text, &child);
        if !verdict.inside || verdict.same {
            return err("path", format!("{shown}: resolves outside the source root"));
        }
        if self.files.len() >= MAX_SOURCE_FILES {
            return source_fail(format!("more than {MAX_SOURCE_FILES} source files"));
        }
        if size > MAX_SOURCE_FILE_BYTES {
            return source_fail(format!(
                "{shown} is {size} bytes, over the {MAX_SOURCE_FILE_BYTES}-byte limit"
            ));
        }
        let file = fs::File::open(path).map_err(|e| LaneLoadError {
            code: "io",
            message: format!("{shown}: {e}"),
        })?;
        let mut bytes = Vec::new();
        file.take(MAX_SOURCE_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| LaneLoadError {
                code: "io",
                message: format!("{shown}: {e}"),
            })?;
        if bytes.len() as u64 > MAX_SOURCE_FILE_BYTES {
            return source_fail(format!(
                "{shown} grew past the {MAX_SOURCE_FILE_BYTES}-byte limit while reading"
            ));
        }
        self.total += bytes.len() as u64;
        if self.total > MAX_SOURCE_TOTAL_BYTES {
            return source_fail(format!(
                "the source files exceed {MAX_SOURCE_TOTAL_BYTES} bytes in total"
            ));
        }
        self.files.push((shown.to_owned(), bytes));
        Ok(())
    }
}

/// The `warrant-src-v1` digest (SHA-256, lowercase hex) of the tree at the
/// repository-relative `rel_root` under `lane`'s rule for `kind`.  The root is
/// reached under the same rules as every other read (validated, lexically
/// contained, links refused per component, must be a directory); the walk is
/// described above.  A lane with no rule is `source-tree`.
pub fn source_tree_digest(
    repo_root: &Path,
    rel_root: &str,
    lane: &str,
    kind: SourceKind,
    pycache_isolated: bool,
) -> Result<String, LaneLoadError> {
    let Some(mut rule) = source_rule(lane, kind) else {
        return source_fail(format!("no source rule for lane {lane}"));
    };
    // A manifest that records a private bytecode cache (the Python runner ran
    // with `sys.pycache_prefix` set, so no `__pycache__` in the tree was ever
    // executed) makes a `__pycache__` directory irrelevant: it is skipped like
    // any derived directory.  Without that record it must be empty.
    if pycache_isolated && rule.pycache {
        rule.pycache = false;
        rule.skip_dir = |name| name == "__pycache__";
    }
    let (root, root_text) = contained_path(repo_root, rel_root, Kind::Dir)?;
    let mut walk = Walk {
        rule,
        root_text,
        files: Vec::new(),
        total: 0,
    };
    walk.directory(&root, "", 0)?;
    let framed = coding_adventures_test_bootstrap::manifest::frame_sources(&walk.files);
    Ok(sha256_hex(&framed))
}

// ---------------------------------------------------------------------------
// The Go module files (WD-004b2 review)
// ---------------------------------------------------------------------------
//
// `go.mod` is not part of the Go digest, yet its `replace` line chooses which
// directory the runner module is built from.  So it is pinned exactly instead:
// only the directives below, with only the values below, or the load fails.

const GO_MODULE_BASE: &str = "github.com/adhithyan15/coding-adventures/";

fn go_mod_fail<T>(file: &str, message: impl Into<String>) -> Result<T, LaneLoadError> {
    err("go-mod", format!("{file}: {}", message.into()))
}

/// Pin one `go.mod`: `module <module>`, `go <major.minor[.patch]>`, and, when
/// `runner` is given, exactly one `require <runner module> v0.0.0` and one
/// `replace <runner module> => <target>`.  Blank lines and `//` comments are
/// harmless; any other directive, block form or value is an error.
fn pin_go_mod(
    file: &str,
    text: &str,
    module: &str,
    runner: Option<(&str, &str)>,
) -> Result<(), LaneLoadError> {
    let (mut seen_module, mut seen_go, mut seen_require, mut seen_replace) =
        (false, false, false, false);
    let version_ok = |v: &str| {
        (2..=3).contains(&v.split('.').count())
            && v.split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if line.ends_with('(') {
            return go_mod_fail(file, "a block form hides directives from the pin");
        }
        match tokens.as_slice() {
            ["module", path] if *path == module && !seen_module => seen_module = true,
            ["module", ..] => {
                return go_mod_fail(file, format!("module path must be exactly {module}"));
            }
            ["go", version] if version_ok(version) && !seen_go => seen_go = true,
            ["require", path, "v0.0.0"]
                if runner.is_some_and(|(m, _)| m == *path) && !seen_require =>
            {
                seen_require = true;
            }
            ["replace", path, "=>", target]
                if runner.is_some_and(|(m, t)| m == *path && t == *target) && !seen_replace =>
            {
                seen_replace = true;
            }
            ["replace", ..] => {
                return go_mod_fail(
                    file,
                    format!(
                        "replace must be exactly the runner module => {}",
                        runner.map_or("(none allowed)", |(_, t)| t)
                    ),
                );
            }
            [directive, ..] => {
                return go_mod_fail(file, format!("unexpected directive `{directive}`"));
            }
            [] => {}
        }
    }
    if !seen_module {
        return go_mod_fail(file, "missing module directive");
    }
    if !seen_go {
        return go_mod_fail(file, "missing go directive");
    }
    if runner.is_some() && !(seen_require && seen_replace) {
        return go_mod_fail(
            file,
            "missing the runner module require and replace directives",
        );
    }
    Ok(())
}

/// Pin the Go lane's two `go.mod` files: the package's (module path from its
/// registry `package_root`, the runner module required and replaced with the
/// sibling `../<runner dir>`) and the runner's own (module and go version only).
fn check_go_modules(
    repo_root: &Path,
    package_root: &str,
    runner_dir: &str,
) -> Result<(), LaneLoadError> {
    let parent = |p: &str| p.rsplit_once('/').map(|(d, _)| d.to_owned());
    let runner_name = runner_dir.rsplit('/').next().unwrap_or("");
    if parent(package_root) != parent(runner_dir) {
        return go_mod_fail(package_root, "the runner must be a sibling of the package");
    }
    let runner_module = format!("{GO_MODULE_BASE}{runner_dir}");
    let target = format!("../{runner_name}");
    for (rel_dir, module, runner) in [
        (
            package_root,
            format!("{GO_MODULE_BASE}{package_root}"),
            Some((runner_module.as_str(), target.as_str())),
        ),
        (runner_dir, runner_module.clone(), None),
    ] {
        let path = format!("{rel_dir}/go.mod");
        let bytes = read_repo_file(repo_root, &path, MAX_BUILD_FILE_BYTES)?;
        let Ok(text) = std::str::from_utf8(&bytes) else {
            return go_mod_fail(&path, "not UTF-8");
        };
        pin_go_mod(&path, text, &module, runner)?;
    }
    Ok(())
}

/// Refuse a Go workspace file anywhere the `go` command could find one.
///
/// `go.work` (and its `go.work.sum`) is read from the working directory
/// upward, and its `replace` and `use` directives override every `go.mod`, so
/// a `go.work` in any ancestor can swap the runner module for another
/// directory while the pinned `go.mod` files still look right.  Every
/// directory from each of the two roots up to and including the repository
/// root is checked, and *any* entry with either name (a file, a directory, a
/// link) is refused: absence is the only accepted state.  The BUILD files also
/// run `go` with `GOWORK=off`; this check is the loader's own, since it cannot
/// see the environment the runner was started in.
fn check_go_workspace(
    repo_root: &Path,
    package_root: &str,
    runner_dir: &str,
) -> Result<(), LaneLoadError> {
    let mut checked: Vec<String> = Vec::new();
    for root in [package_root, runner_dir] {
        let mut dir = root.to_owned();
        loop {
            if !checked.contains(&dir) {
                checked.push(dir.clone());
                // The directory is reached with the usual rules (contained,
                // links refused per component) before its listing is trusted.
                if !dir.is_empty() {
                    contained_path(repo_root, &dir, Kind::Dir)?;
                }
                for name in ["go.work", "go.work.sum"] {
                    let rel = if dir.is_empty() {
                        name.to_owned()
                    } else {
                        format!("{dir}/{name}")
                    };
                    match fs::symlink_metadata(repo_root.join(&rel)) {
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return go_mod_fail(&rel, format!("cannot be inspected: {e}")),
                        Ok(_) => {
                            return go_mod_fail(
                                &rel,
                                "a Go workspace file can replace the runner module; none may exist in the package, the runner or any directory above them (VER04i 8.1)",
                            );
                        }
                    }
                }
            }
            match dir.rsplit_once('/') {
                Some((parent, _)) => dir = parent.to_owned(),
                None if !dir.is_empty() => dir = String::new(),
                None => break,
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Sourceless bytecode in a Python package (WD-004b2 review)
// ---------------------------------------------------------------------------
//
// The source digest walks only the binding's source root, so `tests/` is never
// examined.  A legacy sourceless `tests/shutil.pyc` is imported by the
// interpreter in preference to nothing at all, and (with the test directory on
// `sys.path`) in preference to the standard library module of the same name.
// So no `*.pyc` or `*.pyo` may exist under the package root unless it sits in
// a `__pycache__` directory, which the private `sys.pycache_prefix` makes
// irrelevant.  The same scan runs over the runner's package
// (`python/warrant-corpus`): its `tests/` is outside the digested runner root,
// and a `tests/shutil.pyc` there would shadow the standard library for the
// runner's own test run (WD-004b3).  The package-root `.venv` is the build's own environment, wiped
// and recreated by `uv venv --clear` on every build, and is not scanned.

fn check_python_bytecode(repo_root: &Path, package_root: &str) -> Result<(), LaneLoadError> {
    let (root, _) = contained_path(repo_root, package_root, Kind::Dir)?;
    let mut seen = 0usize;
    scan_bytecode(&root, package_root, 0, &mut seen)
}

fn scan_bytecode(
    dir: &Path,
    shown: &str,
    depth: usize,
    seen: &mut usize,
) -> Result<(), LaneLoadError> {
    if depth > MAX_SOURCE_DEPTH {
        return source_fail("package tree is nested too deeply");
    }
    let listing = fs::read_dir(dir).map_err(|e| LaneLoadError {
        code: "io",
        message: format!("cannot list {shown}: {e}"),
    })?;
    for entry in listing {
        let entry = entry.map_err(|e| LaneLoadError {
            code: "io",
            message: format!("cannot list {shown}: {e}"),
        })?;
        *seen += 1;
        if *seen > MAX_PACKAGE_ENTRIES {
            return source_fail(format!(
                "more than {MAX_PACKAGE_ENTRIES} entries in {shown}"
            ));
        }
        let Ok(name) = entry.file_name().into_string() else {
            return source_fail("non-UTF-8 file name in the package tree");
        };
        let path = format!("{shown}/{name}");
        let meta = fs::symlink_metadata(entry.path()).map_err(|e| LaneLoadError {
            code: "io",
            message: format!("{path}: {e}"),
        })?;
        if is_link(&meta) {
            return err(
                "symlink",
                format!("{path} is a symbolic link; links are refused"),
            );
        }
        if meta.is_dir() {
            let build_env = depth == 0 && name == ".venv";
            if name != "__pycache__" && !build_env {
                scan_bytecode(&entry.path(), &path, depth + 1, seen)?;
            }
        } else {
            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".pyc") || lower.ends_with(".pyo") {
                return err(
                    "uncovered-source",
                    format!("{path} is sourceless bytecode outside __pycache__: Python runs it in place of source, so none is accepted (VER04i 8.1)"),
                );
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Canonical JSON and the labels digest
// ---------------------------------------------------------------------------

/// Escape one JSON string per the canonical writer profile: every character
/// outside U+0020..U+007E becomes lowercase `\uXXXX` (an astral character is a
/// surrogate pair), `"` and `\` are backslash-escaped, `\b \f \n \r \t` use
/// their short forms, and `/` is never escaped.
fn escape_canonical(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ' '..='~' => out.push(c),
            other => {
                let mut units = [0_u16; 2];
                for unit in other.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
}

fn render_canonical(value: &JsonValue, depth: usize, out: &mut String) {
    let pad = |level: usize| "  ".repeat(level);
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JsonValue::Number(n) => {
            if n.fract() == 0.0 {
                out.push_str(&format!("{n:.0}"));
            } else {
                out.push_str(&format!("{n}"));
            }
        }
        JsonValue::String(s) => escape_canonical(s, out),
        JsonValue::Array(items) if items.is_empty() => out.push_str("[]"),
        JsonValue::Array(items) => {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                out.push_str(&pad(depth + 1));
                render_canonical(item, depth + 1, out);
                out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
            }
            out.push_str(&pad(depth));
            out.push(']');
        }
        JsonValue::Object(map) if map.is_empty() => out.push_str("{}"),
        JsonValue::Object(map) => {
            // A BTreeMap<String, _> iterates in UTF-8 byte order, which is
            // Unicode code point order: exactly the profile's key order.
            out.push_str("{\n");
            for (i, (key, item)) in map.iter().enumerate() {
                out.push_str(&pad(depth + 1));
                escape_canonical(key, out);
                out.push_str(": ");
                render_canonical(item, depth + 1, out);
                out.push_str(if i + 1 < map.len() { ",\n" } else { "\n" });
            }
            out.push_str(&pad(depth));
            out.push('}');
        }
    }
}

/// The canonical rendering of `value`: two-space indent, LF endings, one final
/// LF.  Identical across the Python, TypeScript and Go runners.
pub fn canonical_json(value: &JsonValue) -> String {
    let mut out = String::new();
    render_canonical(value, 0, &mut out);
    out.push('\n');
    out
}

/// The `manifest_sha256` a labels sidecar must carry: the SHA-256 of the
/// canonical rendering of the manifest with `toolchain.version` replaced by the
/// empty string.  `toolchain.name` stays, read from the manifest and never a
/// literal: a different name is a different digest, a different version is the
/// same digest (the version is diagnostic, not evidence).
pub fn labels_manifest_digest(manifest_bytes: &[u8]) -> Result<String, LaneLoadError> {
    let text = std::str::from_utf8(manifest_bytes).map_err(|_| LaneLoadError {
        code: "manifest",
        message: "manifest must be UTF-8".into(),
    })?;
    let mut value = JsonValue::parse(text).map_err(|e| LaneLoadError {
        code: "manifest",
        message: format!("invalid manifest JSON: {e}"),
    })?;
    let toolchain = match &mut value {
        JsonValue::Object(map) => match map.get_mut("toolchain") {
            Some(JsonValue::Object(toolchain)) => toolchain,
            _ => return err("manifest", "manifest has no toolchain object"),
        },
        _ => return err("manifest", "manifest must be an object"),
    };
    toolchain.insert("version".into(), JsonValue::String(String::new()));
    Ok(sha256_hex(canonical_json(&value).as_bytes()))
}

// ---------------------------------------------------------------------------
// The coverage-labels sidecar (`domain-labels.json`)
// ---------------------------------------------------------------------------

const LABELS_SCHEMA: &str = "warrant-lane-domain-labels-v1";

/// Coverage labels, weakest last: a suite is as weak as its weakest binder.
const LABEL_RANK: [&str; 3] = ["full", "bounded-set", "sample"];

/// One suite's coverage label, joined to the manifest's exhaustive row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuiteLabel {
    pub id: String,
    pub label: String,
    pub domain: String,
    pub cases: u64,
}

/// The parsed sidecar.  `cases_by_label` is the honest headline: the manifest's
/// own `exhaustive` total conflates `full`, `bounded-set` and `sample` domains
/// (backlog LP-012).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainLabels {
    pub package: String,
    pub module: String,
    pub manifest_sha256: String,
    pub cases_by_label: BTreeMap<String, u64>,
    pub suites: Vec<SuiteLabel>,
}

/// The label of one domain descriptor, as the runners derive it.  `Bytes<0>`
/// has one value and `Bytes<1>` has 256, so both enumerate their whole type;
/// every other descriptor is a fixed, strict subset.  `None` for a descriptor
/// this profile does not know: the loader cannot vouch for a label it cannot
/// re-derive.
fn descriptor_label(descriptor: &str) -> Option<&'static str> {
    match descriptor {
        "bool" | "u8_all" => Some("full"),
        "u64_edge" | "bytes_d1" | "nat_0_64" => Some("bounded-set"),
        other => {
            let digits = other.strip_prefix("bytes_n(")?.strip_suffix(')')?;
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            Some(if digits.parse::<u64>().ok()? <= 1 {
                "full"
            } else {
                "bounded-set"
            })
        }
    }
}

fn labels_fail<T>(message: impl Into<String>) -> Result<T, LaneLoadError> {
    err("labels", message)
}

type Obj = BTreeMap<String, JsonValue>;

fn closed<'a>(value: &'a JsonValue, keys: &[&str], what: &str) -> Result<&'a Obj, LaneLoadError> {
    let Some(map) = value.as_object() else {
        return labels_fail(format!("{what} must be an object"));
    };
    if map.len() != keys.len() || keys.iter().any(|k| !map.contains_key(*k)) {
        return labels_fail(format!("{what} fields do not match the closed schema"));
    }
    Ok(map)
}

fn text<'a>(map: &'a Obj, key: &str, what: &str) -> Result<&'a str, LaneLoadError> {
    match map.get(key).and_then(JsonValue::as_str) {
        Some(s) if !s.is_empty() => Ok(s),
        _ => labels_fail(format!("{what}.{key} must be a non-empty string")),
    }
}

fn count(map: &Obj, key: &str, what: &str) -> Result<u64, LaneLoadError> {
    match map.get(key).and_then(JsonValue::as_i64) {
        Some(n) if n >= 0 && (n as u64) <= (1 << 53) => Ok(n as u64),
        _ => labels_fail(format!(
            "{what}.{key} must be a bounded non-negative integer"
        )),
    }
}

fn known_label(label: &str, what: &str) -> Result<usize, LaneLoadError> {
    match LABEL_RANK.iter().position(|known| *known == label) {
        Some(rank) => Ok(rank),
        None => labels_fail(format!("{what} label is not full, bounded-set or sample")),
    }
}

/// Parse and self-check a sidecar: the closed schema, every binder label
/// re-derived from its descriptor, every suite label equal to its weakest
/// binder, and `cases_by_label` equal to the per-label sum of the suites.
/// Binding to a manifest is [`load_lane`]'s job.
pub fn parse_domain_labels(bytes: &[u8]) -> Result<DomainLabels, LaneLoadError> {
    let source = std::str::from_utf8(bytes).map_err(|_| LaneLoadError {
        code: "labels",
        message: "sidecar must be UTF-8".into(),
    })?;
    // The same strict number rules as the manifest (VER04i 8.2): canonical
    // non-negative integers within 2^53 only, so `65537.0`, `6.5537e4` and
    // `065537` are refused before the f64-based parser can round them.
    lift(
        "labels",
        validate_integer_tokens(bytes, "domain-labels sidecar"),
    )?;
    let root = JsonValue::parse(source).map_err(|e| LaneLoadError {
        code: "labels",
        message: format!("invalid sidecar JSON: {e}"),
    })?;
    let top = closed(
        &root,
        &[
            "schema",
            "package",
            "module",
            "manifest_sha256",
            "note",
            "cases_by_label",
            "suites",
        ],
        "sidecar",
    )?;
    if text(top, "schema", "sidecar")? != LABELS_SCHEMA {
        return labels_fail("unsupported sidecar schema");
    }
    let manifest_sha256 = text(top, "manifest_sha256", "sidecar")?;
    if manifest_sha256.len() != 64
        || !manifest_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return labels_fail("sidecar.manifest_sha256 must be lowercase SHA-256 hex");
    }
    text(top, "note", "sidecar")?;

    let Some(by_label) = top.get("cases_by_label").and_then(JsonValue::as_object) else {
        return labels_fail("sidecar.cases_by_label must be an object");
    };
    let mut declared = BTreeMap::new();
    for key in by_label.keys() {
        known_label(key, "cases_by_label")?;
        declared.insert(key.clone(), count(by_label, key, "cases_by_label")?);
    }

    let Some(rows) = top.get("suites").and_then(JsonValue::as_array) else {
        return labels_fail("sidecar.suites must be an array");
    };
    let mut suites: Vec<SuiteLabel> = Vec::with_capacity(rows.len());
    let mut summed: BTreeMap<String, u64> = BTreeMap::new();
    for row in rows {
        let suite = closed(row, &["id", "label", "domain", "cases", "binders"], "suite")?;
        let id = text(suite, "id", "suite")?;
        let label = text(suite, "label", "suite")?;
        let cases = count(suite, "cases", "suite")?;
        let mut weakest = 0;
        let Some(binders) = suite.get("binders").and_then(JsonValue::as_array) else {
            return labels_fail("suite.binders must be an array");
        };
        if binders.is_empty() {
            return labels_fail("suite.binders must not be empty");
        }
        for binder in binders {
            let binder = closed(binder, &["name", "domain", "label"], "binder")?;
            text(binder, "name", "binder")?;
            let descriptor = text(binder, "domain", "binder")?;
            let derived = descriptor_label(descriptor).ok_or_else(|| LaneLoadError {
                code: "labels",
                message: format!(
                    "unknown domain descriptor {descriptor:?}: no label can be derived"
                ),
            })?;
            if text(binder, "label", "binder")? != derived {
                return labels_fail(format!(
                    "binder label for descriptor {descriptor:?} is not the derived label {derived}"
                ));
            }
            weakest = weakest.max(known_label(derived, "binder")?);
        }
        if known_label(label, "suite")? != weakest {
            return labels_fail(format!(
                "suite {id} label is not its weakest binder's label"
            ));
        }
        let total = summed.entry(label.to_owned()).or_insert(0_u64);
        *total = total.checked_add(cases).ok_or_else(|| LaneLoadError {
            code: "labels",
            message: "cases_by_label sum overflows u64".into(),
        })?;
        suites.push(SuiteLabel {
            id: id.to_owned(),
            label: label.to_owned(),
            domain: text(suite, "domain", "suite")?.to_owned(),
            cases,
        });
    }
    if !suites.windows(2).all(|pair| pair[0].id < pair[1].id) {
        return labels_fail("sidecar suites must be unique and sorted by id");
    }
    if summed != declared {
        return labels_fail("cases_by_label is not the per-label sum of the suites");
    }
    Ok(DomainLabels {
        package: text(top, "package", "sidecar")?.to_owned(),
        module: text(top, "module", "sidecar")?.to_owned(),
        manifest_sha256: manifest_sha256.to_owned(),
        cases_by_label: declared,
        suites,
    })
}

// ---------------------------------------------------------------------------
// The loaded lane
// ---------------------------------------------------------------------------

/// The outcome of the static checks of one lane.  It exists only inside a
/// successful [`LoadedLane`]: a failed check is an `Err`, never a `false`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneReadiness {
    /// Names of the checks that passed, in [`READINESS_CHECKS`] order.
    pub passed: Vec<&'static str>,
    /// Always `false` today: the sandbox rung (VER04i 12) is not armed in any
    /// lane, and no lane is release-ready without it.
    pub release_ready: bool,
    /// Always `"absent"` (no sandbox is armed) until VER08b lands.
    pub sandbox: &'static str,
}

/// A lane whose artifacts opened safely and passed the named checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedLane {
    pub consumer: LaneConsumer,
    pub manifest: LaneEvidenceManifest,
    pub labels: DomainLabels,
    pub readiness: LaneReadiness,
}

impl LoadedLane {
    /// Human-readable readiness, ending with the honest limits.  WD-004b2 will
    /// put the same facts in a report; this text is the loader's own output.
    pub fn render_readiness(&self) -> String {
        let by_label: Vec<String> = self
            .labels
            .cases_by_label
            .iter()
            .map(|(label, cases)| format!("{label}={cases}"))
            .collect();
        format!(
            "lane: {package} (lane {lane}, toolchain {tool})\n\
             static checks passed: {checks}\n\
             cases_by_label: {labels}\n\
             release_ready: {ready}\n\
             sandbox: {sandbox} (not armed: no lane runs under a runtime jail yet)\n\
             This loader makes no claim about the authenticity of the source, the build or the \
             toolchain beyond the checks named above.\n\
             not checked: {unclaimed}\n",
            package = self.consumer.package,
            lane = self.manifest.lane,
            tool = self.manifest.toolchain_name,
            checks = self.readiness.passed.join(", "),
            labels = by_label.join(", "),
            ready = self.readiness.release_ready,
            sandbox = self.readiness.sandbox,
            unclaimed = UNCLAIMED.join(", "),
        )
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn sibling(path: &str, name: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, _)) => format!("{dir}/{name}"),
        None => name.to_owned(),
    }
}

fn read_registry(
    repo_root: &Path,
) -> Result<(Vec<u8>, crate::lane_evidence::LaneConsumerRegistry), LaneLoadError> {
    let path = sibling(CORPUS_FILE, "consumers.json");
    let bytes = read_repo_file(repo_root, &path, MAX_MANIFEST_BYTES)?;
    let registry = lift("registry", parse_consumer_registry(&bytes))?;
    Ok((bytes, registry))
}

/// The consumer registry, or `None` when the registry file does not exist at
/// all (an unknown lane universe, not an empty one).  A registry that exists
/// but is a link, too large or malformed is an error, never `None`: a hostile
/// checkout must not be able to hide its lanes by breaking the file.
pub fn load_registry(
    repo_root: &Path,
) -> Result<Option<crate::lane_evidence::LaneConsumerRegistry>, LaneLoadError> {
    let path = sibling(CORPUS_FILE, "consumers.json");
    match fs::symlink_metadata(repo_root.join(&path)) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        // Any other answer, including an error, goes through the safe reader,
        // which names the problem.
        _ => read_registry(repo_root).map(|(_, registry)| Some(registry)),
    }
}

/// Load one registered lane: open its artifacts safely, run the raw digest
/// checks, join the registry and corpus, and run the static readiness checks.
/// See the module documentation for the order.  `package` is `<lane>/<name>`
/// (for example `python/ct-compare`); a package the registry does not list,
/// including every Rust package, is `not-registered`.
pub fn load_lane(repo_root: &Path, package: &str) -> Result<LoadedLane, LaneLoadError> {
    // --- registry join -----------------------------------------------------
    let (registry_bytes, registry) = read_registry(repo_root)?;
    let consumer = registry
        .consumers
        .iter()
        .find(|row| row.package == package)
        .ok_or_else(|| LaneLoadError {
            code: "not-registered",
            message: format!("{package} is not in the consumer registry"),
        })?
        .clone();
    if consumer.status != "active" {
        return err(
            "not-active",
            format!(
                "{package} is {} in the registry; planned rows cannot authorize lane evidence",
                consumer.status
            ),
        );
    }

    // --- the artifacts, each bounded and contained --------------------------
    let spec = read_repo_file(repo_root, SPEC_FILE, MAX_SPEC_BYTES)?;
    let corpus = read_repo_file(repo_root, CORPUS_FILE, MAX_CORPUS_BYTES)?;
    let cases = read_repo_file(
        repo_root,
        &sibling(CORPUS_FILE, "cases.tfv"),
        MAX_CORPUS_BYTES,
    )?;
    let manifest_bytes =
        read_repo_file(repo_root, &consumer.evidence_manifest, MAX_MANIFEST_BYTES)?;
    let labels_path = sibling(&consumer.evidence_manifest, "domain-labels.json");
    let labels_bytes = read_repo_file(repo_root, &labels_path, MAX_MANIFEST_BYTES)?;
    let binding = read_repo_file(repo_root, &consumer.binding, MAX_BINDING_BYTES)?;

    // --- strict parse and identity -----------------------------------------
    let manifest = lift("manifest", parse_lane_manifest(&manifest_bytes))?;
    if manifest.package != consumer.package || manifest.lane != consumer.lane {
        return err(
            "manifest",
            format!(
                "manifest is for package {} but the registry row is {}",
                manifest.package, consumer.package
            ),
        );
    }

    // --- raw digest checks on exactly the bytes read ------------------------
    for (name, recorded, bytes) in [
        ("spec_sha256", &manifest.spec_sha256, &spec),
        ("corpus_sha256", &manifest.corpus_sha256, &corpus),
        ("cases_sha256", &manifest.cases_sha256, &cases),
    ] {
        if *recorded != sha256_hex(bytes) {
            return err("digest", format!("{name} does not match the file on disk"));
        }
    }
    // The WD-004a join: ids, counts, domains, stream digests, controls.
    lift(
        "join",
        validate_lane_evidence(&manifest_bytes, &registry_bytes, &spec, &corpus, &cases),
    )?;
    if manifest.binding_sha256 != sha256_hex(&binding) {
        return err(
            "digest",
            format!("binding_sha256 does not match {}", consumer.binding),
        );
    }

    // --- static readiness ----------------------------------------------------
    if manifest.runner_name != RUNNER_NAME || manifest.runner_lane != consumer.lane {
        return err(
            "runner-identity",
            format!("runner must be {RUNNER_NAME} for lane {}", consumer.lane),
        );
    }
    let labels = parse_domain_labels(&labels_bytes)?;
    bind_labels(&labels, &manifest, &manifest_bytes)?;
    // The runner entry must exist and open safely; its bytes are not judged.
    read_repo_file(repo_root, &consumer.runner_entry, MAX_RUNNER_ENTRY_BYTES)?;
    for build_file in &consumer.build_files {
        let bytes = read_repo_file(repo_root, build_file, MAX_BUILD_FILE_BYTES)?;
        check_build_routing(build_file, &bytes)?;
    }
    // The workspace and bytecode refusals come first: they are about files the
    // digest walks would otherwise report under a vaguer class.
    if consumer.lane == "go" {
        if let Some(runner_dir) = runner_root("go") {
            check_go_workspace(repo_root, &consumer.package_root, runner_dir)?;
        }
    }
    // Source freshness (VER04i 9 check 23): both digests recomputed from the tree.
    let source = binding_source(&binding)?;
    let source_root = if source == "." {
        consumer.package_root.clone()
    } else {
        format!("{}/{source}", consumer.package_root)
    };
    let source_digest = source_tree_digest(
        repo_root,
        &source_root,
        &consumer.lane,
        SourceKind::Production,
        manifest.pycache_isolated,
    )?;
    if source_digest != manifest.source_sha256 {
        return err(
            "stale-source",
            format!("source_sha256 does not match the recomputed digest of {source_root}"),
        );
    }
    let Some(runner_dir) = runner_root(&consumer.lane) else {
        return source_fail(format!("no source rule for lane {}", consumer.lane));
    };
    if consumer.lane == "go" {
        check_go_modules(repo_root, &consumer.package_root, runner_dir)?;
    }
    if consumer.lane == "python" {
        check_python_bytecode(repo_root, &consumer.package_root)?;
        // The runner is a package too, and the build installs and runs its tests
        // from the package root, so its `tests/` (outside the digested runner
        // root) gets the same scan (WD-004b3).
        if let Some(runner_package) = runner_package_root(&consumer.lane) {
            check_python_bytecode(repo_root, runner_package)?;
        }
    }
    let runner_digest = source_tree_digest(
        repo_root,
        runner_dir,
        &consumer.lane,
        SourceKind::Runner,
        manifest.pycache_isolated,
    )?;
    if runner_digest != manifest.runner_src_sha256 {
        return err(
            "stale-runner",
            format!("runner.src_sha256 does not match the recomputed digest of {runner_dir}"),
        );
    }

    Ok(LoadedLane {
        consumer,
        manifest,
        labels,
        readiness: LaneReadiness {
            passed: READINESS_CHECKS.to_vec(),
            release_ready: false,
            sandbox: "absent",
        },
    })
}

/// Load every `active` lane of the registry, in registry order.  Fail-closed:
/// the first lane that does not load is the error.  This is the hook WD-004b2
/// uses; it generates nothing.
pub fn load_active_lanes(repo_root: &Path) -> Result<Vec<LoadedLane>, LaneLoadError> {
    let (_, registry) = read_registry(repo_root)?;
    registry
        .consumers
        .iter()
        .filter(|row| row.status == "active")
        .map(|row| load_lane(repo_root, &row.package))
        .collect()
}

/// The sidecar describes *this* manifest: same identity, same digest (with the
/// toolchain version blanked), and the same exhaustive suites with the same
/// domains and case counts.
fn bind_labels(
    labels: &DomainLabels,
    manifest: &LaneEvidenceManifest,
    manifest_bytes: &[u8],
) -> Result<(), LaneLoadError> {
    if labels.package != manifest.package || labels.module != manifest.module {
        return labels_fail("sidecar package or module differs from the manifest");
    }
    if labels.manifest_sha256 != labels_manifest_digest(manifest_bytes)? {
        return labels_fail(
            "sidecar manifest_sha256 does not bind this manifest (toolchain.version excluded)",
        );
    }
    let rows: Vec<&ExhaustiveEntry> = manifest
        .tests
        .iter()
        .filter_map(|t| match t {
            LaneTestEntry::Exhaustive(row) => Some(row),
            _ => None,
        })
        .collect();
    let ids: BTreeSet<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    let label_ids: BTreeSet<&str> = labels.suites.iter().map(|s| s.id.as_str()).collect();
    if ids != label_ids || rows.len() != labels.suites.len() {
        return labels_fail("sidecar suites differ from the manifest's exhaustive rows");
    }
    for suite in &labels.suites {
        let row = rows
            .iter()
            .find(|r| r.id == suite.id)
            .expect("ids were just compared");
        if row.cases != suite.cases || row.domain != suite.domain {
            return labels_fail(format!(
                "suite {} cases or domain differ from the manifest",
                suite.id
            ));
        }
    }
    Ok(())
}

/// A static *text* check: the build file names the manifest it validates and
/// the sabotage step.  It shows the routing was written, not that it ran.
fn check_build_routing(path: &str, bytes: &[u8]) -> Result<(), LaneLoadError> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return err("build-routing", format!("{path} is not UTF-8 text"));
    };
    for needle in ["evidence/test-manifest.json", "sabotage"] {
        if !text.contains(needle) {
            return err("build-routing", format!("{path} does not mention {needle}"));
        }
    }
    Ok(())
}
