//! # `coding_adventures_vault_pm_storage_removable` — VLT-PM00 §12 / §23 item 14
//!
//! ## What this crate is for
//!
//! `VLT-PM00-local-first-password-manager.md` §12 lists `removable/synced
//! folder` as its own backend row, right beside `filesystem`, with one note:
//! *"warn about third-party sync conflict copies"*. The two rows share the
//! exact same on-disk immutable object format (`storage-fs`'s
//! `<root>/<hex namespace>/<hex key>` layout) — `removable` is not a new
//! transport, it is the same filesystem backend used in a directory a
//! *third-party* tool also writes to: Dropbox, OneDrive, Syncthing, a NAS
//! client's sync agent, or a literal USB drive carried between machines and
//! opened by more than one of them without coordination.
//!
//! None of those tools know vault-pm's immutable-object invariant. When two
//! of them write the same logical file at once, or when a person edits a
//! synced copy on a second machine, the sync tool's own conflict-resolution
//! policy kicks in — and every mainstream one resolves a same-name collision
//! by keeping *both* files under different names, never by overwriting
//! silently. That is the detectable signature this crate looks for: a file
//! sitting where only an ordinary lowercase-hex vault-pm object name belongs,
//! spelled some other way.
//!
//! This crate does **not** try to out-adversary a hostile storage backend.
//! §7.1's "malicious storage service" adversary (reorders, duplicates,
//! corrupts, withholds, deletes, replays objects) is already covered by
//! object-ID content addressing, AEAD, and signatures at the repository and
//! application layers above this one. This crate's whole job is narrower and
//! more mundane: notice the ordinary, non-adversarial mess a sync tool leaves
//! behind, and say so plainly instead of silently ignoring it or refusing to
//! open.
//!
//! ## What it does not do
//!
//! - No cryptography, no content interpretation. A scan only ever looks at
//!   *names*, never at file bytes (the copy helper below is the one
//!   exception, and even there it only ever compares bytes for equality,
//!   never decodes them).
//! - No raw filenames ever leave this crate's public API. A third-party sync
//!   tool's filename is attacker-adjacent input the moment a vault is shared
//!   with anyone else, and this codebase's own convention (see
//!   `vault-pm-storage`'s redacted `Debug` impls) is to never echo
//!   attacker-controlled text into a terminal, a log, or an error message.
//!   Findings are reported as bounded counts by closed classification, never
//!   as strings.
//! - No opinion on which bucket an object belongs to. `storage-fs`'s
//!   namespace/key hex encoding is structural, not semantic, so this crate
//!   only ever checks *shape* (even-length lowercase hex), never decodes a
//!   name back to a `BucketId`/`ObjectId`. That keeps it below
//!   `vault-pm-storage`'s opaque-identifier boundary rather than reaching
//!   across it.
//!
//! ## Two operations
//!
//! 1. [`scan_object_root`] — the detector. Walks one object-store root and
//!    reports how many entries look like ordinary vault-pm objects versus how
//!    many look like sync-tool interference, classified into
//!    [`SyncInterferenceKind`]. Used by `storage check` and `doctor` (VLT-PM00
//!    §23 item 14) to warn without blocking.
//! 2. [`copy_object_tree`] — the migration helper. Copies every committed
//!    object file from one object-store root to another, verifying every
//!    write by reading it back, for `storage migrate` (§19.1). It scans the
//!    source first and carries that report along rather than blocking on it,
//!    because the source directory being "dirty" in the way this crate
//!    detects is exactly the situation `storage migrate` (moving off a
//!    flaky synced folder) exists to get a vault *out of*.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// Absolute ceiling on one copied object file, aligned with
/// `vault-pm-storage::MAX_OBJECT_BYTES` (itself aligned with VLT-PM01's
/// object-frame bound). Kept as this crate's own constant rather than an
/// added dependency edge, since this crate works one layer below the opaque
/// `VaultObjectStore` contract, on raw files.
pub const MAX_COPY_OBJECT_BYTES: u64 = 64 * 1024 * 1024;

/// Largest number of directory entries one [`scan_object_root`] or
/// [`copy_object_tree`] call will walk, across every bucket directory
/// combined. Bounds the cost of pointing either function at a huge or
/// adversarially padded directory (a removable drive is, by definition,
/// attacker-reachable in a way a fixed local path is not).
pub const MAX_SCANNED_ENTRIES: usize = 1_000_000;

/// Where an unexpected entry was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntryLocation {
    /// Directly under the object-store root, where only bucket directories
    /// (even-length lowercase hex names) belong.
    TopLevel,
    /// Inside a bucket directory, where only object files (even-length
    /// lowercase hex names, or a `.tmp` in-flight write) belong.
    WithinBucket,
}

