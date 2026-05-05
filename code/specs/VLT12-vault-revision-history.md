# VLT12 — Vault Revision History

## Overview

Every `put` at the application tier archives the *prior*
ciphertext to a sibling history list keyed by
`(namespace, key)`. `restore(ns, key, rev)` brings back an old
revision as a new write — the history list stays append-only.

This is the layer behind:

- 1Password's "version history",
- Bitwarden Premium's password history,
- HashiCorp Vault KV-v2's versioning,
- `pass`'s `git log` integration.

Implementation lives at `code/packages/rust/vault-revisions/`.

## Why this layer exists

Users mistype passwords. Sites rotate them silently. Compliance
asks "what did this credential look like 30 days ago?". A vault
without a history layer answers none of those. A vault with a
history layer answers them all.

The crate is intentionally narrow: per-`(namespace, key)`
history list + retention policy + `restore` semantics. Sealing
(VLT01), policy (VLT06), audit (VLT09), search (VLT13), and
sync (VLT10) are siblings.

## API

```rust
pub struct Revision {
    pub id: u64,
    pub archived_at_ms: u64,
    pub ciphertext: Vec<u8>,    // OPAQUE — sealed at VLT01 above
}

pub struct RevisionMeta {
    pub id: u64,
    pub archived_at_ms: u64,
    pub ciphertext_len: usize,
}

pub struct RetentionPolicy {
    pub max_revisions_per_key: Option<usize>,
    pub max_age_ms: Option<u64>,
}
impl RetentionPolicy {
    pub fn default_password_manager() -> Self;  // 32 revs, 90 days
    pub fn unbounded() -> Self;                 // keep everything
}

pub trait RevisionStore: Send + Sync {
    fn archive(&self, namespace: &str, key: &str,
               ciphertext: Vec<u8>, archived_at_ms: u64)
        -> Result<Revision, RevisionError>;
    fn list(&self, namespace: &str, key: &str)
        -> Result<Vec<RevisionMeta>, RevisionError>;
    fn get_revision(&self, namespace: &str, key: &str, id: u64)
        -> Result<Revision, RevisionError>;
    fn restore(&self, namespace: &str, key: &str,
               id: u64, archived_at_ms: u64)
        -> Result<Revision, RevisionError>;
    fn purge_due(&self, namespace: &str,
                 retention: &RetentionPolicy, now_ms: u64)
        -> Result<usize, RevisionError>;
    fn policy_for(&self, namespace: &str) -> RetentionPolicy;
    fn set_policy(&self, namespace: &str, policy: RetentionPolicy)
        -> Result<(), RevisionError>;
}

pub struct InMemoryRevisionStore;  // reference implementation
```

## Semantics

- **Archive** — assigns the next monotonic id at this
  `(namespace, key)` (starts at 1). Applies the namespace's
  retention policy (oldest evicted on overflow).
- **List** — returns metadata only (`RevisionMeta`) so the
  cheap "show me the versions" call doesn't copy ciphertext
  bytes. Sorted ascending by id.
- **`get_revision`** — opt-in fetch of one revision's
  ciphertext.
- **Restore** — fetch revision `id`, then `archive` its
  ciphertext as a *new* revision. Append-only: the original
  revision stays in the history.
- **`purge_due(now_ms)`** — caller-driven sweep: evicts
  revisions older than `now_ms - max_age_ms` (no-op if
  `max_age_ms` is `None`). Crate has no built-in timer.

## Bounds (tested)

| Field             | Cap          |
|-------------------|--------------|
| `namespace`       | 128 bytes    |
| `key`             | 512 bytes    |
| `ciphertext`      | 1 MiB        |

Capping `ciphertext` at 1 MiB matches the sync layer (VLT10)
and bounds memory growth from a malicious caller. Larger
payloads chunk through VLT14 attachments.

## Threat model & test coverage

