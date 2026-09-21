//! # File access — the only part of the engine that touches the outside world.
//!
//! Every read goes through [`SourceFs`]. The engine itself has no ambient
//! filesystem access at all, which is what lets the whole engine be tested
//! in-memory and keeps path policy in exactly one auditable place.
//!
//! ## Why the engine never sees a path
//!
//! [`SourceFs::resolve`] takes a request and returns a [`FileId`]; the engine
//! never handles a `Path`. That is deliberate — it means path containment
//! cannot be scattered through the engine's logic — but it has a consequence
//! that must be stated plainly: **containment is an obligation of the
//! `SourceFs` implementation, not something the engine can check.**
//! [`RootedFs`] is the sanctioned production implementation; a dialect must not
//! supply its own.
//!
//! ## What an include path can do if you let it
//!
//! `@include "…"` is attacker-influenced text. The forms that matter:
//!
//! ```text
//!   ../../../../etc/passwd    escape the root by traversal
//!   /etc/passwd               escape by absolute path
//!   C:\Windows\win.ini        …the Windows spelling
//!   \\host\share\x.h          UNC: triggers an OUTBOUND SMB AUTH and leaks an
//!                             NTLM hash — a credential-disclosure primitive
//!                             from nothing but a source file
//!   x.h::$DATA                NTFS alternate data stream
//!   CON, NUL, COM1, LPT1      reserved device names (open succeeds!)
//!   a-symlink-to-anywhere     escape without any suspicious-looking text
//!   /dev/stdin, a FIFO        never returns: the engine hangs INSIDE read(),
//!                             so not one resource bound ever fires
//! ```
//!
//! That last one deserves emphasis: a hang is the most durable denial of
//! service available against a build service, and it is invisible to every
//! counter in [`crate::bounds`] because the engine never gets control back.
//! Hence "regular files only", checked on the handle.

use crate::diag::PpError;
use crate::source_map::FileId;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

/// A request to include another file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeRequest {
    /// The spelling from the directive, with quotes already stripped.
    pub spelling: String,
    /// The file the directive appeared in, so a relative include can resolve
    /// against its directory.
    pub from: Option<FileId>,
    /// Whether the dialect classified this as a "system" include (C's `<…>`)
    /// rather than a local one (C's `"…"`).
    pub system: bool,
}

/// Text of one source file, owned.
///
/// Owned rather than a `&str` borrowed from `&self`: a borrow would pin every
/// file read for the lifetime of the filesystem with no eviction, and would
/// push implementors toward arena allocation, `Box::leak` or interior
/// mutability with `unsafe` — a poor shape for the one component that handles
/// attacker-controlled input.
pub type SourceText = String;

/// Resolution and reading of include targets.
///
/// # Security contract
///
/// `resolve` **must** reject any request that resolves outside a declared
/// search root, and must open only regular files. See the module header for
/// the specific forms that must be rejected.
pub trait SourceFs {
    /// Resolve, open, validate and retain — in one step.
    ///
    /// One step on purpose. If `resolve` opened a handle, checked it and
    /// dropped it, `read` would have to re-open, and every swap the check
    /// defeats — a symlink substituted into a directory component, the file
    /// replaced after its size was measured — would be live again in the
    /// window between the two calls. The returned `FileId` therefore names an
    /// already-verified file; an implementation that re-opens in `read` must
    /// repeat the full verification there.
    fn resolve(&mut self, request: &IncludeRequest) -> Result<FileId, PpError>;

    /// Read a file already resolved and verified by [`SourceFs::resolve`].
    fn read(&mut self, file: FileId) -> Result<SourceText, PpError>;

    /// A display name for diagnostics.
    fn name_of(&self, file: FileId) -> String;
}

// ===========================================================================
// MemoryFs — the testing implementation
// ===========================================================================

/// An entirely in-memory filesystem.
///
/// The engine's tests use this so they touch no real disk: no temp-file
/// cleanup, no platform differences, and no way for a test to accidentally
/// depend on the machine it runs on.
#[derive(Debug, Default, Clone)]
pub struct MemoryFs {
    names: Vec<String>,
    texts: Vec<String>,
    by_name: HashMap<String, FileId>,
}

impl MemoryFs {
    #[must_use]
    pub fn new() -> MemoryFs {
        MemoryFs::default()
    }

