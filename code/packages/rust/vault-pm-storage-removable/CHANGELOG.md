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