| Threat                                                          | Defence                                                  | Test                                                  |
|-----------------------------------------------------------------|----------------------------------------------------------|-------------------------------------------------------|
| Server / store reads plaintext                                  | ciphertext is opaque `Vec<u8>`; sealing is VLT01's job   | structural                                            |
| Storage tampering rewrites a historical revision                | detected by VLT09 audit log (chain of archive events)    | structural — out of this crate                        |
| Caller passes empty / oversize / control-char namespace         | `validate_namespace_key` rejects                         | `rejects_empty_namespace`, `rejects_oversize_namespace`, `rejects_namespace_with_control_chars` |
| Trojan Source / bidi-override (CVE-2021-42574) in namespace/key | reject U+202A–U+202E, U+2066–U+2069, U+200B–U+200D, U+FEFF | `rejects_key_with_bidi_override`                  |
| Caller passes empty / oversize ciphertext                       | `validate_ciphertext` rejects                            | `rejects_empty_ciphertext`, `rejects_oversize_ciphertext` |
| Replay floods exhaust storage                                   | `max_revisions_per_key` cap, oldest evicted              | `max_revisions_evicts_oldest_first`, `policy_default_caps_after_archive` |
| Compliance retention required indefinitely                      | `RetentionPolicy::unbounded()` keeps everything          | `unbounded_policy_keeps_everything`                   |
| `restore` mutates / loses prior revisions                       | `restore` appends as a new revision, never in-place      | `restore_appends_new_revision`                        |
| Out-of-namespace purge accidentally drops other namespaces      | `purge_due` filters by namespace                         | `purge_due_only_affects_target_namespace`             |
| `Revision` `Debug` leaks ciphertext bytes                       | hand-rolled redacted `Debug` (lengths only)              | `revision_debug_redacts_ciphertext`                   |
| Concurrent archives mint duplicate ids                          | `Mutex<History>` + `checked_add` for next_id             | `concurrent_archives_all_get_unique_ids` (16 threads) |
| Mutex poisoning silently DoSes the store                        | `lock_recover` via `PoisonError::into_inner`             | structural — invariants verified coherent             |
| `id` counter overflow on a 64-bit history                       | `checked_add(1)` returns `RevisionError::Overflow`       | structural                                            |
| `get_revision` for unknown path leaks "exists?" oracle          | uniform `NotFound` for unknown path; `UnknownRevision` for known path + missing id | `get_revision_unknown_path_returns_not_found`, `get_revision_unknown_id_returns_unknown_revision` |
| **`set_policy` accepted unvalidated namespace strings**         | `set_policy` returns `Result`; validates via shared helper | `set_policy_rejects_invalid_namespace`               |
| **Footgun: `max_revisions_per_key = Some(0)` silently lost data** | rejected at `set_policy` boundary                        | `set_policy_rejects_zero_max_revisions`              |
| `purge_due` skipped namespace length cap                        | switched to shared `validate_namespace`                  | `purge_due_rejects_oversize_namespace`               |
| `policy_for(unknown_ns)` panics or surfaces nonsense            | infallible, falls back to default                        | `policy_for_unknown_namespace_returns_default`       |
| `now_ms = u64::MAX` to `purge_due` evicts everything            | documented as privileged maintenance call (host sources from trusted clock) | structural / doc                            |

## Out of scope (future PRs)

- Persistent stores (`vault-revisions-fs`,
  `vault-revisions-postgres`). Each implements the same trait.
- Re-wrap on `restore` for key-rotation flows. The crate hands
  back historical ciphertext as-is; the application or a thin
  VLT01 adapter handles re-wrap.
- GC of an entire key's history (a destructive op gated by
  VLT06).
- Time-range / actor query indexes (VLT13 encrypted search).
- Cross-tier hash-chained signatures over the archive sequence
  (VLT09 records that already; layered, not duplicated).

## Citations

- VLT00-vault-roadmap.md — VLT12 placement.
- HashiCorp Vault KV-v2 — versioned KV semantics.
- 1Password "version history".
- Bitwarden Premium password history.
- VLT01-vault-sealed-store — produces the ciphertext this layer
  archives.
- VLT09-vault-audit-log — records the archive events.
- VLT10-vault-sync-engine — propagates revisions across devices.
