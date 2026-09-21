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
use crate::bounds::Bounds;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
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
            // Quoted, even though this is the in-memory implementation.
            //
            // `MemoryFs` is NOT test-only: `macrooct-iir-compiler` builds one
            // for every `compile_source`, which is what `lang-aot`'s
            // `compile_source_to_iir` calls. A security review reproduced a raw
            // terminal escape reaching a build log straight through here, from
            // `@include "<ESC>[2Jpwned"`, because the spelling was interpolated
            // untouched — and untruncated, so a 5 KB spelling produced a 5 KB
            // diagnostic.
            //
            // The lesson is the interesting part: `quote`'s own doc claimed
            // that centralising the escape meant "a new interpolation cannot
            // forget", which is only true of call sites that actually call it.
            // Centralising a helper does not centralise the decision to use it.
            PpError::new(format!(
                "no such included file: {}",
                PpError::quote(&request.spelling, Bounds::default().diagnostic_quote_bytes)
            ))
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

/// One resolved, verified file.
struct Entry {
    /// The spelling the program used, for diagnostics. Never the real path.
    name: String,
    /// The handle opened and verified during `resolve`, taken by first `read`.
    handle: Option<File>,
    /// Text, cached on first read so a legitimate repeat include is served
    /// without re-opening anything by path.
    text: Option<String>,
}

/// A filesystem confined to a set of declared search roots.
///
/// # Resolve opens; read never touches a path
///
/// `resolve` canonicalises, checks containment against that canonical
/// **path**, then opens the file once and checks its kind and size against the
/// open **handle**, which it retains. `read` reads that handle and never
/// re-resolves or re-opens anything.
///
/// The containment check is the one that remains path-based, and saying so
/// matters: a swap between the canonicalise and the open still yields a handle
/// that passes the kind and size checks while pointing outside the root. The
/// window is far narrower than re-opening in `read`, but it is not zero.
/// Closing it needs handle-identity verification (device+inode /
/// `FILE_ID_INFO`), recorded as future work rather than claimed here.
///
/// An earlier version stored only the `PathBuf` and had `read` call
/// `std::fs::read(&path)`. That was canonicalise-then-open — TOCTOU by
/// construction — and a security review demonstrated it without needing a race
/// at all: rewriting the file between the two calls returned 5 MB through a
/// 64-byte bound. In that window the size bound, the regular-file check *and*
/// containment were all void, because `std::fs::read` follows a symlink
/// planted at the canonical path afterwards.
///
/// # Identity, not sequence
///
/// Files are keyed by canonical path, so resolving the same file twice returns
/// the same [`FileId`]. That is what makes the engine's include-cycle check
/// work: it compares ids, so minting a fresh id per inclusion (as an earlier
/// version did) meant a self-including file was never detected as a cycle — it
/// merely ran into the depth bound 200 levels later.
pub struct RootedFs {
    roots: Vec<PathBuf>,
    entries: Vec<Entry>,
    by_canon: HashMap<PathBuf, FileId>,
    bounds: Bounds,
}

impl std::fmt::Debug for RootedFs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RootedFs")
            .field("roots", &self.roots)
            .field("resolved", &self.entries.len())
            .finish()
    }
}

impl RootedFs {
    /// Declare the search roots, canonicalised once here so later comparisons
    /// are against a real path rather than a spelling.
    ///
    /// Takes the whole [`Bounds`] rather than a bare byte cap, so the per-file
    /// size limit and the diagnostic-quoting limit come from the same budget
    /// the engine enforces instead of a second number that can drift from it.
    pub fn new(
        roots: impl IntoIterator<Item = PathBuf>,
        bounds: Bounds,
    ) -> Result<RootedFs, PpError> {
        let mut canon = Vec::new();
        for r in roots {
            let c = r
                .canonicalize()
                .map_err(|_| PpError::new(format!("search root {} is unusable", r.display())))?;
            canon.push(c);
        }
        if canon.is_empty() {
            return Err(PpError::new("at least one search root is required"));
        }
        // Clamp here too, and not only in `preprocess`.
        //
        // `bytes_per_file` and `diagnostic_quote_bytes` are read ONLY from this
        // stored copy — the engine never looks at them — so clamping at the
        // engine's entry point did nothing for the two bounds that actually
        // gate attacker-controlled file reads. A security review drove 40 MiB
        // through the documented 16 MiB per-file cap by handing `u64::MAX`
        // straight to this constructor. Fixing one of the two places a bound
        // lives is not fixing it.
        let bounds = bounds.tighten(Bounds::default());
        Ok(RootedFs { roots: canon, entries: Vec::new(), by_canon: HashMap::new(), bounds })
    }

