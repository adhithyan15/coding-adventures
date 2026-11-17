//! # The filesystem edge (VOS02 section 6.8)
//!
//! The only code in this program that touches the disk.  Rules, each with the attack it
//! answers:
//!
//! * **Paths have no `..` and no symlink.**  A path from the command line is made
//!   absolute against the *canonical* working directory, refused if it has a `..`
//!   component, and refused unless canonicalising it gives it back unchanged (so no
//!   component is a symlink).  *A symlinked crate or `code/specs` directory could make
//!   the tool read or write somewhere the repository's review never looked.*
//! * **A file is lstat'ed, never followed, then opened with `O_NOFOLLOW | O_NONBLOCK`,
//!   and the OPEN handle is checked**: it must be a regular file and, on Unix, the very
//!   `dev`/`ino` the lstat saw.  *A swap between the check and the open is caught, and a
//!   FIFO swapped in cannot hang the open.*
//! * **A file over its cap is refused before it is read**, and the read itself is
//!   bounded with `take`.  *A growing file cannot exhaust memory.*
//! * **The one write goes through a fresh file** (`create_new`, so an existing symlink or
//!   file at the temporary name fails) and a rename, and a symlink or non-regular file
//!   already at the target is refused first.
//!
//! The `O_NOFOLLOW` constants are the table of `vos-shape-check`'s loader (it keeps them
//! in its binary, so they are repeated here; backlog VOS-D059): a target not in the table
//! is refused at run time rather than guessed.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use crate::error::NoVerdict;

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

/// Most entries read from one directory listing.
pub const MAX_DIR_ENTRIES: usize = 64;

/// Quote an untrusted string for a report: escapes ESC, newlines and every other
/// non-printable (Rust `Debug` form), so a hostile file name or `vos.toml` value cannot
/// inject terminal control sequences or forge a report line (backlog S-L1).
pub fn esc(s: &str) -> String {
    format!("{s:?}")
}

/// A path, quoted and escaped for a report.
pub fn q(p: impl AsRef<Path>) -> String {
    format!("{:?}", p.as_ref())
}

fn io_err(what: &str, path: &Path, e: &std::io::Error) -> NoVerdict {
    NoVerdict::new(format!("{what} {}: {e}", q(path)))
}

/// Drop the Windows verbatim prefix that `canonicalize` adds (`\\?\C:\x` is `C:\x`,
/// `\\?\UNC\h\s` is `\\h\s`), so a canonical path can be compared with the path the user
/// typed.  Pure string work, so it is tested on every platform.
pub fn strip_verbatim(s: &str) -> String {
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        s.to_string()
    }
}

/// Are two paths the same, as written?  Windows is case-insensitive about the drive
/// letter and everything after it, and separators may differ; Unix is exact.
pub fn same_path_text(a: &str, b: &str, windows: bool) -> bool {
    if windows {
        let norm = |x: &str| strip_verbatim(x).replace('/', "\\").to_lowercase();
        norm(a) == norm(b)
    } else {
        a == b
    }
}

#[cfg(unix)]
fn open_no_follow(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    let Some(flags) = OPEN_FLAGS else {
        return Err(std::io::Error::other(
            "O_NOFOLLOW is not verified for this target; refusing to guess",
        ));
    };
    OpenOptions::new().read(true).custom_flags(flags).open(path)
}

#[cfg(not(unix))]
fn open_no_follow(path: &Path) -> std::io::Result<File> {
    // No O_NOFOLLOW here: the lstat check and the open-handle `is_file` check carry it.
    OpenOptions::new().read(true).open(path)
}

#[cfg(unix)]
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev() && a.ino() == b.ino()
}

#[cfg(not(unix))]
fn same_file(_a: &fs::Metadata, _b: &fs::Metadata) -> bool {
    true
}

/// Does the path text have a `..` component?  Pure string work, so it is tested on every
/// platform.
///
/// `Path::components` is not enough on Windows: for a *verbatim* path (`\\?\C:\x\..\y`,
/// which is what `canonicalize` returns and so what any test or caller that starts from a
/// canonical directory holds) std deliberately does NOT interpret `.` or `..`, so they
/// come back as `Normal` components and a `ParentDir` check never fires.  So split the
/// text ourselves: on Windows both `/` and `\` separate components (the verbatim form
/// excepted by std, but refusing is the safe side), on Unix only `/` does.
pub fn has_parent_component(s: &str, windows: bool) -> bool {
    s.split(|c| c == '/' || (windows && c == '\\'))
        .any(|part| part == "..")
}

/// Make a command-line path absolute, with no `..` and no symlink in it.
pub fn absolute_plain(arg: &Path) -> Result<PathBuf, NoVerdict> {
    if arg.as_os_str().is_empty() {
        return Err(NoVerdict::new("an empty path"));
    }
    if arg.components().any(|c| matches!(c, Component::ParentDir))
        || has_parent_component(&arg.to_string_lossy(), cfg!(windows))
    {
        return Err(NoVerdict::new(format!(
            "{}: `..` is not allowed in a path",
            q(arg)
        )));
    }
    let abs = if arg.is_absolute() {
        arg.to_path_buf()
    } else {
        let cwd = std::env::current_dir().map_err(|e| NoVerdict::new(format!("cwd: {e}")))?;
        let cwd = cwd
            .canonicalize()
            .map_err(|e| NoVerdict::new(format!("cwd: {e}")))?;
        cwd.join(arg)
    };
    // Lexically normalise `.` away.
    let mut clean = PathBuf::new();
    for c in abs.components() {
        if !matches!(c, Component::CurDir) {
            clean.push(c.as_os_str());
        }
    }
    let canon = clean
        .canonicalize()
        .map_err(|e| io_err("cannot resolve", &clean, &e))?;
    if !same_path_text(
        &canon.to_string_lossy(),
        &clean.to_string_lossy(),
        cfg!(windows),
    ) {
        return Err(NoVerdict::new(format!(
            "{}: a component is a symlink (it resolves to {})",
            q(&clean),
            q(&canon)
        )));
    }
    Ok(clean)
}

