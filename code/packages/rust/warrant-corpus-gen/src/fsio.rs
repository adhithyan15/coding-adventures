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
//! has no `..`, is not empty, ends in `.txt` and is at most 133 bytes (two 64-character claim
//! names, the `/` that becomes `.`, and `.txt`).  No separator, no drive letter, no NUL:
//! nothing derived from a document can leave the output directory.
//!
//! ## Links, FIFOs and what is *not* promised
//!
//! * A symbolic link **present at the start** at `chunks/` or at a target is refused in both
//!   modes.  The output directory itself may be a link: it is the caller's own choice.
//! * The temporary file is created with `create_new` (O_EXCL) after removing any file of the
//!   same reserved name (`.<name>.tmp`): O_EXCL never follows a link, never reuses an inode
//!   (a planted hard link is unlinked, not written through) and never opens a FIFO.
//! * `--check` reads only regular files, through a handle, and never more than one byte past
//!   the expected length, so a FIFO or an oversized file is a finding, not a hang or an
//!   allocation.
//! * **Concurrent modification of the output directory is unsupported**: between a check and
//!   the use of a path another process could still swap an entry.  This is a developer tool
//!   writing a directory the developer owns, not a service over a hostile directory.
//! * File names found on disk are never echoed raw: they are escaped (`escape_default`) so a
//!   hostile name cannot inject terminal escapes or CI workflow commands into the report.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::error::{code, GenError};
use crate::CorpusV2;

/// The longest chunk file name: `<64>.<64>.txt`.
pub const MAX_NAME: usize = 133;

fn esc(name: &str) -> String {
    name.escape_default().to_string()
}

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
            && n.len() <= MAX_NAME
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

/// Read the regular file at `path`, at most `cap + 1` bytes, through a handle.  Returns the
/// bytes read and the file's length.  A symbolic link, a FIFO, a directory or anything else
/// that is not a regular file is an error (`io`) and is never opened for reading.
///
/// # Errors
///
/// `io` for a missing or irregular file.
pub fn read_regular(path: &Path, cap: u64, follow: bool) -> Result<(Vec<u8>, u64), GenError> {
    let meta = if follow {
        fs::metadata(path)
    } else {
        fs::symlink_metadata(path)
    }
    .map_err(|_| io("file", "missing or unreadable"))?;
    if !meta.is_file() {
        return Err(io("file", "is not a regular file"));
    }
    let f = fs::File::open(path).map_err(|_| io("file", "missing or unreadable"))?;
    let len = f
        .metadata()
        .map_err(|_| io("file", "missing or unreadable"))?
        .len();
    let mut buf = Vec::new();
    f.take(cap.saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(|_| io("file", "missing or unreadable"))?;
    Ok((buf, len))
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
            extras.push(esc(&name));
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
    // a leftover temporary of ours from an interrupted run is not "someone else's file": it is
    // overwritten (or refused if it is a symbolic link) when its own file is written
    let mut known = owned_names(c);
    known.extend(owned_names(c).iter().map(|n| format!(".{n}.tmp")));
    let extras = chunk_dir_extras(out, &known)?;
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
    if is_symlink(&chunks) {
        return Err(io("chunks", "chunks is a symbolic link: refused"));
    }
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
    // One fixed reserved temporary name per target.  Whatever sits there (a stale file of ours,
    // a planted link, a hard link to someone else's file) is unlinked first, then the file is
    // created with O_EXCL: it never follows a link, never reuses an inode and never opens a
    // FIFO.  Two generators writing the same directory at once is not supported.
    let tmp: PathBuf = dir.join(format!(".{name}.tmp"));
    let _ = fs::remove_file(&tmp);
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
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
    let linked_chunks = is_symlink(&out.join("chunks"));
    if linked_chunks {
        findings.push("chunks: is a symbolic link".to_string());
    }
    for (rel, text) in c.files() {
        guard(&rel)?;
        if linked_chunks && rel.starts_with("chunks/") {
            continue;
        }
        let path = out.join(&rel);
        if is_symlink(&path) {
            findings.push(format!("{rel}: is a symbolic link"));
            continue;
        }
        match read_regular(&path, text.len() as u64, false) {
            Err(e) => findings.push(format!("{rel}: {}", e.message)),
            Ok((have, len)) => {
                if let Some((off, line, col)) = first_difference(text.as_bytes(), &have) {
                    findings.push(format!(
                        "{rel}: differs at line {line}, column {col} (byte {off}); expected {} bytes, found {len}",
                        text.len(),
                    ));
                }
            }
        }
    }
    if !linked_chunks {
        for extra in chunk_dir_extras(out, &owned_names(c))? {
            findings.push(format!("chunks/{extra}: extra file the corpus does not own"));
        }
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