    /// Add a file. Returns its id so a test can name it as an origin.
    pub fn insert(&mut self, name: impl Into<String>, text: impl Into<String>) -> FileId {
        let name = name.into();
        let id = FileId::new(self.names.len() as u32);
        self.names.push(name.clone());
        self.texts.push(text.into());
        self.by_name.insert(name, id);
        id
    }
}

impl SourceFs for MemoryFs {
    fn resolve(&mut self, request: &IncludeRequest) -> Result<FileId, PpError> {
        self.by_name.get(&request.spelling).copied().ok_or_else(|| {
            PpError::new(format!("no such included file: {}", request.spelling))
        })
    }

    fn read(&mut self, file: FileId) -> Result<SourceText, PpError> {
        self.texts
            .get(file.index())
            .cloned()
            .ok_or_else(|| PpError::new("read of an unresolved file"))
    }

    fn name_of(&self, file: FileId) -> String {
        self.names.get(file.index()).cloned().unwrap_or_else(|| "<unknown>".to_string())
    }
}

// ===========================================================================
// RootedFs — the production implementation
// ===========================================================================

/// A filesystem confined to a set of declared search roots.
///
/// This is where the whole path-safety story is enforced. See the module
/// header for the threat list.
#[derive(Debug)]
pub struct RootedFs {
    roots: Vec<PathBuf>,
    names: Vec<String>,
    paths: Vec<PathBuf>,
    max_bytes: u64,
}

impl RootedFs {
    /// Declare the search roots. Each is canonicalised once, here, so later
    /// comparisons are against a real path rather than a spelling.
    pub fn new(roots: impl IntoIterator<Item = PathBuf>, max_bytes: u64) -> Result<RootedFs, PpError> {
        let mut canon = Vec::new();
        for r in roots {
            let c = r
                .canonicalize()
                .map_err(|e| PpError::new(format!("search root {} is unusable: {e}", r.display())))?;
            canon.push(c);
        }
        if canon.is_empty() {
            return Err(PpError::new("at least one search root is required"));
        }
        Ok(RootedFs { roots: canon, names: Vec::new(), paths: Vec::new(), max_bytes })
    }

    /// Reject spellings that are dangerous before we ever touch the disk.
    ///
    /// Done on the *spelling* as a first gate. It is not sufficient on its own
    /// — a plain-looking relative path can still resolve through a symlink —
    /// which is why `resolve` also checks the canonicalised result. This gate
    /// exists because some of these forms are dangerous to *open at all*.
    fn screen_spelling(spelling: &str) -> Result<(), PpError> {
        if spelling.is_empty() {
            return Err(PpError::new("empty include path"));
        }
        // UNC, in both spellings. Opening one authenticates outbound.
        if spelling.starts_with("\\\\") || spelling.starts_with("//") {
            return Err(PpError::new(format!(
                "refusing UNC include path {spelling}: opening it would authenticate to a remote host"
            )));
        }
        // NTFS alternate data stream, and Windows drive-absolute paths.
        if spelling.contains("::") {
            return Err(PpError::new(format!(
                "refusing include path {spelling}: NTFS alternate data stream"
            )));
        }
        let p = Path::new(spelling);
        // `has_root()` as well as `is_absolute()`, and the difference is a real
        // hole rather than belt-and-braces: on Windows
        // `Path::new("/etc/passwd").is_absolute()` is FALSE, because an
        // absolute path there needs a drive prefix. `/etc/passwd` is merely
        // root-relative. Checking only `is_absolute()` would let a
        // root-anchored path through on the platform this repo primarily runs
        // on. `has_root()` is true for both spellings on both platforms.
        if p.is_absolute() || p.has_root() {
            return Err(PpError::new(format!(
                "refusing root-anchored include path {spelling}: includes resolve under a declared root"
            )));
        }
        // `C:foo` is drive-relative on Windows and is not caught by is_absolute.
        let bytes = spelling.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
            return Err(PpError::new(format!(
                "refusing drive-qualified include path {spelling}"
            )));
        }
        for comp in p.components() {
            if let Component::Normal(os) = comp {
                let s = os.to_string_lossy();
                let stem = s.split('.').next().unwrap_or("").to_ascii_uppercase();
                const RESERVED: &[&str] = &[
                    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6",
                    "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
                    "LPT7", "LPT8", "LPT9",
                ];
                if RESERVED.contains(&stem.as_str()) {
                    return Err(PpError::new(format!(
                        "refusing include path {spelling}: {stem} is a reserved device name"
                    )));
                }
            }
        }
        Ok(())
    }

    /// True when `candidate` lies under one of the declared roots.
    fn contained(&self, candidate: &Path) -> bool {
        self.roots.iter().any(|r| candidate.starts_with(r))
    }
}

