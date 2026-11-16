//! # The filesystem edge (binary only)
//!
//! The library is pure.  This module is the *only* code that touches the disk,
//! and it follows VOS02 section 6.8:
//!
//! * every path is `lstat`ed (`symlink_metadata`), never followed: a symlink
//!   anywhere under the crate is refused, because a link could point a
//!   "crate" at files outside the tree the reviewer looked at;
//! * only regular files are read, and a file over its cap is refused
//!   **before** it is read; the read itself is also bounded (`take`), so a
//!   file that grows between the check and the read cannot overrun the cap;
//! * the directory walk is bounded in depth and entry count;
//! * nothing is ever written.
//!
//! An [`Err`] here means "no verdict is possible" and the binary exits 2.

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use vos_shape_check::manifest::{CARGO_TOML_MAX_BYTES, VOS_TOML_MAX_BYTES};
use vos_shape_check::{CrateInput, SourceFile, MAX_SOURCE_BYTES, MAX_SOURCE_FILES};

/// Deepest directory nesting under `src/`.
const MAX_DEPTH: usize = 16;
/// Most entries in the crate root.
const MAX_ROOT_ENTRIES: usize = 4_096;

/// Read a regular file of at most `cap` bytes as UTF-8 text.
fn read_capped(path: &Path, cap: usize) -> Result<String, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.file_type().is_symlink() {
        return Err(format!("{}: is a symlink (refused)", path.display()));
    }
    if !meta.is_file() {
        return Err(format!("{}: is not a regular file", path.display()));
    }
    if meta.len() > cap as u64 {
        return Err(format!("{}: over the {cap}-byte cap", path.display()));
    }
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() > cap {
        return Err(format!(
            "{}: grew past the {cap}-byte cap while reading",
            path.display()
        ));
    }
    String::from_utf8(bytes).map_err(|_| format!("{}: not valid UTF-8", path.display()))
}

fn walk(dir: &Path, rel: &str, depth: usize, out: &mut Vec<SourceFile>) -> Result<(), String> {
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
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if meta.file_type().is_symlink() {
            return Err(format!("{}: is a symlink (refused)", path.display()));
        }
        let child = format!("{rel}/{name}");
        if meta.is_dir() {
            walk(&path, &child, depth + 1, out)?;
        } else if meta.is_file() && name.ends_with(".rs") {
            if out.len() >= MAX_SOURCE_FILES {
                return Err(format!("more than {MAX_SOURCE_FILES} source files"));
            }
            out.push(SourceFile {
                path: child,
                text: read_capped(&path, MAX_SOURCE_BYTES)?,
            });
        } else if !meta.is_file() {
            return Err(format!(
                "{}: is not a regular file or directory",
                path.display()
            ));
        }
    }
    Ok(())
}

/// Read the crate at `dir` into memory.
pub fn load_crate(dir: &Path) -> Result<CrateInput, String> {
    let meta = fs::symlink_metadata(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(format!("{}: is not a plain directory", dir.display()));
    }
    let mut root_entries: Vec<String> = Vec::new();
    for e in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let e = e.map_err(|e| format!("{}: {e}", dir.display()))?;
        root_entries.push(e.file_name().to_string_lossy().into_owned());
        if root_entries.len() > MAX_ROOT_ENTRIES {
            return Err(format!(
                "{}: more than {MAX_ROOT_ENTRIES} entries",
                dir.display()
            ));
        }
    }
    root_entries.sort();
    let has = |n: &str| root_entries.iter().any(|e| e == n);
    if !has("Cargo.toml") {
        return Err(format!("{}: no Cargo.toml", dir.display()));
    }
    let cargo_toml = Some(read_capped(&dir.join("Cargo.toml"), CARGO_TOML_MAX_BYTES)?);
    let vos_toml = if has("vos.toml") {
        Some(read_capped(&dir.join("vos.toml"), VOS_TOML_MAX_BYTES)?)
    } else {
        None
    };
    let mut sources = Vec::new();
    if has("src") {
        walk(&dir.join("src"), "src", 0, &mut sources)?;
    }
    Ok(CrateInput {
        cargo_toml,
        vos_toml,
        root_entries,
        sources,
    })
}