    /// The single refusal message for anything that fails after the spelling
    /// gate.
    ///
    /// Deliberately uniform, and deliberately naming only the *spelling*. An
    /// earlier version reported the resolved absolute path, the raw
    /// `io::Error`, or both — which turned an include into a filesystem oracle
    /// for anyone who can supply source to a shared builder: existence probing
    /// anywhere reachable, "access denied" distinguishable from "not found",
    /// and the build directory plus the service account's username recoverable
    /// from the out-of-root message. All outcomes now render identically.
    fn refuse(&self, spelling: &str) -> PpError {
        PpError::new(format!(
            "cannot include {}: no such file under any declared search root",
            PpError::quote(spelling, self.bounds.diagnostic_quote_bytes)
        ))
    }

    /// Reject spellings that are dangerous before we ever touch the disk.
    ///
    /// A first gate on the *spelling*. Not sufficient alone — a plain-looking
    /// relative path can still resolve through a symlink, which is what the
    /// canonicalised-result check in `resolve` is for — but some of these
    /// forms are dangerous to *open at all*.
    fn screen_spelling(spelling: &str) -> Result<(), PpError> {
        if spelling.is_empty() {
            return Err(PpError::new("empty include path"));
        }
        if spelling.as_bytes().contains(&0) {
            return Err(PpError::new("include path contains a NUL byte"));
        }
        if spelling.starts_with("\\\\") || spelling.starts_with("//") {
            return Err(PpError::new(
                "refusing UNC include path: opening it would authenticate to a remote host",
            ));
        }
        let p = Path::new(spelling);
        // `has_root()` as well as `is_absolute()`, and the difference is a real
        // hole rather than belt-and-braces: on Windows
        // `Path::new("/etc/passwd").is_absolute()` is FALSE, because absolute
        // needs a drive prefix there — it is merely root-relative. Checking
        // only `is_absolute()` lets a root-anchored path through on the
        // platform this repo primarily runs on.
        if p.is_absolute() || p.has_root() {
            return Err(PpError::new(
                "refusing root-anchored include path: includes resolve under a declared root",
            ));
        }
        let bytes = spelling.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
            return Err(PpError::new("refusing drive-qualified include path"));
        }
        for comp in p.components() {
            if let Component::Normal(os) = comp {
                let s = os.to_string_lossy();

                // ANY colon, not just `::`. The canonical alternate-data-stream
                // spelling is a SINGLE colon — `host.h:hidden` — and screening
                // only for `::` let it straight through: it canonicalises to a
                // path under the root, so containment passed and `is_file()`
                // was true. An ADS is a content channel that directory
                // listings, code review and most scanners do not show, which
                // makes it a good place to park a payload behind an
                // innocent-looking header. The one legitimate colon, a drive
                // letter, is already rejected above.
                if s.contains(':') {
                    return Err(PpError::new(
                        "refusing include path: a colon names an NTFS alternate data stream",
                    ));
                }

                // Trailing spaces and dots are STRIPPED by Win32 path
                // normalisation, so `NUL ` and `CON.` reach the same devices as
                // `NUL` and `CON`. Trim before comparing, or one space bypasses
                // the gate.
                let stem = s
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .trim_end_matches([' ', '.'])
                    .to_ascii_uppercase();
                const RESERVED: &[&str] = &[
                    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6",
                    "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
                    "LPT7", "LPT8", "LPT9",
                ];
                if RESERVED.contains(&stem.as_str()) {
                    return Err(PpError::new("refusing include path: reserved device name"));
                }
            }
        }
        Ok(())
    }

    /// True when `candidate` lies under one of the declared roots.
    ///
    /// `Path::starts_with` is COMPONENT-wise, not a string prefix test, which
    /// is what makes this sound: a root of `/a/b` does not contain
    /// `/a/bc/secret.h`. Pinned by
    /// `containment_compares_whole_components_not_string_prefixes`, because
    /// this is exactly the line someone later "simplifies" into a
    /// `to_string_lossy().starts_with(..)`, silently breaking it.
    fn contained(&self, candidate: &Path) -> bool {
        self.roots.iter().any(|r| candidate.starts_with(r))
    }
}