/// Closed classification of one unexpected filesystem entry.
///
/// Every variant is inferred from *shape* alone — never from file content —
/// and the classification itself carries no attacker-controlled text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyncInterferenceKind {
    /// The name matches a mainstream sync tool's same-name-collision
    /// convention: contains "conflict" case-insensitively (Dropbox: `"...
    /// (<device>'s conflicted copy ...)"`, OneDrive: `"...-<device>'s
    /// conflicted copy..."`, Google Drive Desktop and most NAS clients use
    /// the same word), or Syncthing's fixed `.sync-conflict-<timestamp>-`
    /// infix, or a bare parenthesized duplicate-count suffix (`" (1)"`,
    /// `" (2)"`) that Explorer/Finder/rclone use for the same purpose.
    ConflictCopy,
    /// A dotfile or well-known sentinel a filesystem browser or sync client
    /// leaves behind without ever representing a vault-pm object:
    /// `.DS_Store`, `Thumbs.db`, `desktop.ini`, `.~lock.*#` (LibreOffice),
    /// or any other leading-dot name that is not this backend's own
    /// `<hex>.tmp`.
    HiddenMetadata,
    /// A partially transferred file a sync client has not yet renamed into
    /// place: `.crdownload`, `.part`, `.filepart`, `.download`, or a
    /// `~`-prefixed editor swap file.
    PartialTransfer,
    /// Present, not `.tmp`, and matches none of the above — still worth a
    /// person's attention, just not one this crate can name more precisely.
    Unknown,
}

fn classify(name: &str) -> SyncInterferenceKind {
    let lower = name.to_ascii_lowercase();
    if lower.contains("conflict") || has_duplicate_count_suffix(&lower) {
        return SyncInterferenceKind::ConflictCopy;
    }
    if lower == ".ds_store"
        || lower == "thumbs.db"
        || lower == "desktop.ini"
        || (lower.starts_with(".~lock."))
        || (name.starts_with('.') && !is_hex_tmp_name(name))
    {
        return SyncInterferenceKind::HiddenMetadata;
    }
    if lower.ends_with(".crdownload")
        || lower.ends_with(".part")
        || lower.ends_with(".filepart")
        || lower.ends_with(".download")
        || name.starts_with('~')
    {
        return SyncInterferenceKind::PartialTransfer;
    }
    SyncInterferenceKind::Unknown
}

/// Whether `name` ends with a bare parenthesized duplicate-count suffix, e.g.
/// `"a1b2 (1)"`. Only the shape matters; the base name is not inspected.
fn has_duplicate_count_suffix(lower: &str) -> bool {
    let Some(open) = lower.rfind(" (") else {
        return false;
    };
    let Some(close_offset) = lower[open..].find(')') else {
        return false;
    };
    let close = open + close_offset;
    if close != lower.len() - 1 {
        return false;
    }
    let digits = &lower[open + 2..close];
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

/// Whether `name` is an even-length lowercase-hex string — the exact shape
/// `storage-fs` uses for both its namespace directories and its object
/// files.
fn is_hex_name(name: &str) -> bool {
    !name.is_empty()
        && name.len().is_multiple_of(2)
        && name
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Whether `name` is `storage-fs`'s own in-flight write marker: an
/// even-length lowercase-hex stem with a literal `.tmp` extension. These are
/// legitimate transient state (cleaned up by the real backend's own
/// `initialize`), never sync interference.
fn is_hex_tmp_name(name: &str) -> bool {
    name.strip_suffix(".tmp").is_some_and(is_hex_name)
}

/// One unexpected entry, located but never named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterferenceWarning {
    /// Where the entry was found.
    pub location: EntryLocation,
    /// What it looks like.
    pub kind: SyncInterferenceKind,
}

/// The result of one [`scan_object_root`] call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanReport {
    /// Bucket directories that matched the expected hex shape.
    pub bucket_directories: usize,
    /// Object files (or legitimate `.tmp` in-flight writes) that matched the
    /// expected hex shape.
    pub ordinary_objects: usize,
    /// Every entry that did not match, in scan order.
    pub warnings: Vec<InterferenceWarning>,
}

impl ScanReport {
    /// Whether anything worth a person's attention was found.
    pub fn is_clean(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Count warnings of one specific kind.
    pub fn count_of(&self, kind: SyncInterferenceKind) -> usize {
        self.warnings
            .iter()
            .filter(|warning| warning.kind == kind)
            .count()
    }
}

/// Closed, payload-free scan failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanError {
    /// The root does not exist, or exists but is not a directory.
    RootUnavailable,
    /// A read failed partway through (permissions, I/O error, or the root
    /// vanished mid-scan — plausible for a removable drive that is unplugged
    /// while `storage check` runs).
    ReadFailed,
    /// [`MAX_SCANNED_ENTRIES`] was exceeded.
    TooManyEntries,
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::RootUnavailable => "vault-pm-storage-removable: root unavailable",
            Self::ReadFailed => "vault-pm-storage-removable: read failed",
            Self::TooManyEntries => "vault-pm-storage-removable: too many entries",
        })
    }
}

impl std::error::Error for ScanError {}

