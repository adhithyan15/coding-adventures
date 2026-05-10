# VLT13 — Vault Encrypted Search

## Overview

Local trigram index + BM25 ranker. The vault is E2EE — the
server cannot help with search — so search runs entirely on
the client. The index lives in memory while the vault is
unlocked and is persisted as ordinary vault records (encrypted
under VLT01 sealed-store) so it survives restarts and rides
along with normal sync (VLT10).

Implementation lives at `code/packages/rust/vault-search/`.

## Why this layer exists

A 100-row password manager doesn't need an index. A 5,000-row
machine-secret store with 30-character namespaces does — a
linear scan on every keystroke is unbearable on a phone.

The index is a *local* primitive: the host indexes whatever
fields the schema (VLT02) declares searchable, persists the
index alongside the data, and rebuilds it on unlock. Because
the persisted bytes go through VLT01 sealed-store, an attacker
with access to the storage server sees only ciphertext. Local
disk forensics on a locked vault sees the same.

## Surface

```rust
pub struct DocumentId(/* opaque */);
impl DocumentId { pub fn new(s) -> Result<Self, SearchError>; }

pub struct SearchableFields {                 // per-schema
    pub weights: BTreeMap<String, f32>,
}
impl SearchableFields {
    pub fn new() -> Self;
    pub fn with(self, field, weight: f32) -> Self;
}

pub struct SearchHit { pub id: DocumentId, pub score: f32 }

pub enum SearchError {
    InvalidParameter(&'static str),
    TooLarge(&'static str),
    Decode(&'static str),
}

pub struct SearchIndex { /* in-memory, threadsafe */ }
impl SearchIndex {
    pub fn new() -> Self;
    pub fn index(&self, id: DocumentId,
                 fields: &BTreeMap<String, String>,
                 searchable: &SearchableFields)
        -> Result<(), SearchError>;
    pub fn remove(&self, id: &DocumentId) -> Result<(), SearchError>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn search(&self, query: &str, top_n: usize)
        -> Result<Vec<SearchHit>, SearchError>;
    pub fn clear(&self);
    pub fn to_bytes(&self) -> Vec<u8>;
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SearchError>;
}
```

## How it works

- **Trigrams**: every 3-byte ASCII window of every indexed
  field (lowercased) becomes a posting. "Foo" ⇒ `foo`;
  "abcabc" ⇒ `abc, bca, cab, abc`. Substring matches ("git"
  matches "github" *and* "digital") are natural.
- **BM25**: standard `k1=1.2, b=0.75`. `tf` is the per-doc
  trigram count weighted by the field's declared weight;
  `idf = ln((N - df + 0.5) / (df + 0.5) + 1.0)`; document
  length normalisation against `avg_dl`.
- **Persistence**: `to_bytes()` produces a 4-byte magic
  `"VSI1"` + version 1 + length-prefixed entries. The host
  wraps it in a `vault_sealed_store::Sealed` and writes it
  through whatever storage backend it uses.

## Per-schema searchability

`SearchableFields` is an explicit allow-list. A field that the
caller never declares is never indexed. This is the
load-bearing security property:

> A stolen and decrypted index file gives up titles and URLs
> but NOT passwords — because passwords were never put into
> the index.

Test `search_does_not_index_undeclared_fields` exercises this.

## Bounds

| Field                    | Cap                |
|--------------------------|--------------------|
| `MAX_INDEXED_FIELD_LEN`  | 64 KiB             |
| `MAX_INDEXED_DOCS`       | 1,000,000          |
| `MAX_FIELDS_PER_DOC`     | 64                 |
| `MAX_DOC_ID_LEN`         | 256 bytes          |
| `MAX_QUERY_LEN`          | 4 KiB              |

## Threat model & test coverage

