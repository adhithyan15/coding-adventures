//! # Writing and checking a corpus directory (VER04r 1.1, 13: `--check`)
//!
//! This is the crate's only file-system code besides the CLI's reads (`cli.rs`).  The rules:
//!
//! ```text
//!   write   every path is validated BEFORE anything is written (path guard);
//!           chunks/ may hold nothing the corpus does not own (the generator never deletes
//!           your files: it refuses and says which);
//!           each file is written to a temporary name in its own directory and renamed
//!           into place, so a reader sees the old file or the new one, never half of one;
//!           corpus.json is renamed LAST: it is the commit point.
//!   check   regenerate, then compare byte for byte: corpus.json, cases.tfv, every chunk
//!           file, and the exact file set of chunks/.  Every difference is reported with
//!           its location (file, line, column, byte offset); content is never echoed.
//! ```
//!
//! ## The path guard
//!
//! A relative path is accepted only if it is `corpus.json`, `cases.tfv`, or `chunks/<name>`
//! where `<name>` is made of ASCII letters, digits, `_` and `.`, does not start with `.`,
//! has no `..`, is not empty and is at most 128 bytes and ends in `.txt`.  No separator,
//! no drive letter, no NUL: nothing derived from a document can leave the output directory.
//! Existing symbolic links at `chunks/` or at a target are refused, so a planted link cannot
//! redirect a write.  The output directory itself is whatever the caller names.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{code, GenError};
use crate::CorpusV2;

fn io(at: &str, what: &str) -> GenError {
    GenError::new(code::IO, at, what)
}

/// Is `rel` a path the corpus may own?  See the module documentation.
///
/// # Errors
///
/// `io` naming the rule.
pub fn guard(rel: &str) -> Result<(), GenError> {
    let ok_name = |n: &str, ext: bool| {
        !n.is_empty()
            && n.len() <= 128
            && !n.starts_with('.')
            && !n.contains("..")
            && n.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
            && (!ext || n.ends_with(".txt"))
    };
    let allowed = match rel {
        "corpus.json" | "cases.tfv" => true,
        _ => rel
            .strip_prefix("chunks/")
            .is_some_and(|n| ok_name(n, true)),
    };
    if allowed {
        Ok(())
    } else {
        Err(io(
            "path",
            "refused: a corpus file is corpus.json, cases.tfv or chunks/<name>.txt with a plain name",
        ))
    }
}

fn is_symlink(p: &Path) -> bool {
    fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink())
}