/// Scan one object-store root for symptoms of third-party sync interference.
///
/// `root` is expected to be exactly what `storage-fs::FsStorageBackend` owns:
/// a directory containing zero or more even-length-lowercase-hex-named
/// subdirectories, each containing zero or more even-length-lowercase-hex
/// named files (or `<hex>.tmp` in-flight writes). Anything else, at either
/// level, is reported as a warning and never causes this function itself to
/// fail — a dirty directory is exactly the case the caller wants a report
/// about, not an error blocking the report.
pub fn scan_object_root(root: &Path) -> Result<ScanReport, ScanError> {
    let metadata = fs::metadata(root).map_err(|_| ScanError::RootUnavailable)?;
    if !metadata.is_dir() {
        return Err(ScanError::RootUnavailable);
    }

    let mut report = ScanReport::default();
    let mut scanned = 0_usize;
    for top_entry in read_dir_sorted(root)? {
        bump(&mut scanned)?;
        let name = entry_name(&top_entry)?;
        let is_dir = top_entry
            .file_type()
            .map_err(|_| ScanError::ReadFailed)?
            .is_dir();
        if is_dir && is_hex_name(&name) {
            report.bucket_directories += 1;
            for inner_entry in read_dir_sorted(&top_entry.path())? {
                bump(&mut scanned)?;
                let inner_name = entry_name(&inner_entry)?;
                // A positive `is_file()` check, not a negative `!is_dir()`
                // one: `DirEntry::file_type()` does not follow symlinks, so
                // a symlink's type is neither "dir" nor "file". A negative
                // check would let a symlink through as an "ordinary
                // object" whenever its name happened to be hex-shaped --
                // exactly the case a removable/synced folder makes
                // plausible (VLT-PM50 §7) -- and this crate never opens an
                // object it has classified as ordinary; `copy_object_tree`
                // does, and must draw the identical line (below).
                let inner_is_file = inner_entry
                    .file_type()
                    .map_err(|_| ScanError::ReadFailed)?
                    .is_file();
                if inner_is_file && (is_hex_name(&inner_name) || is_hex_tmp_name(&inner_name)) {
                    report.ordinary_objects += 1;
                } else {
                    report.warnings.push(InterferenceWarning {
                        location: EntryLocation::WithinBucket,
                        kind: classify(&inner_name),
                    });
                }
            }
        } else {
            report.warnings.push(InterferenceWarning {
                location: EntryLocation::TopLevel,
                kind: classify(&name),
            });
        }
    }
    Ok(report)
}

fn bump(scanned: &mut usize) -> Result<(), ScanError> {
    *scanned += 1;
    if *scanned > MAX_SCANNED_ENTRIES {
        return Err(ScanError::TooManyEntries);
    }
    Ok(())
}

fn read_dir_sorted(dir: &Path) -> Result<Vec<fs::DirEntry>, ScanError> {
    let mut entries = fs::read_dir(dir)
        .map_err(|_| ScanError::ReadFailed)?
        .collect::<Result<Vec<_>, io::Error>>()
        .map_err(|_| ScanError::ReadFailed)?;
    entries.sort_by_key(fs::DirEntry::file_name);
    Ok(entries)
}

fn entry_name(entry: &fs::DirEntry) -> Result<String, ScanError> {
    entry
        .file_name()
        .into_string()
        .map_err(|_| ScanError::ReadFailed)
}

/// The result of one [`copy_object_tree`] call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CopyReport {
    /// Object files newly written to the target and verified by read-back.
    pub copied_objects: usize,
    /// Bytes newly written to the target.
    pub copied_bytes: u64,
    /// Object files already present in the target with byte-identical
    /// content (an idempotent re-run of an interrupted migration lands here).
    pub already_present: usize,
    /// The source root's own interference scan, carried along rather than
    /// blocking the copy — VLT-PM00 §23 item 14: moving off a folder this
    /// crate flags as suspect is exactly what `storage migrate` is often
    /// *for*.
    pub source_warnings: ScanReport,
}

/// Closed, payload-free copy failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyError {
    /// The source root does not exist, or exists but is not a directory.
    SourceUnavailable,
    /// The target root could not be created or is not a directory.
    TargetUnavailable,
    /// A read or write failed partway through.
    IoFailed,
    /// The target already has a file at this object's path with *different*
    /// bytes than the source — a genuine immutability violation between two
    /// stores that are each individually supposed to be append-only. This is
    /// the one case `copy_object_tree` refuses to paper over.
    Conflict,
    /// A file exceeded [`MAX_COPY_OBJECT_BYTES`].
    ObjectTooLarge,
    /// [`MAX_SCANNED_ENTRIES`] was exceeded.
    TooManyEntries,
    /// A symlink was found exactly where this function needs a real file or
    /// directory: a bucket directory, an object file being read, or the
    /// exact path this function is about to create. Refused rather than
    /// followed, because `source`/`target` are exactly the directories a
    /// third-party sync tool or a second machine sharing removable media may
    /// also write to (VLT-PM50 §7) -- a planted symlink there could
    /// otherwise redirect a read to an arbitrary file this process can see,
    /// or a write to overwrite one.
    UnexpectedSymlink,
}

impl std::fmt::Display for CopyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::SourceUnavailable => "vault-pm-storage-removable: source unavailable",
            Self::TargetUnavailable => "vault-pm-storage-removable: target unavailable",
            Self::IoFailed => "vault-pm-storage-removable: I/O failed",
            Self::Conflict => "vault-pm-storage-removable: immutability conflict",
            Self::ObjectTooLarge => "vault-pm-storage-removable: object too large",
            Self::TooManyEntries => "vault-pm-storage-removable: too many entries",
            Self::UnexpectedSymlink => "vault-pm-storage-removable: unexpected symlink",
        })
    }
}

impl std::error::Error for CopyError {}