| Threat                                                              | Defence                                                  | Test                                                            |
|---------------------------------------------------------------------|----------------------------------------------------------|-----------------------------------------------------------------|
| Server reads plaintext index                                        | host wraps `to_bytes()` in VLT01 before persistence      | structural — to_bytes returns opaque `Vec<u8>`                 |
| Plaintext trigrams linger in heap after lock                        | `clear()` and `Drop` walk per-doc tf maps and zeroize    | `clear_drops_documents`, `drop_runs_without_panic_after_indexing` |
| Index is built over un-declared fields (e.g. passwords)             | `SearchableFields` is an explicit allow-list             | `search_does_not_index_undeclared_fields`                       |
| Caller cheats the ranker via huge or NaN weight                     | weight ≤ 0, NaN, or infinite is silently dropped         | `zero_or_negative_weight_skipped`                               |
| Oversize field input blows up the indexer                           | `MAX_INDEXED_FIELD_LEN` rejects pre-state                | `index_rejects_oversize_field`                                  |
| Index growth without bound                                          | `MAX_INDEXED_DOCS`, `MAX_FIELDS_PER_DOC` capped          | `index_rejects_too_many_fields`                                 |
| Oversize query exhausts CPU                                         | `MAX_QUERY_LEN` rejection                                | `search_rejects_oversize_query`                                 |
| Re-index leaves stale postings for an id                            | old per-doc tf is removed from postings before re-insert | `re_indexing_replaces_prior_postings`                           |
| `from_bytes` accepts truncated or malformed input                   | strict reader: bad magic / version / sizes / NaN / EOF   | `from_bytes_rejects_bad_magic`, `from_bytes_rejects_unsupported_version`, `from_bytes_rejects_truncated`, `from_bytes_rejects_trailing_bytes`, `from_bytes_rejects_non_finite_tf_weight`, `from_bytes_rejects_oversize_doc_count` |
| Trojan Source / bidi-override in `DocumentId`                       | reject U+202A–U+202E, U+2066–U+2069, U+200B–U+200D, U+FEFF | `doc_id_rejects_control_chars`                                |
| `DocumentId` overflow / control chars                               | length cap + control / whitespace rejection              | `doc_id_rejects_oversize`, `doc_id_rejects_empty`               |
| Mutex poisoning silently DoSes the index                            | `lock_recover` via `PoisonError::into_inner`             | structural — invariants verified coherent                       |
| Concurrent indexers race on postings                                | mutex-guarded mutation                                   | `concurrent_indexers_all_succeed` (16 threads)                  |
| BM25 score is non-finite                                            | `idf` formula keeps it ≥ 0, finite                        | `bm25_score_is_finite_and_non_negative`                         |
| Search returns inconsistent order across replicas                   | tie-break on id ascending after score descending         | `search_results_sorted_descending_by_score`                     |
| **Decompression bomb in `from_bytes` via inflated `tf_count`**      | per-doc cap + remaining-input cross-check before allocation | `from_bytes_rejects_tf_count_above_per_doc_cap`, `from_bytes_rejects_inflated_tf_count` |
| **Inflated `n_docs` triggers premature allocation**                 | `n_docs * MIN_DOC_BYTES > remaining` cross-check         | `from_bytes_rejects_inflated_n_docs`                            |
| **Crafted `total_len` manipulates BM25 ranking + overflows sum**    | per-doc `MAX_TOTAL_LEN` cap; `saturating_add` fold       | `from_bytes_rejects_oversize_total_len`                         |
| `Drop` skipped scrub when mutex was poisoned (panic mid-search)     | `get_mut()` instead of `lock()` — `&mut self` is exclusive | structural — invariants verified coherent                       |

## Out of scope (future PRs)

- **Server-side searchable encryption (SSE)**. v1 is local-
  only; SSE is much stronger but operationally heavy.
- **Stemmer / language-aware tokenizer**.
- **Phrase / boolean queries**.
- **Field-level highlighting** — the host fetches the matching
  record and renders.
- **Forward-secure search** — at-rest sealing protects the
  index until the master key falls; forward-secure SSE
  primitives are future work.

## Citations

- VLT00-vault-roadmap.md — VLT13 placement.
- Robertson + Walker, *Some simple effective approximations to
  the 2-Poisson model for probabilistic weighted retrieval*
  (BM25).
- Manning et al., *Introduction to Information Retrieval* —
  trigram-based candidate-set search.
- VLT01-vault-sealed-store — what wraps the persisted index.
- VLT02-vault-records — supplies the `(field_name, weight)`
  schema declarations.
- VLT09-vault-audit-log — records search events if the host
  chooses (this crate stays silent).
- VLT10-vault-sync-engine — propagates index records across
  devices.
