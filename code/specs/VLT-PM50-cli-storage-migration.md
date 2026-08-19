# VLT-PM50 — Removable/Synced-Folder Mode and the Mirror Decorator

## Status

Normative Phase 1B contract for VLT-PM00 §23 item 14, "removable/
synced-folder mode and mirror decorator" — the last item of Phase 1B.
Ships `storage add|list|check|migrate`, `StorageKind::Removable`,
`vault-pm-storage-removable`'s conflict-copy detector and migration-copy
helper, and `vault-pm-storage::ReplicaSetObjectStore`. §9 records what is
explicitly deferred and why, and §10 checks Phase 1B's completion against
the master spec.

## 1. Purpose

VLT-PM00 §12's backend table lists `removable/synced folder` as its own
row, right beside `filesystem`, both Phase 1(B):

| Backend | Phase | Notes |
|---|---:|---|
| filesystem | 1 | same immutable format as cloud |
| removable/synced folder | 1B | warn about third-party sync conflict copies |

The two rows share the exact on-disk object format
(`coding_adventures_storage_fs`'s `<root>/<hex namespace>/<hex key>`
layout). `removable` is not a new transport — it is the same filesystem
backend used inside a directory a *third-party* tool also writes to:
Dropbox, OneDrive, Syncthing, a NAS client's sync agent, or a literal USB
drive carried between machines and opened by more than one without
coordination. None of those tools know vault-pm's immutable-object
invariant, and every mainstream one resolves a same-name write collision
by keeping *both* files under different names rather than overwriting —
that renaming convention is the detectable signature this slice looks
for.

§11.5 separately lists `ReplicaSetObjectStore` among the storage
decorators: "publish local objects to configured remote replicas." §19.1
already specifies `storage migrate SOURCE TARGET [--mirror]`'s seven
steps in full, and §19.2 gives the mirroring semantic contract: replicas
receive identical ciphertext, a local commit succeeds independently of
remote availability, and a replica never gains plaintext. This item's job
is to make both of those real.

## 2. What already existed, checked against the real code before writing
##    any of this

Per this campaign's standing finding that the reuse map is right about
pure-parsing reuse and wrong about crypto/storage-layer reuse, both
claims below were verified against the actual workspace, not assumed
from §6's/§11's table text.

- **`storage migrate`/`storage add`/`storage check` were entirely
  unimplemented.** `vault-pm-cli`'s `Command` enum had no `Storage*`
  variant; the four verbs existed only in §14.4's documentation and in
  the fixed `USAGE` string as absent lines. There was no partial
  `migrate` missing only `--mirror` to build on — the whole surface
  needed building.
