//! # The filesystem edge
//!
//! The checking core is pure (text in, [`crate::Report`] out).  This module is the
//! *only* code that touches the disk, and it follows VOS02 section 6.8.  It is a
//! library module (not private to the binary) since VOS-039b-2a, because
//! `vos-link check-contract` reads a contract crate's `src/` with exactly the same
//! hardened walk instead of writing a second, weaker one:
//!
//! * every path is `lstat`ed, never followed: a symlink anywhere under the
//!   crate is refused;
//! * a file is **opened without following links** (`O_NOFOLLOW`) and without
//!   blocking (`O_NONBLOCK`, so a FIFO swapped in after the check cannot hang
//!   the open), and then the OPEN handle is checked: it must be a regular
//!   file and (on Unix) the very `dev`/`ino` the `lstat` saw, so a swap between
//!   the check and the open is caught, not trusted;
//! * a file over its cap is refused **before** it is read, and the read itself
//!   is bounded (`take`);
//! * the walk is bounded in depth, in entries per directory, in entries in
//!   total and in total bytes, all enforced DURING the walk;
//! * on a case-insensitive filesystem `Lib.RS` or `Build.rs` would be compiled
//!   or run while a case-sensitive name check misses it, so an entry whose
//!   lower-cased name ends `.rs` but whose exact name does not, and two entries
//!   that differ only in case, are refused (no verdict);
//! * nothing is ever written.
//!
//! Non-Unix platforms have no `O_NOFOLLOW` here; they rely on the `lstat`
//! check plus the open-handle `is_file` check (documented in VOS03).
//!
//! An [`Err`] here means "no verdict is possible" and the binary exits 2.

use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::Path;

use crate::manifest::{CARGO_TOML_MAX_BYTES, VOS_TOML_MAX_BYTES};
use crate::{CrateInput, SourceFile, MAX_SOURCE_BYTES, MAX_SOURCE_FILES, MAX_TOTAL_BYTES};

/// Deepest directory nesting under `src/`.
const MAX_DEPTH: usize = 16;
/// Most entries in one directory.
const MAX_DIR_ENTRIES: usize = 4_096;
/// Most entries seen in the whole walk.
const MAX_TOTAL_ENTRIES: usize = 16_384;

/// `O_NOFOLLOW | O_NONBLOCK`, from an EXPLICIT per-target table.  The values
/// are not portable (Linux alone has two `O_NOFOLLOW` values by architecture),
/// so a target that is not listed is **refused at runtime** (`None`): a wrong
/// constant would silently open the file with some other flag, and `0` would
/// silently turn the protection off.  Add a target only after checking its
/// `<fcntl.h>`.
///
/// | target | `O_NOFOLLOW` | `O_NONBLOCK` |
/// |---|---|---|
/// | macOS, iOS, FreeBSD, DragonFly, NetBSD, OpenBSD | `0x100` | `0x4` |
/// | Linux x86, x86_64 | `0o400000` | `0o4000` |
/// | Linux aarch64, arm, powerpc, powerpc64 | `0o100000` | `0o4000` |
/// | any other Linux (riscv64, loongarch64, s390x, mips, ...) | not verified: refused | |
#[cfg(all(
    unix,
    any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    )
))]
const OPEN_FLAGS: Option<i32> = Some(0x100 | 0x4);
#[cfg(all(
    unix,
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64")
))]
const OPEN_FLAGS: Option<i32> = Some(0o400000 | 0o4000);
#[cfg(all(
    unix,
    target_os = "linux",
    any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "powerpc",
        target_arch = "powerpc64"
    )
))]
const OPEN_FLAGS: Option<i32> = Some(0o100000 | 0o4000);
#[cfg(all(
    unix,
    not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd",
        all(
            target_os = "linux",
            any(
                target_arch = "x86",
                target_arch = "x86_64",
                target_arch = "aarch64",
                target_arch = "arm",
                target_arch = "powerpc",
                target_arch = "powerpc64"
            )
        )
    ))
))]
const OPEN_FLAGS: Option<i32> = None;

/// What the walk has consumed so far.
#[derive(Default)]
struct Budget {
    bytes: usize,
    entries: usize,
}

fn open_no_follow(path: &Path) -> std::io::Result<File> {
    let mut o = OpenOptions::new();
    o.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let Some(flags) = OPEN_FLAGS else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "this platform has no verified O_NOFOLLOW/O_NONBLOCK constants; refused (see loader.rs)",
            ));
        };
        o.custom_flags(flags);
    }
    o.open(path)
}