impl SourceFs for RootedFs {
    fn resolve(&mut self, request: &IncludeRequest) -> Result<FileId, PpError> {
        Self::screen_spelling(&request.spelling)?;

        for root in self.roots.clone() {
            let joined = root.join(&request.spelling);

            // Canonicalise, which resolves `..` AND follows any symlink or
            // reparse point. The RESULT is then checked against the roots, so
            // a symlink pointing outside is caught rather than trusted.
            let canon = match joined.canonicalize() {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Try the next root rather than ending the search: a spelling
            // that escapes root A may resolve legitimately inside root B, and
            // the first root must not be able to veto the others.
            if !self.contained(&canon) {
                continue;
            }

            // Identity: the same file resolved twice is the same FileId, which
            // is what makes the engine's cycle check work at all.
            if let Some(id) = self.by_canon.get(&canon) {
                return Ok(*id);
            }

            // Open ONCE, then verify the handle — never the path.
            let handle = match File::open(&canon) {
                Ok(h) => h,
                Err(_) => continue,
            };
            let meta = match handle.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            // Regular files only, checked on the opened handle. A FIFO or
            // character device would make the later read block forever, and no
            // resource bound can fire while the engine is stuck inside it.
            if !meta.is_file() {
                return Err(PpError::new(format!(
                    "refusing to include {}: not a regular file",
                    PpError::quote(&request.spelling, self.bounds.diagnostic_quote_bytes)
                )));
            }
            if meta.len() > self.bounds.bytes_per_file {
                return Err(PpError::new(format!(
                    "refusing to include {}: exceeds the {}-byte per-file bound",
                    PpError::quote(&request.spelling, self.bounds.diagnostic_quote_bytes),
                    self.bounds.bytes_per_file
                )));
            }

            let id = FileId::new(self.entries.len() as u32);
            self.entries.push(Entry {
                name: request.spelling.clone(),
                handle: Some(handle),
                text: None,
            });
            self.by_canon.insert(canon, id);
            return Ok(id);
        }

        Err(self.refuse(&request.spelling))
    }

    fn read(&mut self, file: FileId) -> Result<SourceText, PpError> {
        let quote_bytes = self.bounds.diagnostic_quote_bytes;
        let limit = self.bounds.bytes_per_file;
        let entry = self
            .entries
            .get_mut(file.index())
            .ok_or_else(|| PpError::new("read of an unresolved file"))?;

        // Served from cache on a repeat include of the same file, so a
        // legitimate second include does not fail on a consumed handle.
        //
        // Scope, stated precisely because an earlier comment overclaimed: the
        // cache is bounded by `total_source_bytes` only WITHIN one
        // `preprocess` run, and only because the engine charges every first
        // read. `RootedFs` is public and entries are never evicted, so a single
        // instance reused across N compilations accumulates up to N times that
        // budget. Nothing ties this cache to the engine's `Spend`. If a host
        // ever reuses one `RootedFs` across translation units, it must drop and
        // rebuild it per unit, or this needs real eviction.
        if let Some(text) = &entry.text {
            return Ok(text.clone());
        }

        // Read the handle opened and verified in `resolve`, then drop it, so
        // the descriptor cost is one file at a time rather than one per
        // inclusion.
        let mut handle = entry
            .handle
            .take()
            .ok_or_else(|| PpError::new("internal: verified handle already consumed"))?;
        // Bounded read, not `read_to_end`.
        //
        // Retaining the verified handle closes the PATH-swap window — a
        // symlink planted at the canonical path, or the name rebound to
        // another file, cannot affect us because we never look the path up
        // again. It does NOT close the REWRITE window: `std::fs::write`
        // truncates and rewrites the same file object, and an open handle sees
        // the new contents. A test here proved exactly that, driving 5 MB
        // through a 64-byte bound even with the handle retained.
        //
        // So the size bound is enforced where it cannot be evaded: on the read
        // itself. `take(limit + 1)` lets us tell "exactly at the limit" from
        // "over it" without ever allocating more than one byte past the cap.
        let mut bytes = Vec::new();
        (&mut handle)
            .take(limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| {
                PpError::new(format!("cannot read {}", PpError::quote(&entry.name, quote_bytes)))
            })?;
        drop(handle);
        if bytes.len() as u64 > limit {
            return Err(PpError::new(format!(
                "refusing to include {}: exceeds the {}-byte per-file bound",
                PpError::quote(&entry.name, quote_bytes),
                limit
            )));
        }

        // Reject non-UTF-8 rather than converting lossily. Silent
        // replacement-character substitution would change the token stream,
        // which is a correctness bug disguised as leniency.
        let text = String::from_utf8(bytes).map_err(|_| {
            PpError::new(format!(
                "{} is not valid UTF-8",
                PpError::quote(&entry.name, quote_bytes)
            ))
        })?;
        entry.text = Some(text.clone());
        Ok(text)
    }

    fn name_of(&self, file: FileId) -> String {
        self.entries
            .get(file.index())
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "<unknown>".to_string())
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

        let mut fs = RootedFs::new([root.clone()], Bounds::default()).unwrap();

        // Inside the root: fine.
        let id = fs.resolve(&req("ok.h")).unwrap();
        assert_eq!(fs.read(id).unwrap(), "ok");

        // Escaping it: refused, even though the spelling gate allowed `..`.
        assert!(fs.resolve(&req("../secret.h")).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_directory_is_not_readable_as_an_include() {
        let dir = std::env::temp_dir().join("prep01_rootedfs_dir_test");
        let root = dir.join("root");
        std::fs::create_dir_all(root.join("subdir")).unwrap();

        let mut fs = RootedFs::new([root], Bounds::default()).unwrap();
        assert!(fs.resolve(&req("subdir")).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refusals_do_not_leak_a_filesystem_oracle() {
        // The security property, asserted directly rather than implied.
        //
        // An earlier version reported the resolved absolute path for an
        // out-of-root hit, and the raw io::Error otherwise. That let a program
        // that can only supply SOURCE probe a shared builder's filesystem:
        // distinguish a file that exists but is unreadable from one that is
        // absent, confirm existence anywhere reachable, and recover the build
        // directory and the service account's username from the out-of-root
        // message.
        //
        // So the test is not "it refuses" — it is that the refusals are
        // INDISTINGUISHABLE from each other, and mention neither the resolved
        // path nor the OS error.
        let dir = std::env::temp_dir().join("prep01_rootedfs_oracle_test");
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(dir.join("present_outside.h"), "secret").unwrap();
        std::fs::write(root.join("real.h"), "ok").unwrap();

        let mut fs = RootedFs::new([root.clone()], Bounds::default()).unwrap();

        // The spelling itself is echoed back, which reveals nothing: the
        // program supplied it. What must not differ is everything ELSE, so
        // normalise the spelling out before comparing.
        let shape = |fs: &mut RootedFs, spelling: &str| {
            fs.resolve(&req(spelling)).unwrap_err().to_string().replace(spelling, "<SPELLING>")
        };

        let outside = shape(&mut fs, "../present_outside.h");
        let absent = shape(&mut fs, "../absent_entirely.h");
        let missing_inside = shape(&mut fs, "no_such.h");

        assert_eq!(
            outside, absent,
            "a file that EXISTS outside the root must not be distinguishable from one that does not exist"
        );
        assert_eq!(outside, missing_inside);

        for m in [&outside, &absent, &missing_inside] {
            assert!(!m.contains("present_outside"), "leaked the resolved target: {m}");
            assert!(
                !m.to_lowercase().contains("os error") && !m.contains("denied"),
                "leaked the raw OS error: {m}"
            );
            // The canonical root is an absolute path; none of it may appear.
            let root_str = root.canonicalize().unwrap().display().to_string();
            assert!(!m.contains(&root_str), "leaked the absolute build path: {m}");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_same_file_resolves_to_the_same_id_so_cycles_are_detectable() {
        // The engine detects include cycles by comparing FileIds on its active
        // stack. That check is worthless unless resolution is keyed on file
        // IDENTITY: an earlier version minted a fresh id per inclusion, so a
        // self-including file was never reported as a cycle and instead ran
        // into the depth bound 200 levels later.
        let dir = std::env::temp_dir().join("prep01_rootedfs_identity_test");
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("shared.h"), "shared").unwrap();

        let mut fs = RootedFs::new([root], Bounds::default()).unwrap();
        let a = fs.resolve(&req("shared.h")).unwrap();
        let b = fs.resolve(&req("shared.h")).unwrap();
        assert_eq!(a, b, "the same file must resolve to the same FileId");

        // And the spelling may differ while the file is the same.
        let c = fs.resolve(&req("./shared.h")).unwrap();
        assert_eq!(a, c, "identity is the canonical path, not the spelling");

        // A repeat read is still served (from cache) rather than failing
        // because the verified handle was already consumed.
        assert_eq!(fs.read(a).unwrap(), "shared");
        assert_eq!(fs.read(b).unwrap(), "shared");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_size_bound_holds_even_if_the_file_grows_after_resolution() {
        // The TOCTOU regression, without needing a race: `resolve` verifies an
        // open handle and `read` reads THAT handle, so rewriting the path in
        // between cannot smuggle bytes past the per-file bound. When `read`
        // re-opened by path, a security review drove 5 MB through a 64-byte
        // bound this way.
        let dir = std::env::temp_dir().join("prep01_rootedfs_toctou_test");
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("small.h");
        std::fs::write(&target, "small").unwrap();

        let mut fs =
            RootedFs::new([root], Bounds { bytes_per_file: 64, ..Bounds::default() }).unwrap();
        let id = fs.resolve(&req("small.h")).unwrap();

        // Swap in something far over the bound, after verification.
        std::fs::write(&target, "x".repeat(5_000_000)).unwrap();

        // The property that matters is that the BOUND holds. Retaining the
        // handle does not by itself achieve it: `std::fs::write` truncates and
        // rewrites the same file object, which an open handle sees. So the
        // read is bounded too, and this must refuse rather than return 5 MB.
        let err = fs
            .read(id)
            .expect_err("a file that grew past the bound after resolution must be refused");
        assert!(err.to_string().contains("per-file bound"), "{err}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_single_colon_alternate_data_stream_is_refused() {
        // `::$DATA` is the spelling everyone screens for; `file:stream` is the
        // one that actually gets used, and screening only for `::` let it
        // straight through. An ADS canonicalises to a path under the root, so
        // containment passes and `is_file()` is true — the payload rides along
        // behind an innocent-looking header that no directory listing shows.
        for p in ["host.h:hidden", "host.h:hidden:$DATA", "host.h::$DATA", "sub/x.h:s"] {
            assert!(RootedFs::screen_spelling(p).is_err(), "{p} should be refused");
        }
    }

    #[test]
    fn reserved_device_names_are_refused_through_win32_normalisation() {
        // Win32 strips trailing spaces and dots, so `NUL ` and `CON.` reach the
        // same devices as `NUL` and `CON`. A gate that compares before
        // trimming is bypassed by one space.
        for p in ["NUL ", "CON ", "CON.", "sub/COM1 ", "nul .h", "LPT9."] {
            assert!(RootedFs::screen_spelling(p).is_err(), "{p} should be refused");
        }
    }

    #[test]
    fn a_caller_cannot_widen_the_filesystem_bounds() {
        // The same class of bug as the engine's clamp, and it lived in TWO
        // places — `bytes_per_file` and `diagnostic_quote_bytes` are read only
        // from RootedFs's own copy, so clamping in `preprocess` alone left
        // them wide open. A security review drove 40 MiB through the
        // documented 16 MiB cap by handing u64::MAX straight to this
        // constructor.
        let dir = std::env::temp_dir().join("prep01_rootedfs_widen_test");
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let big = Bounds::default().bytes_per_file as usize + 1024;
        std::fs::write(root.join("huge.h"), "x".repeat(big)).unwrap();

        let mut fs = RootedFs::new(
            [root],
            Bounds { bytes_per_file: u64::MAX, diagnostic_quote_bytes: u32::MAX, ..Bounds::default() },
        )
        .unwrap();

        // Refused at the default cap despite the caller asking for no cap.
        assert!(fs.resolve(&req("huge.h")).is_err(), "a widened per-file bound must not be honoured");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_earlier_root_cannot_veto_a_later_one() {
        // Containment and open failures `continue` to the next root rather
        // than ending the search, so a spelling that escapes root A but
        // resolves legitimately inside root B still works.
        let dir = std::env::temp_dir().join("prep01_rootedfs_multiroot_test");
        let a = dir.join("a");
        let b = dir.join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(b.join("only_in_b.h"), "from_b").unwrap();

        let mut fs = RootedFs::new([a, b], Bounds::default()).unwrap();
        let id = fs.resolve(&req("only_in_b.h")).unwrap();
        assert_eq!(fs.read(id).unwrap(), "from_b");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_embedded_nul_byte_is_refused() {
        assert!(RootedFs::screen_spelling("ok\0.h").is_err());
    }
    #[test]
    fn an_oversized_file_is_refused_from_metadata_not_after_reading_it() {
        let dir = std::env::temp_dir().join("prep01_rootedfs_size_test");
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("big.h"), vec![b'x'; 4096]).unwrap();

        let mut fs = RootedFs::new([root], Bounds { bytes_per_file: 1024, ..Bounds::default() }).unwrap();
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

        let mut fs = RootedFs::new([root], Bounds::default()).unwrap();
        let id = fs.resolve(&req("bad.h")).unwrap();
        let e = fs.read(id).unwrap_err().to_string();
        assert!(e.contains("not valid UTF-8"), "{e}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn containment_compares_whole_components_not_string_prefixes() {
        // `contained()` is `candidate.starts_with(root)`, and its correctness
        // rests entirely on `Path::starts_with` being COMPONENT-wise rather
        // than a string prefix test. If it were a string comparison, a root of
        // `/a/b` would happily contain `/a/bc/secret.h` — a sibling directory
        // whose name merely begins with the root's — and the whole containment
        // property would be worthless.
        //
        // Verified against the real implementation rather than assumed, and
        // pinned here because this is exactly the kind of line someone later
        // "simplifies" into `to_string_lossy().starts_with(...)`.
        let root = Path::new("/a/b");
        assert!(Path::new("/a/b/ok.h").starts_with(root));
        assert!(!Path::new("/a/bc/secret.h").starts_with(root), "sibling prefix must NOT be contained");
        assert!(!Path::new("/a/bcd").starts_with(root));

        let wroot = Path::new(r"C:\a\b");
        assert!(Path::new(r"C:\a\b\ok.h").starts_with(wroot));
        assert!(!Path::new(r"C:\a\bc\secret.h").starts_with(wroot));
    }

    #[test]
    fn a_filesystem_with_no_roots_is_refused() {
        assert!(RootedFs::new([], Bounds::default()).is_err());
    }
}