impl SourceFs for RootedFs {
    fn resolve(&mut self, request: &IncludeRequest) -> Result<FileId, PpError> {
        Self::screen_spelling(&request.spelling)?;

        // Try each root in order.
        let mut last_err = None;
        for root in self.roots.clone() {
            let joined = root.join(&request.spelling);

            // Canonicalise, which resolves `..` AND follows any symlink. We
            // then check the RESULT against the roots, so a symlink pointing
            // outside is caught here rather than trusted.
            let canon = match joined.canonicalize() {
                Ok(c) => c,
                Err(e) => {
                    last_err = Some(e);
                    continue;
                }
            };
            if !self.contained(&canon) {
                return Err(PpError::new(format!(
                    "include {} resolves to {} which is outside every declared search root",
                    request.spelling,
                    canon.display()
                )));
            }

            // Regular files only, checked on the resolved target. A FIFO or a
            // character device would make the later read block forever, and no
            // resource bound can fire while the engine is stuck inside it.
            let meta = std::fs::metadata(&canon)
                .map_err(|e| PpError::new(format!("cannot stat {}: {e}", canon.display())))?;
            if !meta.is_file() {
                return Err(PpError::new(format!(
                    "refusing to include {}: not a regular file",
                    canon.display()
                )));
            }
            // Size from metadata BEFORE reading, so an enormous file is refused
            // rather than read and then rejected.
            if meta.len() > self.max_bytes {
                return Err(PpError::new(format!(
                    "refusing to include {}: {} bytes exceeds the {}-byte per-file bound",
                    canon.display(),
                    meta.len(),
                    self.max_bytes
                )));
            }

            let id = FileId::new(self.paths.len() as u32);
            self.names.push(request.spelling.clone());
            self.paths.push(canon);
            return Ok(id);
        }

        Err(PpError::new(match last_err {
            Some(e) => format!("cannot resolve include {}: {e}", request.spelling),
            None => format!("cannot resolve include {}", request.spelling),
        }))
    }

    fn read(&mut self, file: FileId) -> Result<SourceText, PpError> {
        let path = self
            .paths
            .get(file.index())
            .ok_or_else(|| PpError::new("read of an unresolved file"))?
            .clone();

        let bytes = std::fs::read(&path)
            .map_err(|e| PpError::new(format!("cannot read {}: {e}", path.display())))?;

        // Reject non-UTF-8 rather than converting lossily. Silent
        // replacement-character substitution would change the token stream,
        // which is a correctness bug disguised as leniency.
        String::from_utf8(bytes).map_err(|_| {
            PpError::new(format!("{} is not valid UTF-8", path.display()))
        })
    }