- **None of §11.5's decorators except `FaultInjectingObjectStore`
  existed anywhere in the workspace.** `grep`ing for
  `RetryingObjectStore`, `RateLimitedObjectStore`, `MetricsObjectStore`,
  `ReplicaSetObjectStore`, and `CachingObjectStore` across
  `code/packages` and `code/programs` found zero implementations —
  every one of those names appeared only in `VLT-PM00`'s own §11.5 table
  text. `vault-pm-storage` (VLT-PM02's storage crate) already contains
  exactly the pattern a new decorator should follow:
  `FaultInjectingObjectStore<S>` wraps any `S: VaultObjectStore` and
  passes the same `run_conformance_suite` the wrapped store does. §4
  below is `ReplicaSetObjectStore<P, M>`, built the same way, in the
  same crate.
- **`vault-pm-repository::RepositoryAddress::derive`** — the function
  that turns a 32-byte key into the two `BucketId`s (`object_bucket`,
  `announcement_bucket`) a vault's repository actually uses — takes
  `keys.locator_key()`, i.e. **key material available only after
  unlocking**, not the plaintext locator visible in configuration. A
  storage-level migration tool therefore cannot legitimately enumerate
  "which buckets does this vault use" without first authenticating,
  which would make `storage migrate` an application-layer operation
  wrapping a storage-layer one. §19.3 already resolves this the other
  way for backups ("byte-for-byte encrypted object repository; safe to
  mirror") — a storage migration works below the bucket-ID boundary,
  on the raw content-blind directory tree, exactly like a filesystem
  `cp -r` would. §5 below follows that precedent: `copy_object_tree`
  copies files, not buckets, and needs no vault-pm-format or
  vault-pm-repository dependency at all.
- **`configured_vault` (in `vault-pm-cli`) hardcoded a vault's storage
  location to one of exactly two paths** the composition root itself
  creates (the default `paths.object_root()`, or a named target's
  `paths.object_root()/targets/<locator-hex>`). That restriction
  predates `storage add` existing — there was no way to point a vault
  anywhere else — and had to be relaxed for this item to do anything at
  all (§6.1).

## 3. Config schema: `StorageKind::Removable`

`coding_adventures_vault_pm_config::StorageKind` gains a fourth variant,
`Removable` (`kind = "removable"` in TOML, alongside the existing
`filesystem`/`gdrive`/`webdav`/`s3`). It is constructed, validated, and
rendered by the exact same closed-schema machinery as `Filesystem` — no
new required field. `StorageKind::is_local_directory()` reports `true`
for `Filesystem` and `Removable` and `false` for the three cloud kinds,
so every call site that needs to say "this is an ordinary local directory
tree" (the storage-location checks below, `storage add`'s kind gate)
names one predicate instead of enumerating two variants each time.

`removable` carries no on-disk difference from `filesystem` — the same
`coding_adventures_storage_fs::FsStorageBackend` opens either. The only
place the distinction matters operationally is that a `removable`
location is *expected* to occasionally show sync-tool interference in
ordinary use, where a `filesystem` location (exclusively written by this
product) finding the same pattern is more surprising. V1 does not encode
that expectation anywhere machine-readable (`storage check`'s report
shape is identical for both kinds); it exists only as the documented
reason a person would choose `removable` over `filesystem` when running
`storage add`.

## 4. The mirror decorator: `ReplicaSetObjectStore<P, M>`

Added to `coding_adventures_vault_pm_storage` (VLT-PM02's storage crate),
next to `FaultInjectingObjectStore`, implementing the same
`VaultObjectStore` contract:

```rust
pub struct ReplicaSetObjectStore<P, M = P> { /* primary: P, mirrors: Vec<M>, .. */ }

impl<P, M> ReplicaSetObjectStore<P, M> {
    pub fn single(primary: P) -> Self;      // zero mirrors
    pub fn new(primary: P, mirrors: Vec<M>) -> Self;
    pub fn replica_health(&self) -> Vec<ReplicaHealth>;
}
```

Behavior, matching §19.2 exactly:

- **`initialize`/`put_immutable`**: the primary call must succeed and its
  result is what the caller sees. Only *after* it succeeds does each
  mirror get the identical call, best-effort — a mirror's failure is
  recorded in its `ReplicaHealth` (`attempted`/`succeeded`/`last_error`)
  and never returned to the caller. This is §19.2's "a local commit
  succeeds independently of remote availability," literally: the mirror
  call cannot even begin until the primary one has already returned
  success.
- **`get`**: primary first; if the primary reports the object missing or
  itself errors, each mirror is tried in order and the first hit is
  returned. This is §19.2's "read fallback verifies all bytes" read the
  way this content-blind layer can honor it: the bytes returned still
  pass through the same content-addressed/AEAD verification one layer up
  regardless of which store answered, so no second verification duty
  belongs here.
- **`list`/`stat`/`delete_unreferenced`/`changes`**: primary-only in this
  slice (§9).
- **`ReplicaSetObjectStore::single(store)`** (zero mirrors) is a verified
  transparent pass-through — it passes the shared 24-check conformance
  suite identically to the wrapped store, so every existing caller that
  predates this item sees no behavior change.

## 5. `vault-pm-storage-removable`: detection and migration copy

A new crate, `code/packages/rust/vault-pm-storage-removable`
(`coding_adventures_vault_pm_storage_removable`), owning the one piece of
real filesystem I/O this item needs, kept out of `vault-pm-storage`
itself (which the crate's own module documentation states has "no I/O
authority").

### 5.1 `scan_object_root(root) -> Result<ScanReport, ScanError>`

Walks `root` and classifies every entry against `storage-fs`'s own
naming shape (even-length lowercase hex; bucket directories at the top
level, object files — or a `<hex>.tmp` in-flight write, cleaned up by the
real backend's own `initialize` — one level down). Anything that does not
match is a warning, classified by *name shape only* (never content) into:

- `ConflictCopy` — contains `"conflict"` case-insensitively (covers
  Dropbox's, OneDrive's, and Google Drive Desktop's `"... conflicted
  copy ..."` wording), Syncthing's fixed `.sync-conflict-<timestamp>-`
  infix, or Explorer/Finder/rclone's bare `" (N)"` duplicate-count
  suffix.
- `HiddenMetadata` — `.DS_Store`, `Thumbs.db`, `desktop.ini`, a
  LibreOffice `.~lock.*#` file, or any other leading-dot name that is not
  this backend's own `<hex>.tmp`.
- `PartialTransfer` — `.crdownload`, `.part`, `.filepart`, `.download`,
  or a `~`-prefixed editor swap file.
- `Unknown` — present, not `.tmp`, matches none of the above.

**No raw filename ever appears in `ScanReport`, an error, or any `Debug`
output.** A third-party sync tool's filename is attacker-adjacent input
the moment a vault is shared or synced anywhere outside one machine, and
this codebase's established convention — `vault-pm-storage`'s own
redacted `Debug` impls, its closed `StoreError` taxonomy carrying no
input bytes — is to never echo attacker-controlled text into a terminal,
log, or error. `ScanReport` exposes only bounded counts by closed
classification (`count_of(kind)`); the CLI (§7) renders those counts and
nothing else.

This is deliberately a *structural*, non-cryptographic check. §7.1's
"malicious storage service" adversary (reorders/duplicates/corrupts/
withholds/deletes/replays objects) is already covered by object-ID
content addressing, AEAD, and signatures at the repository/application
layers above this one; reimplementing content verification here would
duplicate that coverage while breaking `storage-fs`'s own documented
"opaque to record content" boundary for no additional real protection.
This crate's whole job is narrower: notice the *ordinary*, non-
adversarial mess a sync tool leaves behind, and say so.

### 5.2 `copy_object_tree(source, target) -> Result<CopyReport, CopyError>`

The storage-level half of `storage migrate` (§19.1 steps 2-4). Scans
`source` first (§5.1, carried in the report rather than blocking the
copy — a "dirty" source directory is exactly the situation `storage
migrate` off a flaky synced folder exists to get a vault *out of*), then
for every hex-shaped `<bucket>/<object>` file: write-tmp-then-rename into
`target` (the same durability discipline `storage-fs` itself uses),
**read the written bytes back and compare them to the source bytes before
counting the object copied** (§19.1 step 4, "reads/stat-verifies target
objects"). An object already present in `target` with byte-identical
content is skipped and counted separately (`already_present`), so
re-running an interrupted migration is safe; one present with *different*
bytes is `CopyError::Conflict` and aborts rather than silently
overwriting — a genuine immutability violation between two stores that
are each individually supposed to be append-only. `source` is never
modified or deleted.

## 6. CLI wiring

### 6.1 `configured_vault` relaxation

`configured_vault`'s storage-location check (§2) changes from "kind is
exactly `Filesystem` and location is exactly one of two fixed paths" to
"kind's `is_local_directory()` is true and credential_ref is `none`" —
any registered `filesystem`/`removable` location, not just the two this
composition root creates for itself. The cross-vault collision check
(two different vaults must not share one exact filesystem location)
is preserved, generalized the same way. `remote_stores` (VLT-PM07's
existing but previously always-empty replica list field) is no longer
rejected outright: every named remote must itself resolve to a
local-directory-kind, no-credential storage entry, the same rule as the
primary. A cloud primary or a cloud mirror both stay `Unsupported` —
Phase 2's job either way.

### 6.2 Every repository now opens through `ReplicaSetObjectStore`

`repository_factory`/`configured_repository_factory` build
`ReplicaSetObjectStore::single(primary)` when a vault's `remote_stores`
is empty (the default, and every vault before this item shipped) — a
verified no-op — or `ReplicaSetObjectStore::new(primary, mirrors)` when
it is not. This means mirror-write propagation is not limited to the
moment `storage migrate --mirror` runs: every subsequent authenticated
mutation against that vault propagates to its configured mirrors too,
through the ordinary, unmodified application/repository layers, which
know nothing about mirroring — they see one injected `VaultObjectStore`,
as they always have.

### 6.3 Command surface

```text
vault-pm storage add filesystem|removable NAME PATH
vault-pm storage list
vault-pm storage check NAME
vault-pm [--vault NAME] storage migrate SOURCE TARGET [--mirror]
```

Diverges from §14.4's original table in one place, recorded here the way
VLT-PM47 §2.1 recorded `attachment export`'s destination becoming
required: `storage add` takes a required `PATH`, because a storage
location cannot be given a sensible default the way `init`'s two default
names can. `storage add gdrive|webdav|s3 NAME` still parses — every kind
this table has ever documented stays in the grammar — and always fails
closed with the `unsupported` exit class before touching configuration,
the same closed-grammar answer VLT-PM49 §8 gave `import kdbx`: Phase 2
implements the cloud kinds.

`storage add`/`storage list`/`storage check` take no `--vault` selector
(they read or extend configuration itself, joining `init`/`vault
create`/`password generate`'s existing reasons for refusing one).
`storage migrate` does take one — it is the verb that rewrites a specific
vault's `local_store`/`remote_stores` — matching `agent unlock`/`agent
lock`'s existing precedent for a lifecycle-adjacent verb that needs a
target.

### 6.4 `storage migrate`'s confirmation step

§19.1 step 7 requires config to switch "only after explicit
confirmation." Rather than inventing a second ceremony, this slice reuses
one that already exists and already proves the thing that matters: after
`copy_object_tree` succeeds, `storage migrate` collects the vault's real
passphrase and independently unlocks `TARGET` — through
`VaultAccessV1::unlock` over a repository factory pointed *only* at
`TARGET`'s copied objects, the exact machinery `doctor --unlock` already
uses — and only writes configuration if that unlock succeeds. A wrong
passphrase or a corrupted/incomplete copy both fail this exact step,
before configuration is ever touched, which is simultaneously §19.1 step
6 ("opens the target independently... compares... hashes," realized as
"and the unlock either succeeds against real ciphertext or it does not")
and step 7's confirmation — successfully unlocking `TARGET` with the real
passphrase *is* the confirmation, the same answer every other
authenticated command in this grammar already gives to "prove you meant
this."

Without `--mirror`: the vault's `local_store` becomes `TARGET`.
With `--mirror`: `TARGET` is appended to `remote_stores` and
`local_store` is unchanged — the "provider → mirrored configuration"
case §19.1's closing sentence names. `SOURCE` is untouched either way
(step 8's default; no `--delete-source` flag exists in this slice, §9).

### 6.5 `storage check`

Resolves one named storage entry. For a local-directory kind: runs
`scan_object_root` and reports `healthy` (exit 0, no warnings),
`sync_interference_detected` (exit 6, plus a count line per non-zero
classification, e.g. `conflict_copy: 2`) or `unreachable` (exit 7 — the
location does not exist yet, cannot be read, or is genuinely
unmounted; V1 does not distinguish those three, all read as "cannot
confirm this location is healthy right now," which is honest for a
removable location that may simply be unplugged). For a cloud kind: exit
8, unconditionally (Phase 2). For every vault whose `local_store` is the
checked name, one `replica NAME: STATUS` line per configured mirror,
where `STATUS` is `in_sync`, `behind_by_approximately_N_objects`,
`unreachable`, or `unsupported` — a structural heuristic (an object-file
*count* comparison between the primary's and the mirror's own directory
scan), explicitly labeled as such and explicitly not a cryptographic
guarantee (§9).

## 7. Threat-model notes

No new adversary is named for this item; VLT-PM00 §7.1's existing
"malicious storage service" already covers what a *hostile* storage
backend could do, and this item is about a *non-adversarial* third
party's ordinary sync behavior (§1, §5.1). Two places in this slice cross
a trust boundary worth naming explicitly:

- **A mirror target is itself another location a third-party sync tool
  could touch** (a removable/synced folder can be a mirror as easily as
  it can be a primary). `ReplicaSetObjectStore` does not special-case
  this — a mirror is read through the same `get` fallback path as any
  other mirror, and its bytes are subject to the same upper-layer
  content-addressed/AEAD verification as a primary's, regardless of
  which store answered. No additional trust is extended to a mirror
  because it is a mirror.
- **Filenames from a third-party sync tool are parsed and compared inside
  `scan_object_root`.** They are never interpreted as paths beyond
  `std::fs::read_dir`'s own entry, never executed, never formatted into
  an error, and never returned from the crate's public API (§5.1). The
  classification function (`classify`) is a pure, total function over a
  bounded `&str` with no filesystem access of its own.
- **A symlink planted inside `source` or `target` is refused, never
  followed.** Caught by this PR's own security review before merge:
  `DirEntry::file_type()`'s *negative* `!is_dir()` check (the first draft
  of both `scan_object_root` and `copy_object_tree`'s directory walks)
  lets a symlink through as an "ordinary object" whenever its name
  happens to be hex-shaped, and `read_bounded`'s `File::open` follows it —
  so a planted `<hex-name> -> ~/.ssh/id_rsa` inside `source` would have
  been read and copied into `target` under an innocuous object name, a
  real data-exfiltration path when `target` is itself synced or
  cloud-backed (exactly the situation this item's own "provider →
  mirrored configuration" case creates). Fixed by requiring the positive
  `is_file()` everywhere an object is treated as copyable, checking
  `fs::symlink_metadata` (which does not follow the link) before every
  `fs::metadata`/`File::open` call, and opening the migration copy's
  staging file with `OpenOptions::create_new` instead of `File::create`
  to close the TOCTOU window between checking a predictable path and
  writing to it. Six regression tests in `vault-pm-storage-removable`
  cover a symlinked source object, a symlinked source bucket directory,
  and pre-existing symlinks standing in for a target bucket directory, a
  target object, and the top-level target itself.
- **Two different-looking storage locations resolving to one real
  directory.** `configured_vault`'s cross-vault collision check compared
  location strings exactly; once `storage add` allows an arbitrary path
  (rather than one of two fixed ones this composition root creates for
  itself), two different spellings — a relative path, a symlink, a
  trailing separator — could resolve to the same physical directory and
  bypass the check, letting two vaults' object stores silently interleave.
  `same_local_directory` now falls back to comparing `fs::canonicalize`d
  paths when the raw strings differ, closing that gap for any location
  that already exists (one that does not yet cannot collide with anything
  in the first place). A follow-up review round asked whether two
  *concurrent* `vault-pm` processes could each pass this check against
  two not-yet-created aliasing locations before either creates its
  directory; verified against `vault-pm-local-host::LocalWriterGuard`
  that they cannot — it is one non-blocking `try_lock` per config root
  (not per vault), acquired before configuration is even loaded and held
  for the whole command, so a second concurrent invocation against the
  same config root fails closed with `LocalHostError::AlreadyLocked`
  rather than racing past this check.
- The migration-copy read/directory-creation checks (§5.2) close both the
  *static* symlink-planting case (a symlink already present when
  `storage migrate` runs) and the *dynamic* one (a symlink planted in the
  instant between a check and a following read/create call) with true
  kernel-enforced atomic refusals throughout: `O_NOFOLLOW` on Unix
  (`open_no_follow`) for every read, and `create_directory_component_
  atomically` (`fs::create_dir`'s own atomic `AlreadyExists`-if-anything-
  is-there behavior, symlink included) for every directory creation —
  including the top-level `target` directory itself, not only bucket
  directories one level below it. A third review round correctly
  rejected an earlier version of this document's own claim that leaving
  `target`'s creation on a check-then-`create_dir_all` pattern was an
  acceptable residual: `target` is the root the *entire* migration lives
  under, so a race there redirects everything, not one bucket, and
  nothing about it being an operator-configured `storage add` path
  changes that a third party with concurrent write access to `target`'s
  parent directory can still plant a symlink at the exact `target` path
  in the same race window. A bucket directory is now also re-validated
  immediately before every object write, not only once when the bucket
  is first created, closing the analogous narrower window where a
  concurrent writer swaps an already-validated bucket directory for a
  symlink between two writes to it.
- A fourth review round found `MAX_SCANNED_ENTRIES` (§5.1) was enforced
  only after `scan_object_root` had already fully collected and sorted an
  entire directory's raw entries into memory, defeating its own DoS
  purpose against an adversarially padded directory; the bound is now
  checked per entry as it is pulled from the OS, before collection. The
  same round found `target` was re-validated alongside each bucket
  directory (the fix two paragraphs above) but not before *every* object
  write within a bucket the way the bucket directory itself already was
  — closed the same way, by re-checking both together before each write.
- The same round also proposed, implemented, tested, and **reverted** a
  stricter fix: validating every path component of `target` atomically,
  from the filesystem root down, to close the narrower case where an
  attacker plants a symlink at an *ancestor* of `target` that does not
  exist yet (rather than at `target`'s own final component, which was
  already closed). That fix broke on real macOS hosts — `/tmp` and
  `/var` are themselves symlinks to `/private/tmp`/`/private/var` and
  are transparently, legitimately present in the ancestry of essentially
  any absolute path a person could configure, and the stricter check
  could not tell that apart from an attacker-planted one. `ensure_real_
  directory` therefore keeps `fs::create_dir_all` for `target`'s
  parents, recorded as a deliberate scope limit rather than an
  oversight: the residual this leaves can only relocate this
  migration's own freshly written ciphertext to a different physical
  directory the same configured `target` path would still consistently
  resolve through — it cannot read a file that already exists elsewhere
  or overwrite one, which is what every leaf-level check in this section
  exists to prevent for the exact paths this crate reads from and
  writes into.

## 8. Reuse map correction

VLT-PM00 §11.5's decorator list should be read as an aspirational
interface list, not an implemented-and-available-for-reuse one — as of
this item, `ReplicaSetObjectStore` is the second decorator
(`FaultInjectingObjectStore` was the first) to actually exist in
`vault-pm-storage`; `RetryingObjectStore`, `RateLimitedObjectStore`,
`CachingObjectStore`, and `MetricsObjectStore` remain unimplemented and
should not be assumed present by a future slice without checking again
(§2).

## 9. Explicitly deferred

Following this campaign's established practice (Windows named-pipe
support in VLT-PM48 §6, KDBX in VLT-PM49 §8) of shipping a real,
correctly-scoped slice and documenting the rest rather than silently
dropping it:

- **`sync --wait` and its configurable `one`/`all`/quorum durability
  target** (§19.2). What ships here is unconditional best-effort
  write-time propagation plus per-mirror `ReplicaHealth` accounting
  (`vault-pm-storage`) and a directory-scan-based staleness heuristic
  (`storage check`, §6.5) — the building blocks a `sync --wait` ceremony
  would be built on top of, not that ceremony itself.
- **Change-feed-based replica reconciliation.** `storage check`'s replica
  status is an object-*count* comparison between two independent
  directory scans, not a comparison of verified commit heads or catalog
  hashes. It is explicitly labeled a heuristic in both this document and
  the CLI's own output.
- **Physical-delete propagation to mirrors.** `ReplicaSetObjectStore::
  delete_unreferenced` is primary-only; propagating deletion to mirrors
  is left to a future replica-aware GC planner (§19.4), so a mirror never
  loses a still-referenced object ahead of every device having observed
  the pruning checkpoint.
- **`storage migrate --delete-source`.** Not offered in this slice; the
  source is always left untouched (§19.1 step 8's default), and removing
  it is a separate, more destructive decision left to the operator's own
  filesystem tools until a real flag is designed and reviewed.
- **Cloud storage kinds** (`gdrive`/`webdav`/`s3`) for `storage add`,
  `storage migrate`, and mirror targets. Phase 2's job throughout; every
  verb in this slice fails closed with the `unsupported` exit class
  against them rather than silently doing nothing.

## 10. Phase 1B completion check

VLT-PM00 has no dedicated "Phase 1B acceptance criteria" section
analogous to §14.8's Phase 1A one — §23's Phase 1B heading is a plain
numbered list (items 11-14) with no closing acceptance subsection. That
absence is itself worth flagging, the same way this campaign caught and
fixed real Phase 1A completion gaps (crash-recovery wiring, passphrase
rotation, a spec-phase contradiction) before declaring that phase done,
rather than trusting "every numbered item has a checkmark" as sufficient
on its own.

Checked directly against the numbered list and this document plus its
four predecessors:

| Item | Status |
|---|---|
| 11. password generator/TOTP/clipboard/attachments | Shipped (VLT-PM44/45/46/47) |
| 12. local agent/IPC/auto-lock | Shipped (VLT-PM48); Windows named-pipe support explicitly deferred |
| 13. Bitwarden/KDBX/browser CSV import | Shipped (VLT-PM49); KDBX explicitly deferred |
| 14. removable/synced-folder mode and mirror decorator | Shipped (this document); `sync --wait`/quorum durability and cloud kinds explicitly deferred |

Every item is either fully shipped or shipped with an explicit,
documented deferral — no item is silently incomplete. §14.8's own Phase
1A acceptance criteria (one application service/object-store adapter,
crash-safe publication, tamper detection) still hold: this item's
`ReplicaSetObjectStore` wraps the *same* single application-service
composition root with zero behavior change when unconfigured (§4), and
introduces no second mutation or publication path. Phase 1B (daily local
use) is complete under that reading; Phase 2 (cloud) is the next
campaign target per §23's own ordering.

## 11. Acceptance gates

1. `StorageKind::Removable` parses, renders, and round-trips through
   `parse_config`/`render_config`; `is_local_directory()` is true for
   `Filesystem`/`Removable` and false for the three cloud kinds.
2. `ReplicaSetObjectStore::single` passes the shared 24-check
   `run_conformance_suite` identically to the store it wraps; a
   multi-mirror configuration passes it too, reading through the
   primary.
3. A primary commit succeeds and is observable even when every mirror
   is simultaneously failing; `replica_health()` records the failure
   without the caller ever seeing it as an error from `put_immutable`.
4. `scan_object_root` classifies a Dropbox-style, a Syncthing-style, and
   an Explorer/rclone-style conflict-copy filename as `ConflictCopy`; OS
   metadata files as `HiddenMetadata`; partial-download files as
   `PartialTransfer`; and never returns a raw filename anywhere in its
   public API, verified by a test asserting the offending name is absent
   from the formatted report.
5. `copy_object_tree` copies every real object, read-back-verifies each
   one, is idempotent on re-run, and refuses (does not overwrite) a
   target object whose existing bytes differ from the source.
6. `storage add filesystem|removable NAME PATH` registers a location
   without creating it or opening a vault; a duplicate name is refused;
   `gdrive`/`webdav`/`s3` are refused with the unsupported class without
   any configuration change.
7. `storage check` distinguishes an unreachable (not yet materialized)
   location, a healthy one, and one showing sync interference, and
   reports a configured mirror's coarse status.
8. `storage migrate SOURCE TARGET` end to end, through a real vault:
   copies objects, independently unlocks the copy, switches
   `local_store`, and the item created before migration is still
   readable afterward; the source directory is provably untouched.
9. `storage migrate SOURCE TARGET --mirror` end to end: adds `TARGET` to
   `remote_stores` without changing `local_store`, and an item created
   *after* mirroring is created propagates to the mirror through the
   ordinary authenticated write path — proving §6.2's wiring, not only
   the one-time copy.
10. A wrong passphrase during `storage migrate` leaves configuration
    completely unchanged (the copy having already happened is harmless
    and does not need to be redone on retry).
11. A real end-to-end test drives `storage add`/`storage check`/`storage
    migrate` through the actual `vault-pm` executable over a real
    pseudo-terminal, including dropping a real sync-tool conflict-copy
    filename into the migrated location and confirming `storage check`
    reports it without ever printing the filename.
