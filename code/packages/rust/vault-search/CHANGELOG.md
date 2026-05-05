# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT13
  (`code/specs/VLT13-vault-encrypted-search.md`).
- `DocumentId` — opaque newtype around a `String`. Validates
  empty/oversize and rejects control chars + Unicode bidi-
  override / zero-width (CVE-2021-42574).
- `SearchableFields` — `BTreeMap<field_name, weight>` with a
  fluent `with(field, weight)` builder. Weight ≤ 0, NaN, or
  infinite is silently dropped at index time so a malicious
  caller cannot cheat the ranker.
- `SearchHit` — `(DocumentId, score: f32)`. Scores ordered
  descending; ties broken by id ascending so order is
  deterministic across replicas.
- `SearchError` — `InvalidParameter` / `TooLarge` / `Decode`.
- `SearchIndex` — `Send + Sync`, threadsafe via `Mutex`. Public
  API: `new` / `index` / `remove` / `len` / `is_empty` /
  `search` / `clear` / `to_bytes` / `from_bytes`.
- Trigram extractor — lowercases ASCII; treats other bytes
  passthrough; total, allocates once.
- BM25 ranker — standard `k1=1.2, b=0.75` over the candidate
  set (union of posting lists for every query trigram). Uses
  per-document trigram counts (weighted by the field's
  declared weight) for `tf`; uses global posting-list size for
  `idf`. Score is finite and non-negative.
- Wire serialization — `to_bytes()` produces a 4-byte magic
  `"VSI1"` + version 1 + length-prefixed doc/posting list.
  `from_bytes()` is strict: rejects bad magic, unsupported
  version, oversize doc / tf counts, truncated input,
  trailing bytes, non-finite or negative `tf` weights,
  invalid-UTF-8 doc IDs, and doc IDs that fail
  `DocumentId::new`.
- `clear()` and `Drop` walk every doc, calling
  `Zeroize::zeroize` on each per-doc trigram map before the
  `BTreeMap` releases its backing storage. Plaintext trigrams
  do not linger in the heap after the vault locks.
- Mutex poisoning is recovered via `PoisonError::into_inner`
  (consistent with `vault-audit`, `vault-revisions`,
  `vault-sync`).
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.
- 37 unit tests covering: trigram extraction (lowercase /
  short input / overlapping counts), index → search hits,
  weighted ranking (title outranks url for the same match),
  the load-bearing privacy property
  (`search_does_not_index_undeclared_fields`),
  re-indexing replaces prior postings, remove purges
  postings, idempotent remove, search below trigram length
  returns empty, top-N caps results, scores sorted
  descending, search on empty index, validation rejection
  of oversize / control / bidi DocumentId, oversize field /
  too-many-fields / oversize query, round-trip via
  `to_bytes`/`from_bytes`, every `from_bytes` rejection
  path (bad magic, unsupported version, trailing bytes,
  oversize doc count, truncated, non-finite tf weight),
  `clear` drops documents, drop-after-indexing runs without
  panic, `Send + Sync`, 16-thread concurrent indexers, BM25
  finite-and-non-negative, missing field is silently
  skipped, zero/negative/NaN weight skipped.

### Security hardening (pre-merge review)

Three findings flagged before push, all fixed inline:

- **HIGH** — `from_bytes` allocated `HashMap::with_capacity(tf_count)`
  using attacker-controlled `tf_count` after only a constant
  cap check. A 21-byte payload claiming `tf_count = 4M` would
  drive a 256-MiB allocation before any actual entry was read
  (decompression-bomb). Fix: tightened the per-doc cap to
  `MAX_TF_ENTRIES_PER_DOC = 65_536`, and reject any `tf_count`
  whose claimed entries exceed the remaining input bytes
  (`tf_count * 7 > p.remaining()`). Same defence applied at
  the `n_docs` level (`n_docs * MIN_DOC_BYTES > p.remaining()`).
- **HIGH** — `from_bytes` accepted arbitrary `total_len: u64`,
  letting a crafted persisted index manipulate BM25 ranking
  *and* drive a `u64` sum overflow in `search`'s `avg_dl`
  computation (panic in debug, wrap in release). Fix: bound
  `total_len <= MAX_TOTAL_LEN = MAX_INDEXED_FIELD_LEN *
  MAX_FIELDS_PER_DOC` per doc, and switched the in-memory sum
  to `fold(0u64, u64::saturating_add)` for defence-in-depth.
- **MEDIUM** — `Drop` for `SearchIndex` skipped the zeroize
  scrub when the mutex was poisoned — exactly the panic-mid-
  search path the scrub exists to cover. Fix: switched to
  `self.inner.get_mut()` (no lock acquisition; `&mut self`
  already provides exclusive access), so the scrub runs
  regardless of poison state.

### Bounds

`MAX_INDEXED_FIELD_LEN = 64 KiB`, `MAX_INDEXED_DOCS = 1M`,
`MAX_FIELDS_PER_DOC = 64`, `MAX_DOC_ID_LEN = 256 B`,
`MAX_QUERY_LEN = 4 KiB`.

### Out of scope (future PRs)

- **Server-side searchable encryption** (SSE / OPE) — out of
  scope for v1; if needed later, layer on top.
- **Stemmer / tokenizer** — trigrams are language-agnostic
  by design; a localized tokenizer is a sibling crate.
- **Field-level highlighting** — the index returns ranked
  ids; the caller fetches and highlights.
- **Phrase queries / boolean operators** — ranked candidate-
  set search is the v1 surface. Phrase queries can land as
  an additional filter on the candidate set.
- **Forward-secure search** — at-rest sealing protects the
  index until the master key falls; the much stronger
  forward-secure SSE primitives are future work.