fn chunk_dir_extras(dir: &Path, owned: &[String]) -> Result<Vec<String>, GenError> {
    let mut extras = Vec::new();
    let chunks = dir.join("chunks");
    if !chunks.exists() {
        return Ok(extras);
    }
    let rd = fs::read_dir(&chunks).map_err(|_| io("chunks", "the chunks directory cannot be listed"))?;
    for entry in rd {
        let entry = entry.map_err(|_| io("chunks", "a chunks entry cannot be read"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !owned.contains(&name) {
            extras.push(name);
        }
    }
    extras.sort();
    Ok(extras)
}

fn owned_names(c: &CorpusV2) -> Vec<String> {
    c.chunk_files.iter().map(|(n, _)| n.clone()).collect()
}

/// Write `c` into `out` (created if missing).
///
/// # Errors
///
/// `io` for a refused path, a symlink in the way, files in `chunks/` the corpus does not own
/// (nothing is deleted), or a failed write.
pub fn write_corpus(out: &Path, c: &CorpusV2) -> Result<(), GenError> {
    let files = c.files();
    for (rel, _) in &files {
        guard(rel)?;
    }
    let mut seen: Vec<String> = Vec::new();
    for (rel, _) in &files {
        if seen.contains(rel) {
            return Err(io("path", "two corpus files have the same path"));
        }
        seen.push(rel.clone());
    }
    let chunks = out.join("chunks");
    if is_symlink(&chunks) {
        return Err(io("chunks", "chunks is a symbolic link: refused"));
    }
    let extras = chunk_dir_extras(out, &owned_names(c))?;
    if !extras.is_empty() {
        return Err(io(
            "chunks",
            &format!(
                "chunks/ holds {} file(s) the corpus does not own (first: `{}`); remove them yourself",
                extras.len(),
                extras[0]
            ),
        ));
    }
    fs::create_dir_all(&chunks).map_err(|_| io("chunks", "the output directory cannot be created"))?;
    // chunk files, then cases.tfv, then corpus.json (the commit point)
    let mut order: Vec<&(String, &str)> = files.iter().skip(2).collect();
    order.push(&files[1]);
    order.push(&files[0]);
    for (rel, text) in order {
        write_atomic(&out.join(rel), rel, text.as_bytes())?;
    }
    Ok(())
}

fn write_atomic(target: &Path, rel: &str, bytes: &[u8]) -> Result<(), GenError> {
    if is_symlink(target) {
        return Err(io(rel, "the target is a symbolic link: refused"));
    }
    let dir = target.parent().ok_or_else(|| io(rel, "no parent directory"))?;
    let name = target
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io(rel, "no file name"))?;
    // One fixed temporary name per target; a stale one from a crashed run is overwritten, and a
    // symbolic link planted at that name is refused like any other.  Two generators writing
    // the same directory at once is not supported.
    let tmp: PathBuf = dir.join(format!(".{name}.tmp"));
    if is_symlink(&tmp) {
        return Err(io(rel, "the temporary file is a symbolic link: refused"));
    }
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .map_err(|_| io(rel, "the temporary file cannot be created"))?;
        f.write_all(bytes)
            .and_then(|()| f.sync_all())
            .map_err(|_| io(rel, "the temporary file cannot be written"))?;
        drop(f);
        fs::rename(&tmp, target).map_err(|_| io(rel, "the file cannot be moved into place"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Where two texts first differ: `(byte offset, line, column)`, 1-based line and column
/// (a column counts bytes), or `None` if equal.
pub fn first_difference(a: &[u8], b: &[u8]) -> Option<(usize, usize, usize)> {
    let n = a.len().min(b.len());
    let at = (0..n).find(|&i| a[i] != b[i]).or((a.len() != b.len()).then_some(n))?;
    let line = 1 + a[..at.min(a.len())].iter().filter(|&&x| x == b'\n').count();
    let line_start = a[..at.min(a.len())]
        .iter()
        .rposition(|&x| x == b'\n')
        .map_or(0, |p| p + 1);
    Some((at, line, at - line_start + 1))
}

/// Compare `out` with `c` byte for byte.
///
/// # Errors
///
/// `drift`, whose message lists every difference (missing file, extra file in `chunks/`,
/// differing bytes with location), at most 20 of them.
pub fn check_corpus(out: &Path, c: &CorpusV2) -> Result<(), GenError> {
    let mut findings: Vec<String> = Vec::new();
    for (rel, text) in c.files() {
        guard(&rel)?;
        let path = out.join(&rel);
        if is_symlink(&path) {
            findings.push(format!("{rel}: is a symbolic link"));
            continue;
        }
        match fs::read(&path) {
            Err(_) => findings.push(format!("{rel}: missing or unreadable")),
            Ok(have) => {
                if let Some((off, line, col)) = first_difference(text.as_bytes(), &have) {
                    findings.push(format!(
                        "{rel}: differs at line {line}, column {col} (byte {off}); expected {} bytes, found {}",
                        text.len(),
                        have.len()
                    ));
                }
            }
        }
    }
    for extra in chunk_dir_extras(out, &owned_names(c))? {
        findings.push(format!("chunks/{extra}: extra file the corpus does not own"));
    }
    if findings.is_empty() {
        return Ok(());
    }
    let total = findings.len();
    findings.truncate(20);
    let mut msg = findings.join("; ");
    if total > 20 {
        msg.push_str(&format!("; and {} more", total - 20));
    }
    Err(GenError::new(code::DRIFT, "corpus", msg))
}
