# `coding_adventures_vault_revisions` — VLT12 revision history

Every `put` archives the prior ciphertext to a sibling history
list keyed by `(namespace, key)`. `restore(ns, key, rev)` brings
back an old revision as a *new* write — the history list stays
append-only.

This is the layer behind 1Password's "version history",
Bitwarden's password history, HashiCorp Vault KV-v2's
versioning, and `pass`'s `git log` integration.

## Quick example

```rust
use coding_adventures_vault_revisions::{
    InMemoryRevisionStore, RetentionPolicy, RevisionStore,
};

let store = InMemoryRevisionStore::new();
store.set_policy("kv", RetentionPolicy::default_password_manager());

// Each upper-level `put` calls archive() with the *prior*
// ciphertext and the new wall-clock time.
store.archive("kv", "login/github", b"old-ciphertext".to_vec(), 1_700_000_000_000)?;

// Later — list and restore.
let metas = store.list("kv", "login/github")?;
let rev = metas.last().unwrap().id;
let restored = store.restore("kv", "login/github", rev, 1_700_001_000_000)?;
// restored.ciphertext == b"old-ciphertext", appended as a new revision
```

## Retention policy

Per-namespace, two complementary caps:

| Field                    | Behaviour                                          |
|--------------------------|----------------------------------------------------|
| `max_revisions_per_key`  | oldest evicted on every archive (None → unbounded) |
| `max_age_ms`             | applied via `purge_due(now_ms)` (None → unbounded) |

Defaults (`RetentionPolicy::default_password_manager()`): 32
revisions per key, 90 days max age.

## Threat model

- **Storage-agnostic.** Server sees ciphertext bytes only.
  Sealing is VLT01's job above.
- **Append-only.** `restore` does not mutate prior revisions; it
  appends the restored ciphertext as a new one. Truncation /
  rewrite of stored revisions is detected by VLT09 audit log.
- **Bounded sizes.** Every variable-length field is capped:
  `MAX_NAMESPACE_LEN` (128), `MAX_KEY_LEN` (512),
  `MAX_CIPHERTEXT_LEN` (1 MiB).
- **Control-char + bidi defence.** Namespace and key reject
  control chars, whitespace, U+202A–U+202E / U+2066–U+2069
  (Trojan Source / CVE-2021-42574), and U+200B–U+200D / U+FEFF
  (zero-width / BOM) — same defence posture as `vault-sync`
  and `vault-transport-cli`.
- **`Revision` `Debug` is redacted.** Hand-rolled — bytes never
  appear in logs from a stray `dbg!`.
- **Mutex poisoning recovery.** A panic in any thread does not
  permanently DoS the store.

## Where it fits

```text
   ┌──────────────────────────────────────────────┐
   │  application                                 │
   └──────────────────┬───────────────────────────┘
                      │ "put X"
   ┌──────────────────▼───────────────────────────┐
   │  RevisionStore::archive(prior_ciphertext)    │  (this crate)
   └──────────────────┬───────────────────────────┘
                      │
   ┌──────────────────▼───────────────────────────┐
   │  storage backend                              │
   │   (in-memory ref / vault-revisions-fs /       │
   │    vault-revisions-postgres / …)              │
   └───────────────────────────────────────────────┘
```

## What this crate is NOT

- Not encryption — VLT01 sealed-store is above.
- Not auth / policy — VLT05/VLT06 above.
- Not audit — VLT09. The chain of archive events is recorded
  there.
- Not a queryable index — VLT13 (encrypted search) handles
  search; this crate just keeps the chronological history.
- Not a delete primitive — destruction of a key's history
  belongs in a higher tier with an explicit "destroy"
  capability gated by VLT06.

## Capabilities

None — pure data structures. See `required_capabilities.json`.

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT12-vault-revision-history.md`](../../../specs/VLT12-vault-revision-history.md).
