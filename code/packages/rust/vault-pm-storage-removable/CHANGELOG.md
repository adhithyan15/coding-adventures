# Changelog

All notable changes to this package are documented here.

## [0.1.0] - 2026-08-19

### Added

- `scan_object_root`, structural detection of third-party sync-tool
  interference in a `storage-fs`-shaped object-store root: conflict copies
  (Dropbox/OneDrive/Google-Drive-Desktop "conflicted copy" naming, Syncthing's
  `.sync-conflict-` infix, Explorer/Finder/rclone's `" (N)"` duplicate-count
  suffix), OS/client hidden metadata (`.DS_Store`, `Thumbs.db`,
  `desktop.ini`, LibreOffice lock files), partial transfers (`.crdownload`,
  `.part`, `.filepart`, `.download`, `~`-prefixed swap files), and an
  `Unknown` catch-all.
- `copy_object_tree`, a byte-for-byte object-root copy for `storage migrate`
  (VLT-PM00 §19.1): write-tmp-then-rename per object, read-back verified
  before being counted, idempotent on re-run, and refusing (not silently
  overwriting) a target object whose existing bytes differ from the source.
- `ScanReport`/`CopyReport` with bounded, name-free findings, and closed
  `ScanError`/`CopyError` taxonomies.
- `MAX_SCANNED_ENTRIES` and `MAX_COPY_OBJECT_BYTES` bounds against an
  adversarially large or padded removable-drive directory.

### Security

- No filename from a scanned or copied directory ever appears in this
  crate's public API, `Debug` output, or error `Display` text — findings are
  reported as bounded counts by closed classification only, matching this
  codebase's convention of never echoing attacker-controlled text.
- `copy_object_tree` never modifies or deletes anything under `source`, and
  refuses (`CopyError::Conflict`) rather than overwrites when a target object
  already exists with different bytes than the source.
- Every read is bounded by `MAX_COPY_OBJECT_BYTES` before allocation, and
  every directory walk is bounded by `MAX_SCANNED_ENTRIES`.
- **Symlinks are refused, never followed, everywhere this crate touches the
  filesystem** (`CopyError::UnexpectedSymlink`). `source`/`target` are
  exactly the directories a third-party sync tool, a second machine sharing
  removable media, or a mirror configuration may also write to, so an
  attacker-planted symlink here is a real, not merely theoretical, threat:
  a hex-shaped symlink to an arbitrary local file (e.g. `~/.ssh/id_rsa`)
  inside `source` would otherwise be read and copied into `target` under
  an innocuous object name — a data-exfiltration primitive if `target` is
  itself synced or cloud-backed — and a symlink standing in for a bucket
  directory or an object file inside `target` would otherwise let
  `fs::create_dir_all`'s or `File::create`'s symlink-following behavior
  redirect a write to an attacker-chosen location. Every read and every
  directory-creation check now inspects `fs::symlink_metadata` (which does
  not follow the link) before ever calling `fs::metadata`/`File::open`
  (which do); the migration-copy staging file is opened with
  `OpenOptions::create_new` rather than `File::create`, closing the
  TOCTOU window between checking a predictable staging path and writing to
  it. `scan_object_root` also now positively requires `is_file()` (not a
  negative `!is_dir()` check) before counting an entry as an ordinary
  object, so a symlink is reported as an unexpected entry — never silently
  accepted as healthy — even when its name happens to be hex-shaped. Found
  in this PR's own security review and fixed before merge; six regression
  tests cover a symlinked source object, a symlinked source bucket
  directory, a pre-existing symlinked target bucket directory, a
  pre-existing symlinked target object, a symlinked top-level target, and
  `scan_object_root`'s own detection of a hex-named symlink.
- **Second review round: narrowed the remaining check-then-use race
  window.** The fix above closes the *static* case (a symlink already
  present when a scan/copy runs) completely, but a second review round
  correctly noted that a `symlink_metadata` check followed by a separate
  `File::open`/`fs::create_dir_all` call cannot, on its own, close the
  *dynamic* case (a symlink planted in the instant between the two).
  `read_bounded` now opens with `O_NOFOLLOW` on Unix (`open_no_follow`),
  making the kernel itself refuse a symlink at that exact instant rather
  than racing a userspace check against it (`libc` is now a Unix-only
  dependency). Bucket-directory creation (always exactly one path
  component below an already-validated `target`) now uses the atomic,
  non-recursive `fs::create_dir` (`ensure_real_bucket_directory`) instead
  of a check-then-`create_dir_all` pattern, closing the same class of
  window there. The top-level `target` directory still uses
  check-then-`create_dir_all` (it needs recursive parent creation);
  documented in `VLT-PM50-cli-storage-migration.md` §7 as an accepted,
  narrower residual, since `target` is an operator-configured path via
  `storage add`, not content discovered by scanning an untrusted
  directory.
- **Third review round: the "accepted residual" from the round-2 note
  above was itself a real HIGH-severity gap, not a narrow one — fixed.**
  `ensure_real_directory` (the top-level `target` check) still used
  check-then-`create_dir_all`, and unlike a bucket directory, `target` is
  the root every bucket and every object in the whole migration lives
  under; a symlink raced into that exact path redirects the *entire*
  migration, not one bucket. Fixed by extracting the atomic
  `fs::create_dir` + `AlreadyExists`/`symlink_metadata` pattern into
  `create_directory_component_atomically`, shared by both
  `ensure_real_directory` (which now only uses ordinary
  `fs::create_dir_all` for `target`'s *parents*, then creates `target`
  itself atomically) and `ensure_real_bucket_directory`. Also fixed: a
  bucket directory was validated once per bucket rather than once per
  object, leaving a window for a concurrent writer to swap it for a
  symlink between two writes to the same bucket — `copy_object_tree` now
  re-validates the bucket directory immediately before every object
  write. Also fixed: `read_bounded`'s `read_to_end` had no cap of its
  own, so a file a concurrent writer kept appending to could grow the
  in-memory buffer past `MAX_COPY_OBJECT_BYTES` before the post-read
  length check fired; now capped during the read with `Read::take`. Two
  new regression tests: a bucket directory replaced with a symlink
  between two writes is caught, and nested missing parent directories
  are still created correctly (a correctness check on the refactor, not
  a new attack).