/// Read a regular file of at most `cap` bytes as UTF-8 text.
fn read_capped(path: &Path, cap: usize, budget: &mut Budget) -> Result<String, String> {
    let shown = path.display();
    let before = fs::symlink_metadata(path).map_err(|e| format!("{shown}: {e}"))?;
    if before.file_type().is_symlink() {
        return Err(format!("{shown}: is a symlink (refused)"));
    }
    if !before.is_file() {
        return Err(format!("{shown}: is not a regular file"));
    }
    if before.len() > cap as u64 {
        return Err(format!("{shown}: over the {cap}-byte cap"));
    }
    let file = open_no_follow(path).map_err(|e| format!("{shown}: {e}"))?;
    let handle = file.metadata().map_err(|e| format!("{shown}: {e}"))?;
    if !handle.is_file() {
        return Err(format!("{shown}: the opened handle is not a regular file"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if handle.dev() != before.dev() || handle.ino() != before.ino() {
            return Err(format!(
                "{shown}: changed between the check and the open (refused)"
            ));
        }
    }
    let mut bytes = Vec::new();
    file.take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{shown}: {e}"))?;
    if bytes.len() > cap {
        return Err(format!(
            "{shown}: grew past the {cap}-byte cap while reading"
        ));
    }
    budget.bytes += bytes.len();
    if budget.bytes > MAX_TOTAL_BYTES {
        return Err(format!(
            "{shown}: total source size over {MAX_TOTAL_BYTES} bytes"
        ));
    }
    String::from_utf8(bytes).map_err(|_| format!("{shown}: not valid UTF-8"))
}

/// The entries of `dir`, bounded, sorted by name, and checked for case traps.
fn list_dir(dir: &Path, budget: &mut Budget) -> Result<Vec<fs::DirEntry>, String> {
    let shown = dir.display();
    let mut entries: Vec<fs::DirEntry> = Vec::new();
    for e in fs::read_dir(dir).map_err(|e| format!("{shown}: {e}"))? {
        crate::steps::tick();
        entries.push(e.map_err(|e| format!("{shown}: {e}"))?);
        budget.entries += 1;
        if entries.len() > MAX_DIR_ENTRIES || budget.entries > MAX_TOTAL_ENTRIES {
            return Err(format!("{shown}: too many entries (caps {MAX_DIR_ENTRIES} per directory, {MAX_TOTAL_ENTRIES} in all)"));
        }
    }
    entries.sort_by_key(fs::DirEntry::file_name);
    let mut folded: Vec<String> = Vec::with_capacity(entries.len());
    for e in &entries {
        crate::steps::tick();
        let name = e.file_name().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".rs") && !name.ends_with(".rs") {
            return Err(format!("{shown}/{name}: a `.rs` file spelled with other case (a case-insensitive filesystem would compile it; refused)"));
        }
        folded.push(lower);
    }
    folded.sort();
    if let Some(w) = folded.windows(2).find(|w| w[0] == w[1]) {
        return Err(format!(
            "{shown}: two entries differ only in case (`{}`); refused",
            w[0]
        ));
    }
    Ok(entries)
}

fn walk(
    dir: &Path,
    rel: &str,
    depth: usize,
    budget: &mut Budget,
    out: &mut Vec<SourceFile>,
) -> Result<(), String> {
    let top = fs::symlink_metadata(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if top.file_type().is_symlink() || !top.is_dir() {
        return Err(format!("{}: is not a plain directory", dir.display()));
    }
    if depth > MAX_DEPTH {
        return Err(format!(
            "{}: directory nesting over {MAX_DEPTH}",
            dir.display()
        ));
    }
    for e in list_dir(dir, budget)? {
        crate::steps::tick();
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if meta.file_type().is_symlink() {
            return Err(format!("{}: is a symlink (refused)", path.display()));
        }
        let child = format!("{rel}/{name}");
        if meta.is_dir() {
            walk(&path, &child, depth + 1, budget, out)?;
        } else if meta.is_file() && name.ends_with(".rs") {
            if out.len() >= MAX_SOURCE_FILES {
                return Err(format!("more than {MAX_SOURCE_FILES} source files"));
            }
            let text = read_capped(&path, MAX_SOURCE_BYTES, budget)?;
            out.push(SourceFile { path: child, text });
        } else if !meta.is_file() {
            return Err(format!(
                "{}: is not a regular file or directory",
                path.display()
            ));
        }
    }
    Ok(())
}

/// Read only the `*.rs` files under `<dir>/src` (in a deterministic order), with every
/// protection of [`load_crate`] but without requiring a `Cargo.toml`.
///
/// A crate with no `src/` directory has no sources: the result is empty, which the
/// caller must judge (a contract crate with no `src` has no enums to compare).
pub fn load_sources(dir: &Path) -> Result<Vec<SourceFile>, String> {
    let meta = fs::symlink_metadata(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(format!("{}: is not a plain directory", dir.display()));
    }
    let mut budget = Budget::default();
    let mut sources = Vec::new();
    let has_src = list_dir(dir, &mut budget)?
        .iter()
        .any(|e| e.file_name() == "src");
    if has_src {
        walk(&dir.join("src"), "src", 0, &mut budget, &mut sources)?;
    }
    Ok(sources)
}

/// Read the crate at `dir` into memory.
pub fn load_crate(dir: &Path) -> Result<CrateInput, String> {
    let meta = fs::symlink_metadata(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(format!("{}: is not a plain directory", dir.display()));
    }
    let mut budget = Budget::default();
    let root_entries: Vec<String> = list_dir(dir, &mut budget)?
        .iter()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let has = |n: &str| root_entries.iter().any(|e| e == n);
    if !has("Cargo.toml") {
        return Err(format!("{}: no Cargo.toml", dir.display()));
    }
    let cargo_toml = Some(read_capped(
        &dir.join("Cargo.toml"),
        CARGO_TOML_MAX_BYTES,
        &mut budget,
    )?);
    let vos_toml = if has("vos.toml") {
        Some(read_capped(
            &dir.join("vos.toml"),
            VOS_TOML_MAX_BYTES,
            &mut budget,
        )?)
    } else {
        None
    };
    let mut sources = Vec::new();
    if has("src") {
        walk(&dir.join("src"), "src", 0, &mut budget, &mut sources)?;
    }
    Ok(CrateInput {
        cargo_toml,
        vos_toml,
        root_entries,
        sources,
    })
}