/// Require that `root/rel[0]/rel[1]/..` exists, one directory at a time, and that no
/// component is a symlink.
pub fn ensure_plain_dirs(root: &Path, rel: &[&str]) -> Result<PathBuf, NoVerdict> {
    let mut p = root.to_path_buf();
    for part in rel {
        p.push(part);
        let md = fs::symlink_metadata(&p).map_err(|e| io_err("cannot stat", &p, &e))?;
        if md.file_type().is_symlink() {
            return Err(NoVerdict::new(format!("{}: is a symlink", q(p))));
        }
        if !md.is_dir() {
            return Err(NoVerdict::new(format!("{}: is not a directory", q(p))));
        }
    }
    Ok(p)
}

/// Read a regular file of at most `cap` bytes.  `Ok(None)` when it does not exist.
pub fn read_regular_opt(path: &Path, cap: usize) -> Result<Option<Vec<u8>>, NoVerdict> {
    let before = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_err("cannot stat", path, &e)),
    };
    if before.file_type().is_symlink() {
        return Err(NoVerdict::new(format!("{}: is a symlink", q(path))));
    }
    if !before.is_file() {
        return Err(NoVerdict::new(format!(
            "{}: is not a regular file",
            q(path)
        )));
    }
    if before.len() > cap as u64 {
        return Err(NoVerdict::new(format!(
            "{}: {} bytes is over the {cap}-byte cap",
            q(path),
            before.len()
        )));
    }
    let file = open_no_follow(path).map_err(|e| io_err("cannot open", path, &e))?;
    let after = file
        .metadata()
        .map_err(|e| io_err("cannot stat", path, &e))?;
    if !after.is_file() || !same_file(&before, &after) {
        return Err(NoVerdict::new(format!(
            "{}: changed while it was being opened",
            q(path)
        )));
    }
    let mut bytes = Vec::new();
    file.take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| io_err("cannot read", path, &e))?;
    if bytes.len() > cap {
        return Err(NoVerdict::new(format!(
            "{}: grew past the {cap}-byte cap while it was read",
            q(path)
        )));
    }
    Ok(Some(bytes))
}

/// Read a regular file that must exist.
pub fn read_regular(path: &Path, cap: usize) -> Result<Vec<u8>, NoVerdict> {
    read_regular_opt(path, cap)?.ok_or_else(|| NoVerdict::new(format!("{}: not found", q(path))))
}

/// The entry names of a directory (UTF-8, at most [`MAX_DIR_ENTRIES`]).  A directory
/// that does not exist lists as empty; a symlink, or something that is not a directory,
/// is refused.
pub fn list_names(dir: &Path) -> Result<Vec<String>, NoVerdict> {
    match fs::symlink_metadata(dir) {
        Ok(m) => {
            if m.file_type().is_symlink() {
                return Err(NoVerdict::new(format!("{}: is a symlink", q(dir))));
            }
            if !m.is_dir() {
                return Err(NoVerdict::new(format!("{}: is not a directory", q(dir))));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_err("cannot stat", dir, &e)),
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| io_err("cannot list", dir, &e))? {
        let entry = entry.map_err(|e| io_err("cannot list", dir, &e))?;
        if names.len() >= MAX_DIR_ENTRIES {
            return Err(NoVerdict::new(format!(
                "{}: more than {MAX_DIR_ENTRIES} entries",
                q(dir)
            )));
        }
        let Ok(name) = entry.file_name().into_string() else {
            return Err(NoVerdict::new(format!(
                "{}: a file name is not UTF-8",
                q(dir)
            )));
        };
        names.push(name);
    }
    names.sort();
    Ok(names)
}

/// Write `bytes` to `dir/name` through a fresh temporary file and a rename.  Refuses a
/// symlink or a non-regular file already at the target, and a stale temporary.
pub fn write_replacing(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), NoVerdict> {
    let target = dir.join(name);
    match fs::symlink_metadata(&target) {
        Ok(m) => {
            if m.file_type().is_symlink() {
                return Err(NoVerdict::new(format!(
                    "{}: the output path is a symlink",
                    q(target)
                )));
            }
            if !m.is_file() {
                return Err(NoVerdict::new(format!(
                    "{}: the output path is not a regular file",
                    q(target)
                )));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_err("cannot stat", &target, &e)),
    }
    // A unique name (process id plus a counter), still created with `create_new`, so a
    // stale file left by a crashed run never blocks the next one and a planted file or
    // symlink at the chosen name fails the create rather than being written through.
    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let tmp = dir.join(format!(".{name}.{}.{n}.vos-link.tmp", std::process::id()));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| io_err("cannot create the temporary file", &tmp, &e))?;
    let written = f.write_all(bytes).and_then(|()| f.sync_all());
    drop(f);
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(io_err("cannot write", &tmp, &e));
    }
    if let Err(e) = fs::rename(&tmp, &target) {
        let _ = fs::remove_file(&tmp);
        return Err(io_err("cannot rename onto", &target, &e));
    }
    Ok(())
}