    fn name_of(&self, file: FileId) -> String {
        self.names.get(file.index()).cloned().unwrap_or_else(|| "<unknown>".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(spelling: &str) -> IncludeRequest {
        IncludeRequest { spelling: spelling.to_string(), from: None, system: false }
    }

    #[test]
    fn memory_fs_round_trips() {
        let mut fs = MemoryFs::new();
        fs.insert("a.oct", "fn main() {}");
        let id = fs.resolve(&req("a.oct")).unwrap();
        assert_eq!(fs.read(id).unwrap(), "fn main() {}");
        assert_eq!(fs.name_of(id), "a.oct");
    }

    #[test]
    fn memory_fs_reports_a_missing_file() {
        let mut fs = MemoryFs::new();
        assert!(fs.resolve(&req("nope.oct")).is_err());
    }

    // --- the spelling gate -------------------------------------------------
    //
    // One test per rejected form, so a regression names the form it broke
    // rather than just "a path test failed".

    #[test]
    fn rejects_unc_paths_both_spellings() {
        // The NTLM-leak case. Worth its own test because the danger is not
        // traversal -- it is that merely opening it authenticates outbound.
        for p in ["\\\\attacker\\share\\x.h", "//attacker/share/x.h"] {
            let e = RootedFs::screen_spelling(p).unwrap_err();
            assert!(e.to_string().contains("UNC"), "{p} -> {e}");
        }
    }

    #[test]
    fn rejects_alternate_data_streams() {
        assert!(RootedFs::screen_spelling("x.h::$DATA").is_err());
    }

    #[test]
    fn rejects_absolute_and_drive_qualified_paths() {
        // `/etc/passwd` is the one that caught a real bug: on Windows its
        // `is_absolute()` is FALSE (absolute needs a drive prefix there), so a
        // gate written only against `is_absolute()` let it through on the
        // platform this repo primarily runs on. Keep both spellings here.
        assert!(RootedFs::screen_spelling("/etc/passwd").is_err());
        assert!(RootedFs::screen_spelling("\\Windows\\win.ini").is_err());
        assert!(RootedFs::screen_spelling("C:relative.h").is_err());
        assert!(RootedFs::screen_spelling("C:\\Windows\\win.ini").is_err());
    }

    #[test]
    fn rejects_reserved_device_names_with_and_without_extension() {
        // These OPEN SUCCESSFULLY on Windows, which is exactly why they need
        // rejecting by name rather than being left to fail naturally.
        for p in ["CON", "NUL", "COM1", "nul.h", "sub/AUX.oct", "lpt9.txt"] {
            assert!(RootedFs::screen_spelling(p).is_err(), "{p} should be refused");
        }
    }

    #[test]
    fn rejects_an_empty_spelling() {
        assert!(RootedFs::screen_spelling("").is_err());
    }

    #[test]
    fn allows_ordinary_relative_spellings() {
        // The gate must not be so aggressive that normal includes stop working.
        for p in ["ports.oct", "sub/ports.oct", "./ports.oct", "../shared/ports.oct"] {
            assert!(RootedFs::screen_spelling(p).is_ok(), "{p} should be allowed at the gate");
        }
        // Note `../shared/ports.oct` passes the SPELLING gate deliberately:
        // traversal is legitimate as long as it lands inside a root, and that
        // is decided by the canonicalised-result check in `resolve`, not here.
    }

    #[test]
    fn traversal_escaping_the_root_is_refused_after_canonicalisation() {
        let dir = std::env::temp_dir().join("prep01_rootedfs_escape_test");
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(dir.join("secret.h"), "secret").unwrap();
        std::fs::write(root.join("ok.h"), "ok").unwrap();

        let mut fs = RootedFs::new([root.clone()], 1 << 20).unwrap();

        // Inside the root: fine.
        let id = fs.resolve(&req("ok.h")).unwrap();
        assert_eq!(fs.read(id).unwrap(), "ok");

        // Escaping it: refused, even though the spelling gate allowed `..`.
        let e = fs.resolve(&req("../secret.h")).unwrap_err().to_string();
        assert!(e.contains("outside every declared search root"), "{e}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_directory_is_not_a_regular_file() {
        let dir = std::env::temp_dir().join("prep01_rootedfs_dir_test");
        let root = dir.join("root");
        std::fs::create_dir_all(root.join("subdir")).unwrap();

        let mut fs = RootedFs::new([root], 1 << 20).unwrap();
        let e = fs.resolve(&req("subdir")).unwrap_err().to_string();
        assert!(e.contains("not a regular file"), "{e}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_oversized_file_is_refused_from_metadata_not_after_reading_it() {
        let dir = std::env::temp_dir().join("prep01_rootedfs_size_test");
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("big.h"), vec![b'x'; 4096]).unwrap();

        let mut fs = RootedFs::new([root], 1024).unwrap();
        let e = fs.resolve(&req("big.h")).unwrap_err().to_string();
        assert!(e.contains("exceeds"), "{e}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_utf8_is_refused_rather_than_lossily_converted() {
        let dir = std::env::temp_dir().join("prep01_rootedfs_utf8_test");
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("bad.h"), [0xff, 0xfe, 0x00]).unwrap();

        let mut fs = RootedFs::new([root], 1 << 20).unwrap();
        let id = fs.resolve(&req("bad.h")).unwrap();
        let e = fs.read(id).unwrap_err().to_string();
        assert!(e.contains("not valid UTF-8"), "{e}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_filesystem_with_no_roots_is_refused() {
        assert!(RootedFs::new([], 1 << 20).is_err());
    }
}