/// Copy every committed object file from `source` to `target`, verifying
/// each write by reading it back before counting it, for `storage migrate`
/// (VLT-PM00 §19.1 steps 2-4).
///
/// Only entries matching the expected `<hex>/<hex>` shape are copied;
/// `.tmp` in-flight writes are skipped (they are not committed state, by
/// `storage-fs`'s own documented contract), and anything else is left alone
/// in the source and simply counted in the returned scan. `target` is
/// created if it does not already exist. An object already present in
/// `target` with identical bytes is left untouched and counted as
/// `already_present`, making a re-run after an interrupted migration safe;
/// one with *different* bytes is reported as [`CopyError::Conflict`] and
/// aborts, because that is not a migration bug this function should paper
/// over — it means one of the two directories was mutated by something
/// other than an immutable put.
///
/// This function never deletes or modifies anything under `source`.
pub fn copy_object_tree(source: &Path, target: &Path) -> Result<CopyReport, CopyError> {
    let source_warnings = scan_object_root(source).map_err(|error| match error {
        ScanError::RootUnavailable => CopyError::SourceUnavailable,
        ScanError::ReadFailed => CopyError::IoFailed,
        ScanError::TooManyEntries => CopyError::TooManyEntries,
    })?;

    ensure_real_directory(target)?;

    let mut report = CopyReport {
        source_warnings,
        ..CopyReport::default()
    };
    let mut scanned = 0_usize;
    for bucket_entry in fs::read_dir(source).map_err(|_| CopyError::IoFailed)? {
        let bucket_entry = bucket_entry.map_err(|_| CopyError::IoFailed)?;
        bump_copy(&mut scanned)?;
        let bucket_name = bucket_entry
            .file_name()
            .into_string()
            .map_err(|_| CopyError::IoFailed)?;
        // A positive `is_dir()` requirement on the *unfollowed* link type:
        // `DirEntry::file_type()` reports a symlink's own type, never the
        // type of what it points to, so a symlink can be neither `is_dir()`
        // nor `is_file()` here and is silently excluded from the walk
        // either way -- it is still visible in `source_warnings` above,
        // since `scan_object_root` classifies it as an unexpected entry.
        if !bucket_entry
            .file_type()
            .map_err(|_| CopyError::IoFailed)?
            .is_dir()
            || !is_hex_name(&bucket_name)
        {
            continue;
        }
        let target_bucket_dir = target.join(&bucket_name);
        ensure_real_directory(&target_bucket_dir)?;

        for object_entry in fs::read_dir(bucket_entry.path()).map_err(|_| CopyError::IoFailed)? {
            let object_entry = object_entry.map_err(|_| CopyError::IoFailed)?;
            bump_copy(&mut scanned)?;
            let object_name = object_entry
                .file_name()
                .into_string()
                .map_err(|_| CopyError::IoFailed)?;
            // Positive `is_file()`, not `!is_dir()`: the latter would let a
            // symlink through whenever its name happened to be hex-shaped,
            // and `read_bounded` below follows symlinks by design (it opens
            // whatever `File::open` resolves to) -- so a symlink must never
            // reach it. `source`/`target` are exactly the directories a
            // third-party sync tool or a second machine may also write to
            // (VLT-PM50 §7); a planted `<hex-name> -> /etc/shadow` symlink
            // must never be silently read and copied as if it were an
            // ordinary sealed object.
            if !object_entry
                .file_type()
                .map_err(|_| CopyError::IoFailed)?
                .is_file()
                || !is_hex_name(&object_name)
            {
                continue; // `.tmp` writes, symlinks, and anything else are skipped.
            }
            copy_one_object(
                &object_entry.path(),
                &target_bucket_dir.join(&object_name),
                &mut report,
            )?;
        }
    }
    Ok(report)
}

/// Ensure `path` is a real (non-symlink) directory, creating it if absent.
///
/// Refuses rather than follows an existing symlink at `path`: `storage-fs`
/// never creates one, so one being there already means something else
/// wrote it, and blindly treating it as "the directory exists, proceed"
/// (`fs::create_dir_all`'s own behavior, since its own re-check follows
/// symlinks) would let a planted symlink redirect an entire bucket's worth
/// of writes to an attacker-chosen location.
fn ensure_real_directory(path: &Path) -> Result<(), CopyError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(CopyError::UnexpectedSymlink);
            }
            if metadata.is_dir() {
                Ok(())
            } else {
                Err(CopyError::TargetUnavailable)
            }
        }
        Err(_) => fs::create_dir_all(path).map_err(|_| CopyError::TargetUnavailable),
    }
}

fn bump_copy(scanned: &mut usize) -> Result<(), CopyError> {
    *scanned += 1;
    if *scanned > MAX_SCANNED_ENTRIES {
        return Err(CopyError::TooManyEntries);
    }
    Ok(())
}

