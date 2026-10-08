# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- **Vault identity** (VLT01 F12, #13980 P1.20d).
  - **Every vault has a random 16-byte id, created by `init`.** It is carried
    in every KEK id: `kek-<n>.<32 hex digits>`.
    - The KEK id is part of every record's and index's wrap AAD. An envelope
      from another vault therefore fails to unwrap, even under the same key,
      and so does one relabelled with this vault's id.
    - Before this, a vault reinitialized under the same KEK accepted the old
      vault's files, and reconcile would absorb them.
    - The record and index formats do not change.
  - **`FreshnessAnchor` gains `vault_id()` and `bind_vault(id, reset)`.**
    - `init` resets the anchor for the new vault, forgetting every epoch,
      before it writes the manifest.
    - Unsealing a different vault under an anchor is `Tamper`. So is
      unsealing a pre-F12 vault under an anchor that records an id. That
      catches a whole older storage directory put back, manifest included.
    - An anchor with no id adopts the first vault it meets.
    - A reset is now just wiping the storage directory. It no longer needs
      a KEK rotation or removing the anchor by hand.
  - **`FileFreshnessAnchor` keeps the id in a strict `vault-id` file.** The
    file holds 32 lowercase hex digits and a newline, and is written like an
    epoch file. A reset deletes only the hex-named epoch files.
  - **A vault made before F12 is rebound on its first unseal under the
    active entry.** The steps:
    1. The active entry is renamed `kek-<n+1>.<id>` and the manifest records
       `rebind_from`.
    2. Every record and index in the registered namespaces is re-wrapped
       with the same key, each with a CAS. Only the DEK wrap changes.
    3. The marker is cleared.

    A crash at any step is resumed by the next unseal. Two processes
    rebinding at once converge on one id through the manifest CAS.
    Unsealing under a retired entry does not rebind, because that is a
    rotation being resumed.
  - **Rotation keeps the vault id**, and KEK numbering counts both id
    formats.
  - **Nothing in the manifest is taken on trust.** It sits in the storage
    directory, so anyone who can write there can rename or add entries and
    fields. Five protections, each from a security-review finding with a
    PoC:
    - A vault-bound entry's verifier binds its id (`"vault-verifier" || 0 ||
      id`), so the active entry cannot be relabelled to another vault, or to
      look pre-F12.
    - Parsing refuses ids this store does not write, more than one active
      entry, and entries naming two vaults. Unseal tries the active entry
      first. A bound vault runs only as its own entries, never as another
      vault's or a pre-F12 id.
    - The rebind marker carries a tag the key makes over the old and new
      ids, and `rebind_from` must be a pre-F12 id. The anchor adopts the
      vault only after the rebind completes, so a marker replayed after that
      is `Tamper`.
    - A process that loses the rebind race re-reads the manifest and runs as
      the renamed entry, never the old id. A re-wrap that loses its CAS
      re-reads the envelope.
    - `init` refuses with `Tamper` when the reserved namespace still holds
      records. A deleted manifest is no longer silently taken as a reset that
      makes every secret unreadable. The manifest is written create-only.
  - From the second review round:
    - A pre-F12 manifest that holds a vault-bound entry is refused.
    - In a bound vault, a record under a pre-F12 retired id is skipped by
      migration instead of blocking the namespace.
    - A marker seen right after a concurrent rebind finished is re-read once
      before it is called a replay.
  - **`FileFreshnessAnchor::bind_vault(reset)` writes the new id before it
    forgets the epochs.** A failure part-way can no longer leave the old id
    with no epochs, which would have accepted the old vault whole.

- **The freshness anchor** (VLT01 F11, #13980 P1.20b). The new
  `FreshnessAnchor` trait, and `SealedStore::with_anchor`, keep each
  namespace's highest index epoch **outside** the storage directory. Under
  an anchor, an index older than the anchor, or a missing index the anchor
  remembers, is `Tamper`, across restarts. That closes F10: restoring an old
  index together with an old record, and deleting the index to restore a v1
  file.
  - `FileFreshnessAnchor` keeps one hex-named file per namespace in an
    owner-only directory, holding a canonical decimal epoch. Writes go to a
    temporary file that is synced, then renamed into place.
  - It refuses symlinks, damaged files, and a directory writable by others.
    A damaged anchor fails closed and is never treated as absent.
  - An authentic index raises the anchor when it loads. A vault written
    before anchoring is therefore protected from its first anchored read
    (trust on first use), and an advance a crash skipped is repaired.
  - Cache hits are checked against the anchor too.
  - Every advance holds an exclusive OS lock (`std::fs::File::lock`), so
    concurrent writers cannot lower the anchor.

- **Freshness: rollback protection for sealed records** (VLT01 F1-F10,
  #13980 P1.20). Before this, someone who could write the storage directory
  could put back an older ciphertext file and it would still verify. That
  undid a rotated secret, a narrowed policy or a delete.
  - Each namespace now has a sealed index at
    `__vault__/freshness/<namespace>`, holding key, generation,
    live/tombstone/legacy state and the live record's AEAD tag. The tag
    means a generation names exactly one record, even if a put ever reuses
    the number.
  - Records are written in format 2, which carries a `generation` bound
    into the body AAD.
  - On `get`, a record older than the index, a resurrected delete, a v1
    file over a migrated record, and a missing index all read as `Tamper`.
  - Writes are crash-safe. `put` writes the record first, and a record
    ahead of its index is accepted. `delete` writes the tombstone first.
  - Every write first reconciles: it absorbs any authentic record that is
    ahead of the index. A stale index, whether from a crashed put or an old
    copy put back, therefore never becomes the floor the write seals. A
    running store also refuses an index older than one it has already seen
    (the epoch floor). Both came from the security review, which showed a
    generation-reuse attack and an index-laundering attack against the first
    draft.
  - `delete` tombstones at one past the reconciled generation, including for
    keys the index has never seen. An uncommitted record hidden during the
    delete therefore cannot come back. The tombstone is never taken from
    plaintext metadata. Generations are bounded to `i64::MAX`.
  - Migration skips a v1 record that does not parse or decrypt, so one bad
    file cannot block writes to its namespace. A record still under a
    retired KEK stops the migration with an error instead, because resuming
    `rotate_kek` recovers it.
  - A namespace written by the previous release migrates on its first
    write, crash-safe at every step. Migration refuses to adopt format-2
    records it finds without an index, because adopting them would launder
    a rollback.
  - `rotate_kek` re-wraps the indexes.
  - What remains (F10): a consistent snapshot of a namespace's records
    *and* its index is still accepted.

### Changed

- Records are now written as `vault_sealed_version: 2`. Version 1 records
  still read until their namespace is first written, which migrates them.
  `delete` now requires the KEK as well, which `unseal` already provides,
  because it writes the sealed index. It also checks `if_revision` before
  writing a tombstone, so a delete that is going to fail cannot hide the
  record.

- `SealedStore::list_page` and `SealedPage`, which keep the backend's
  `next_cursor`. `list` used to drop it, which left a caller that paged through
  a namespace inferring "done" from a short page. That inference is wrong:
  `storage-fs` returns a short page with more still to come whenever a key is
  deleted between its directory scan and its read, so the caller silently
  missed every record after it. `list` now delegates to `list_page`.
- `SealedStore::put_if_absent` for collision-safe encrypted record creation
  without overwriting an existing credential address.
- `SealedStore::init_with_kek` and `SealedStore::unseal_with_kek` for
  caller-owned random root KEKs unwrapped by `vault-key-custody`.
- Additive per-entry KEK source markers with backward-compatible parsing:
  source-less v1 manifests remain password-derived, while injected entries
  omit Argon2 salt and cannot be opened through the password path.
- `SealedEnvelopeSummary` and `SealedStore::summarize` for redacted
  per-record envelope metadata: revision, timestamps, algorithm/version,
  KEK id, and byte counts without ciphertext, wrapped DEK, nonce, tag, or
  AAD bytes.

### Security

- Removed the legacy JSON value/parser chain from the sealed-store dependency
  graph by using the repository-owned bounded RFC 8259 value model directly.
- Raised measured Tarpaulin LLVM line coverage from 88.71% to 96.99%
  (773/797), closing VLT01's declared 95% target with focused rejection tests
  for malformed KEK manifests, sealed-record metadata, JSON field types,
  fixed-width hex, and public error formatting.

## [0.1.0] — 2026-04-22

### Added

- Initial implementation of VLT01 (`code/specs/VLT01-vault-sealed-store.md`).
- `SealedStore` facade wrapping any `storage_core::StorageBackend`.
- Seal / unseal ceremony using Argon2id for key derivation and a
  known-plaintext verifier stored in the manifest.
- Per-record envelope encryption with XChaCha20-Poly1305:
  - Fresh CSPRNG-drawn DEK per record, wrapped under the master KEK.
  - AEAD AAD binds the ciphertext to its `(namespace, key)` slot.
  - Wrapped-DEK AEAD AAD also binds to the KEK id, so swaps across
    KEKs are detected.
- `put` / `get` / `delete` / `list` data-plane operations.
- Sealed-safe status summaries for initialization state, active/retired
  KEK counts, and registered namespace count.
- `rotate_kek` support: re-wraps DEKs under a new master KEK without
  re-encrypting bodies; restartable, crash-safe, and back-compat with
  the retired KEK's password (the retired entry keeps its own salt
  and verifier so both passwords can unseal during and after rotation).
- Reserved namespace `__vault__` for vault-internal records, including
  a namespace registry side record `(__vault__, namespaces)` that
  drives rotation's per-namespace walk.
- In-memory KEK held in a `Zeroizing<[u8; 32]>`; `seal()` and `drop`
  wipe the key. All ephemeral DEKs and KEKs are wrapped in `Zeroizing`
  at allocation so early-return error paths cannot leak key material.
- Hard upper bounds on Argon2id parameters read from the manifest
  (time_cost ≤ 10, memory ≤ 5 GiB, parallelism ≤ 64) — a tampered
  at-rest manifest cannot stall unseal indefinitely.
- `Validation` error messages are sourced only from literals in this
  crate, never from persisted bytes, so they cannot carry attacker
  payloads.
- 22 unit tests covering roundtrips, tamper detection (body corruption,
  body swap, address rewrite), wrong-password rejection, reserved
  namespace rejection, CAS conflicts, empty / large plaintexts, list
  without decryption, re-unseal across store instances, Argon2 param
  bounds (both caller-supplied and manifest-sourced), duplicate-KEK-id
  rejection, namespace-registry tamper defence, and full `rotate_kek`
  across multiple namespaces with retired-KEK recovery semantics.
