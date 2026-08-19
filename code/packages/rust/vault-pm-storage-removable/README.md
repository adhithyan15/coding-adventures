# `coding_adventures_vault_pm_storage_removable`

VLT-PM00 §12's `removable/synced folder` backend row, and §23 item 14's
implementation. This crate is not a new storage transport — a removable or
synced folder uses the exact same `storage-fs` on-disk immutable object
format as an ordinary `filesystem` storage entry. The difference is entirely
in what may write to the directory *besides* vault-pm: Dropbox, OneDrive,
Syncthing, a NAS client's sync agent, or a USB drive opened on more than one
machine without coordination. None of those tools know vault-pm's immutable
object invariant, and every mainstream one resolves a same-name write
collision by keeping both files under different names rather than silently
overwriting. This crate detects that pattern and reports it, without trying
to authenticate or defend against a genuinely adversarial storage backend —
that job already belongs to the content-addressed object IDs, AEAD, and
signatures above this layer (VLT-PM00 §7.1).

## What it provides

- `scan_object_root(root)` — walks one object-store root directory and
  reports counts of ordinary vault-pm objects versus entries that look like
  third-party sync interference, classified into `SyncInterferenceKind`
  (`ConflictCopy`, `HiddenMetadata`, `PartialTransfer`, `Unknown`) without
  ever returning a raw filename. Used by `vault-pm storage check NAME` and
  `vault-pm doctor` to warn rather than silently proceed or silently fail.
- `copy_object_tree(source, target)` — copies every committed object file
  from one object-store root to another, write-tmp-then-rename, verified by
  read-back before being counted, for `vault-pm storage migrate SOURCE
  TARGET` (VLT-PM00 §19.1 steps 2-4). Re-running after a partial migration is
  safe: an already-copied, byte-identical object is skipped, and a target
  object with genuinely different bytes is refused as a conflict rather than
  overwritten.

## Symlinks are refused, never followed

Every filesystem touch in this crate — the scan and the migration copy
alike — checks `fs::symlink_metadata` before `fs::metadata`/`File::open`
(which follow links), and the migration copy's staging file is created
with `OpenOptions::create_new` rather than `File::create`, closing the gap
between checking a predictable path and writing to it. This is not a
theoretical hardening: `source`/`target` are exactly the directories a
third-party sync tool, a second machine sharing removable media, or a
configured mirror can also write to, so a hex-named symlink planted there
is a realistic way to read an arbitrary local file into the migrated
copy, or to redirect a write outside the intended object tree.

Reads additionally open with `O_NOFOLLOW` on Unix, and bucket-directory
creation uses the atomic, non-recursive `fs::create_dir` rather than a
check-then-`create_dir_all` pattern — closing not just the case where a
symlink is already present, but the narrower window where one is planted
in the instant between the check and the following call.

## Why no filenames ever leave this crate

A third-party sync tool's filename is attacker-adjacent input the moment a
vault is shared with anyone else or synced through a service outside this
product's control. This codebase's established convention — see
`vault-pm-storage`'s redacted `Debug` implementations and its own error
taxonomy — is to never echo attacker-controlled text into a terminal, log, or
error message. Findings are bounded counts by closed classification only.

## Example

```rust
use coding_adventures_vault_pm_storage_removable::scan_object_root;
use std::path::Path;

let report = scan_object_root(Path::new("/media/usb/my-vault"))?;
if !report.is_clean() {
    eprintln!("{} entries look like sync-tool interference", report.warnings.len());
}
# Ok::<(), coding_adventures_vault_pm_storage_removable::ScanError>(())
```

## Deliberate exclusions

No cryptography and no content interpretation — a scan inspects only names.
`copy_object_tree` reads bytes solely to compare them for equality or to
relocate them unchanged; it never decodes an object frame. Neither function
resolves a name back to an opaque `BucketId`/`ObjectId` — that boundary stays
inside `vault-pm-storage`, and this crate works one layer below it, directly
on `storage-fs`'s own even-length-lowercase-hex filename shape.

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_storage_removable --all-targets -- -D warnings
```