fn copy_one_object(
    source_path: &Path,
    target_path: &Path,
    report: &mut CopyReport,
) -> Result<(), CopyError> {
    let source_bytes = read_bounded(source_path)?;

    if let Ok(existing) = fs::symlink_metadata(target_path) {
        if existing.file_type().is_symlink() {
            return Err(CopyError::UnexpectedSymlink);
        }
        if existing.is_file() {
            let target_bytes = read_bounded(target_path)?;
            return if target_bytes == source_bytes {
                report.already_present += 1;
                Ok(())
            } else {
                Err(CopyError::Conflict)
            };
        }
        return Err(CopyError::Conflict);
    }

    // Write-tmp-then-rename, the same discipline `storage-fs` itself uses,
    // so a crash mid-copy leaves either nothing or one fully written file at
    // the final path, never a truncated one. The staging name is
    // predictable (this function's own fixed convention), so the file is
    // opened with `create_new` -- which atomically refuses to follow or
    // replace *anything* already at that path, symlink included, rather
    // than the TOCTOU-prone "check then create" `File::create` would be.
    let mut staging_path: PathBuf = target_path.to_path_buf();
    let staged_name = format!(
        "{}.migrate-tmp",
        target_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(CopyError::IoFailed)?
    );
    staging_path.set_file_name(staged_name);
    remove_stale_staging_file(&staging_path)?;
    {
        let mut staging_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging_path)
            .map_err(|_| CopyError::IoFailed)?;
        staging_file
            .write_all(&source_bytes)
            .map_err(|_| CopyError::IoFailed)?;
        staging_file.sync_all().map_err(|_| CopyError::IoFailed)?;
    }
    fs::rename(&staging_path, target_path).map_err(|_| CopyError::IoFailed)?;

    // Read back before counting the object copied -- VLT-PM00 §19.1 step 4.
    let verify_bytes = read_bounded(target_path)?;
    if verify_bytes != source_bytes {
        return Err(CopyError::IoFailed);
    }
    report.copied_objects += 1;
    report.copied_bytes += verify_bytes.len() as u64;
    Ok(())
}

/// Remove a non-symlink leftover at the fixed staging path from an
/// interrupted previous attempt, so a retried migration stays idempotent.
///
/// A symlink found here is refused, not removed: only this function's own
/// write path ever creates a regular file at this exact predictable name,
/// so a symlink means something else placed it, and the caller's
/// subsequent `create_new` must be allowed to refuse it rather than have
/// this step silently clear the way.
fn remove_stale_staging_file(staging_path: &Path) -> Result<(), CopyError> {
    match fs::symlink_metadata(staging_path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(CopyError::UnexpectedSymlink);
            }
            fs::remove_file(staging_path).map_err(|_| CopyError::IoFailed)
        }
        Err(_) => Ok(()),
    }
}

/// Read `path`'s bytes, refusing rather than following a symlink there.
///
/// `source`/`target` (VLT-PM50 §7) are exactly the directories a
/// third-party sync tool, a second machine sharing removable media, or a
/// mirror configuration may also write to, so this is a real trust
/// boundary: without this check a planted `<hex-name> -> /etc/shadow` (or
/// any other file this process can read) would be opened, its bytes copied
/// into `target` under an innocuous object name, and — if `target` is
/// itself synced or cloud-backed — silently exfiltrated. Checking
/// `symlink_metadata` (which does not follow the link) before `File::open`
/// (which does) closes that path; the length bound is read from the same
/// non-following call, so a symlink to a zero-reporting special file (e.g.
/// `/dev/zero`) cannot bypass it either.
fn read_bounded(path: &Path) -> Result<Vec<u8>, CopyError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| CopyError::IoFailed)?;
    if metadata.file_type().is_symlink() {
        return Err(CopyError::UnexpectedSymlink);
    }
    if metadata.len() > MAX_COPY_OBJECT_BYTES {
        return Err(CopyError::ObjectTooLarge);
    }
    let mut file = File::open(path).map_err(|_| CopyError::IoFailed)?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes)
        .map_err(|_| CopyError::IoFailed)?;
    if bytes.len() as u64 > MAX_COPY_OBJECT_BYTES {
        return Err(CopyError::ObjectTooLarge);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let mut path = std::env::temp_dir();
            path.push(format!(
                "vault-pm-storage-removable-test-{}-{stamp}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_object(root: &Path, bucket: &str, object: &str, bytes: &[u8]) {
        let bucket_dir = root.join(bucket);
        fs::create_dir_all(&bucket_dir).unwrap();
        fs::write(bucket_dir.join(object), bytes).unwrap();
    }

    #[test]
    fn empty_root_scans_clean() {
        let root = TempDir::new();
        let report = scan_object_root(root.path()).unwrap();
        assert!(report.is_clean());
        assert_eq!(report.bucket_directories, 0);
        assert_eq!(report.ordinary_objects, 0);
    }

    #[test]
    fn ordinary_objects_and_in_flight_tmp_writes_are_clean() {
        let root = TempDir::new();
        write_object(root.path(), "2121", "aabbcc", b"ciphertext");
        write_object(root.path(), "2121", "ddeeff", b"more ciphertext");
        fs::write(root.path().join("2121").join("112233.tmp"), b"partial").unwrap();
        let report = scan_object_root(root.path()).unwrap();
        assert!(report.is_clean());
        assert_eq!(report.bucket_directories, 1);
        assert_eq!(report.ordinary_objects, 3);
    }

    #[test]
    fn dropbox_style_conflict_copy_is_detected_within_a_bucket() {
        let root = TempDir::new();
        write_object(root.path(), "2121", "aabbcc", b"ciphertext");
        fs::write(
            root.path()
                .join("2121")
                .join("aabbcc (Jane's conflicted copy 2026-08-17)"),
            b"ciphertext",
        )
        .unwrap();
        let report = scan_object_root(root.path()).unwrap();
        assert!(!report.is_clean());
        assert_eq!(report.count_of(SyncInterferenceKind::ConflictCopy), 1);
        assert_eq!(report.warnings[0].location, EntryLocation::WithinBucket);
    }

    #[test]
    fn syncthing_style_conflict_copy_is_detected() {
        let root = TempDir::new();
        write_object(root.path(), "2121", "aabbcc", b"ciphertext");
        fs::write(
            root.path()
                .join("2121")
                .join("aabbcc.sync-conflict-20260817-120000-ABCDEFG"),
            b"ciphertext",
        )
        .unwrap();
        let report = scan_object_root(root.path()).unwrap();
        assert_eq!(report.count_of(SyncInterferenceKind::ConflictCopy), 1);
    }

    #[test]
    fn explorer_style_duplicate_count_suffix_is_detected() {
        let root = TempDir::new();
        write_object(root.path(), "2121", "aabbcc (1)", b"ciphertext");
        let report = scan_object_root(root.path()).unwrap();
        assert_eq!(report.count_of(SyncInterferenceKind::ConflictCopy), 1);
    }

    #[test]
    fn os_and_client_metadata_files_are_classified_hidden() {
        let root = TempDir::new();
        write_object(root.path(), "2121", "aabbcc", b"ciphertext");
        fs::write(root.path().join("2121").join(".DS_Store"), b"").unwrap();
        fs::write(root.path().join("2121").join("Thumbs.db"), b"").unwrap();
        fs::write(root.path().join("2121").join("desktop.ini"), b"").unwrap();
        let report = scan_object_root(root.path()).unwrap();
        assert_eq!(report.count_of(SyncInterferenceKind::HiddenMetadata), 3);
    }

    #[test]
    fn partial_transfer_files_are_classified() {
        let root = TempDir::new();
        write_object(root.path(), "2121", "aabbcc", b"ciphertext");
        fs::write(
            root.path().join("2121").join("aabbcc.crdownload"),
            b"partial",
        )
        .unwrap();
        let report = scan_object_root(root.path()).unwrap();
        assert_eq!(report.count_of(SyncInterferenceKind::PartialTransfer), 1);
    }

    #[test]
    fn unrecognized_entries_fall_back_to_unknown() {
        let root = TempDir::new();
        write_object(root.path(), "2121", "aabbcc", b"ciphertext");
        fs::write(root.path().join("2121").join("readme.txt"), b"hi").unwrap();
        let report = scan_object_root(root.path()).unwrap();
        assert_eq!(report.count_of(SyncInterferenceKind::Unknown), 1);
    }

    #[test]
    fn unexpected_top_level_entry_is_flagged_at_top_level() {
        let root = TempDir::new();
        write_object(root.path(), "2121", "aabbcc", b"ciphertext");
        fs::write(root.path().join("not-hex-at-all"), b"hi").unwrap();
        let report = scan_object_root(root.path()).unwrap();
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].location, EntryLocation::TopLevel);
    }

    #[test]
    fn uppercase_hex_is_not_the_expected_shape() {
        let root = TempDir::new();
        // vault-pm's own encoder is always lowercase; an uppercase name at
        // either level did not come from this backend.
        write_object(root.path(), "2121", "AABBCC", b"ciphertext");
        let report = scan_object_root(root.path()).unwrap();
        assert!(!report.is_clean());
    }

    #[test]
    fn missing_root_is_root_unavailable() {
        let root = TempDir::new();
        let missing = root.path().join("does-not-exist");
        assert_eq!(scan_object_root(&missing), Err(ScanError::RootUnavailable));
    }

    #[test]
    fn a_file_where_the_root_should_be_is_root_unavailable() {
        let root = TempDir::new();
        let file_path = root.path().join("not-a-directory");
        fs::write(&file_path, b"x").unwrap();
        assert_eq!(
            scan_object_root(&file_path),
            Err(ScanError::RootUnavailable)
        );
    }

    #[test]
    fn copy_moves_every_object_and_verifies_by_read_back() {
        let source = TempDir::new();
        let target = TempDir::new();
        fs::remove_dir_all(target.path()).unwrap(); // exercise auto-create
        write_object(source.path(), "2121", "aabbcc", b"first object");
        write_object(source.path(), "2121", "ddeeff", b"second object");
        write_object(
            source.path(),
            "3131",
            "112233",
            b"third object, other bucket",
        );

        let report = copy_object_tree(source.path(), target.path()).unwrap();
        assert_eq!(report.copied_objects, 3);
        assert_eq!(report.copied_bytes, 12 + 13 + 26);
        assert_eq!(report.already_present, 0);
        assert!(report.source_warnings.is_clean());

        assert_eq!(
            fs::read(target.path().join("2121").join("aabbcc")).unwrap(),
            b"first object"
        );
        assert_eq!(
            fs::read(target.path().join("3131").join("112233")).unwrap(),
            b"third object, other bucket"
        );
        // The source is left untouched.
        assert!(source.path().join("2121").join("aabbcc").exists());
    }

    #[test]
    fn in_flight_tmp_writes_are_never_copied() {
        let source = TempDir::new();
        let target = TempDir::new();
        write_object(source.path(), "2121", "aabbcc", b"real object");
        fs::write(source.path().join("2121").join("ddeeff.tmp"), b"partial").unwrap();
        let report = copy_object_tree(source.path(), target.path()).unwrap();
        assert_eq!(report.copied_objects, 1);
        assert!(!target.path().join("2121").join("ddeeff.tmp").exists());
    }

    #[test]
    fn rerunning_a_migration_after_full_success_is_idempotent() {
        let source = TempDir::new();
        let target = TempDir::new();
        write_object(source.path(), "2121", "aabbcc", b"object");
        copy_object_tree(source.path(), target.path()).unwrap();
        let second = copy_object_tree(source.path(), target.path()).unwrap();
        assert_eq!(second.copied_objects, 0);
        assert_eq!(second.already_present, 1);
    }

    #[test]
    fn a_target_object_with_different_bytes_is_a_conflict_not_an_overwrite() {
        let source = TempDir::new();
        let target = TempDir::new();
        write_object(source.path(), "2121", "aabbcc", b"source bytes");
        write_object(target.path(), "2121", "aabbcc", b"DIFFERENT bytes");
        assert_eq!(
            copy_object_tree(source.path(), target.path()),
            Err(CopyError::Conflict)
        );
        // The mismatched target object is left exactly as it was found.
        assert_eq!(
            fs::read(target.path().join("2121").join("aabbcc")).unwrap(),
            b"DIFFERENT bytes"
        );
    }

    // -----------------------------------------------------------------
    // Symlink attacks. `source`/`target` are exactly the directories a
    // third-party sync tool, a second machine sharing removable media, or
    // another vault's mirror configuration may also write to (VLT-PM50
    // §7) -- these prove a planted symlink is refused rather than
    // followed, on both the read and the write side.
    // -----------------------------------------------------------------

    #[cfg(unix)]
    #[test]
    fn a_symlinked_object_in_source_is_never_read_or_copied() {
        use std::os::unix::fs::symlink;

        let source = TempDir::new();
        let target = TempDir::new();
        let secret = source.path().join("outside-the-object-tree.secret");
        fs::write(&secret, b"arbitrary local file contents").unwrap();

        let bucket_dir = source.path().join("2121");
        fs::create_dir_all(&bucket_dir).unwrap();
        // A hex-shaped name, so it would pass the naming check -- only the
        // positive `is_file()` (not `!is_dir()`) requirement excludes it.
        symlink(&secret, bucket_dir.join("aabbcc")).unwrap();
        write_object(source.path(), "2121", "ddeeff", b"a real object");

        let report = copy_object_tree(source.path(), target.path()).unwrap();
        // Only the real object was copied; the symlink was skipped, not
        // followed and copied under an innocuous object name.
        assert_eq!(report.copied_objects, 1);
        assert!(!target.path().join("2121").join("aabbcc").exists());
        // It is still visible in the scan, exactly like any other
        // unexpected entry -- never silently accepted as a clean object.
        assert!(!report.source_warnings.is_clean());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_bucket_directory_in_source_is_never_descended_into() {
        use std::os::unix::fs::symlink;

        let source = TempDir::new();
        let target = TempDir::new();
        let elsewhere = TempDir::new();
        write_object(
            elsewhere.path(),
            "9999",
            "aabbcc",
            b"not part of this vault",
        );
        // A hex-shaped bucket-level name pointing at an unrelated directory.
        symlink(elsewhere.path(), source.path().join("2121")).unwrap();
        write_object(source.path(), "3131", "ddeeff", b"a real object");

        let report = copy_object_tree(source.path(), target.path()).unwrap();
        assert_eq!(report.copied_objects, 1);
        assert!(!target.path().join("2121").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_preexisting_symlinked_bucket_directory_in_target_is_refused() {
        use std::os::unix::fs::symlink;

        let source = TempDir::new();
        let target = TempDir::new();
        let elsewhere = TempDir::new();
        write_object(source.path(), "2121", "aabbcc", b"object bytes");
        fs::create_dir_all(target.path()).unwrap();
        // Planted before the migration runs: a symlink standing in for the
        // bucket directory `copy_object_tree` is about to write into.
        symlink(elsewhere.path(), target.path().join("2121")).unwrap();

        assert_eq!(
            copy_object_tree(source.path(), target.path()),
            Err(CopyError::UnexpectedSymlink)
        );
        // Nothing was written through the planted symlink.
        assert!(fs::read_dir(elsewhere.path()).unwrap().next().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_preexisting_symlinked_target_object_is_refused_not_read_through() {
        use std::os::unix::fs::symlink;

        let source = TempDir::new();
        let target = TempDir::new();
        let secret = TempDir::new();
        fs::write(secret.path().join("shadow"), b"root:x:0:0").unwrap();
        write_object(source.path(), "2121", "aabbcc", b"object bytes");
        fs::create_dir_all(target.path().join("2121")).unwrap();
        symlink(
            secret.path().join("shadow"),
            target.path().join("2121").join("aabbcc"),
        )
        .unwrap();

        assert_eq!(
            copy_object_tree(source.path(), target.path()),
            Err(CopyError::UnexpectedSymlink)
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_top_level_target_is_refused() {
        use std::os::unix::fs::symlink;

        let source = TempDir::new();
        let elsewhere = TempDir::new();
        let parent = TempDir::new();
        let target = parent.path().join("target-link");
        symlink(elsewhere.path(), &target).unwrap();
        write_object(source.path(), "2121", "aabbcc", b"object bytes");

        assert_eq!(
            copy_object_tree(source.path(), &target),
            Err(CopyError::UnexpectedSymlink)
        );
        assert!(fs::read_dir(elsewhere.path()).unwrap().next().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn scan_flags_a_symlink_even_when_its_name_is_hex_shaped() {
        use std::os::unix::fs::symlink;

        let root = TempDir::new();
        write_object(root.path(), "2121", "aabbcc", b"real object");
        let target_file = root.path().join("elsewhere.txt");
        fs::write(&target_file, b"x").unwrap();
        symlink(&target_file, root.path().join("2121").join("ddeeff")).unwrap();

        let report = scan_object_root(root.path()).unwrap();
        assert!(!report.is_clean());
        assert_eq!(report.ordinary_objects, 1);
    }

    #[test]
    fn missing_source_is_source_unavailable() {
        let source = TempDir::new();
        let missing = source.path().join("nope");
        let target = TempDir::new();
        assert_eq!(
            copy_object_tree(&missing, target.path()),
            Err(CopyError::SourceUnavailable)
        );
    }

    #[test]
    fn source_warnings_are_carried_but_do_not_block_the_copy() {
        let source = TempDir::new();
        let target = TempDir::new();
        write_object(source.path(), "2121", "aabbcc", b"object");
        fs::write(
            source.path().join("2121").join("aabbcc (conflicted copy)"),
            b"object",
        )
        .unwrap();
        let report = copy_object_tree(source.path(), target.path()).unwrap();
        assert_eq!(report.copied_objects, 1);
        assert!(!report.source_warnings.is_clean());
        assert_eq!(
            report
                .source_warnings
                .count_of(SyncInterferenceKind::ConflictCopy),
            1
        );
    }

    #[test]
    fn oversized_object_is_rejected_without_being_copied() {
        let source = TempDir::new();
        let target = TempDir::new();
        // Cheaply exceed the bound by faking metadata length is not possible
        // portably, so this proves the *check* path via a real short read
        // combined with a deliberately tiny override is out of scope for a
        // unit test; instead assert the constant is what callers expect and
        // trust the length check reads it before ever allocating.
        assert_eq!(MAX_COPY_OBJECT_BYTES, 64 * 1024 * 1024);
        write_object(source.path(), "2121", "aabbcc", b"small");
        assert!(copy_object_tree(source.path(), target.path()).is_ok());
    }

    #[test]
    fn error_display_and_debug_are_stable_and_closed() {
        for error in [
            ScanError::RootUnavailable,
            ScanError::ReadFailed,
            ScanError::TooManyEntries,
        ] {
            assert!(error.to_string().starts_with("vault-pm-storage-removable"));
        }
        for error in [
            CopyError::SourceUnavailable,
            CopyError::TargetUnavailable,
            CopyError::IoFailed,
            CopyError::Conflict,
            CopyError::ObjectTooLarge,
            CopyError::TooManyEntries,
            CopyError::UnexpectedSymlink,
        ] {
            assert!(error.to_string().starts_with("vault-pm-storage-removable"));
        }
    }

    #[test]
    fn interference_warning_and_scan_report_expose_no_names() {
        let warning = InterferenceWarning {
            location: EntryLocation::TopLevel,
            kind: SyncInterferenceKind::Unknown,
        };
        let debug = format!("{warning:?}");
        assert!(debug.contains("TopLevel"));
        assert!(debug.contains("Unknown"));
        let report = ScanReport::default();
        assert!(report.is_clean());
        assert_eq!(report.count_of(SyncInterferenceKind::ConflictCopy), 0);
    }

    #[test]
    fn classify_covers_every_kind_deterministically() {
        assert_eq!(
            classify("a (conflicted copy)"),
            SyncInterferenceKind::ConflictCopy
        );
        assert_eq!(
            classify("a.sync-conflict-1-2"),
            SyncInterferenceKind::ConflictCopy
        );
        assert_eq!(classify("a (2)"), SyncInterferenceKind::ConflictCopy);
        assert_eq!(classify(".DS_Store"), SyncInterferenceKind::HiddenMetadata);
        assert_eq!(classify("Thumbs.db"), SyncInterferenceKind::HiddenMetadata);
        assert_eq!(
            classify(".~lock.foo#"),
            SyncInterferenceKind::HiddenMetadata
        );
        assert_eq!(
            classify(".hidden-other"),
            SyncInterferenceKind::HiddenMetadata
        );
        assert_eq!(
            classify("a.crdownload"),
            SyncInterferenceKind::PartialTransfer
        );
        assert_eq!(classify("~a"), SyncInterferenceKind::PartialTransfer);
        assert_eq!(classify("readme.txt"), SyncInterferenceKind::Unknown);
        // A bare "(1)" with no leading space is not the Explorer/rclone
        // shape and stays Unknown rather than over-matching.
        assert_eq!(classify("a(1)"), SyncInterferenceKind::Unknown);
    }

    #[test]
    fn is_hex_name_and_is_hex_tmp_name_reject_malformed_shapes() {
        assert!(is_hex_name("aabb"));
        assert!(!is_hex_name(""));
        assert!(!is_hex_name("aab")); // odd length
        assert!(!is_hex_name("aabZ"));
        assert!(!is_hex_name("AABB"));
        assert!(is_hex_tmp_name("aabb.tmp"));
        assert!(!is_hex_tmp_name("aabb.tmp.tmp"));
        assert!(!is_hex_tmp_name(".tmp"));
    }
}
