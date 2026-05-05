# `coding_adventures_vault_search` — VLT13 encrypted search

Local trigram index + BM25 ranker for the Vault stack. The
index lives in memory at runtime and is persisted as ordinary
vault records — encrypted under VLT01 sealed-store. Server-side
search is **not** in v1; if needed later, layer SSE on top.

## Quick example

```rust
use std::collections::BTreeMap;
use coding_adventures_vault_search::{
    DocumentId, SearchableFields, SearchIndex,
};

let idx = SearchIndex::new();

// Declare which fields are searchable AND with what weight.
// Passwords / TOTP seeds / etc. are NOT in here — they never
// reach the index.
let s = SearchableFields::new()
    .with("title", 2.0)
    .with("url", 1.0);

let mut fields = BTreeMap::new();
fields.insert("title".into(), "GitHub".into());
fields.insert("url".into(), "https://github.com".into());
fields.insert("password".into(), "hunter2".into()); // skipped

idx.index(DocumentId::new("login/github")?, &fields, &s)?;

// Search — partial substring matches just work because we use
// trigrams.
for hit in idx.search("git", 10)? {
    println!("{}: {:.3}", hit.id.as_str(), hit.score);
}
```

## How it works

- **Trigrams**: every 3-byte ASCII window of every indexed
  field (lowercased) becomes a posting. "Foo" gets `foo`;
  "abcabc" gets `abc, bca, cab, abc`. Substring search is
  natural ("git" matches "github" and "digital").
- **BM25**: standard `k1=1.2, b=0.75` over the trigram
  candidate set. `tf` is the per-document trigram count
  (weighted by the field's declared weight); `idf` uses the
  global posting-list size.
- **Persistence**: `to_bytes()` returns an opaque framed
  buffer (`"VSI1" + version + length-prefixed entries`).
  Wrap it in `vault_sealed_store::Sealed` and you've got an
  encrypted, sync-able index record.

## Per-schema searchability

`SearchableFields` is an explicit allow-list. A field that the
caller never declares is never indexed. This is the
load-bearing security property:

> Stolen index file (decrypted by an attacker who later
> compromises the master key) gives up titles and URLs but
> NOT the passwords behind them — because passwords were
> never in the index.

Test `search_does_not_index_undeclared_fields` exercises this
directly.

## Bounds

| Field                    | Cap                |
|--------------------------|--------------------|
| `MAX_INDEXED_FIELD_LEN`  | 64 KiB             |
| `MAX_INDEXED_DOCS`       | 1,000,000          |
| `MAX_FIELDS_PER_DOC`     | 64                 |
| `MAX_DOC_ID_LEN`         | 256 bytes          |
| `MAX_QUERY_LEN`          | 4 KiB              |

## Threat model

- **At-rest confidentiality**: the index is sealed by VLT01
  before persistence.
- **In-memory hygiene**: `clear()` drops every per-document
  trigram map (each scrubbed via `Zeroize`). `Drop` runs the
  same path so even an exotic exit path doesn't leave
  plaintext trigrams in the heap.
- **Bounded memory**: per-field, per-doc, per-query, per-id
  caps fire at the validation layer with no partial state on
  the index.
- **Strict deserializer**: `from_bytes` rejects bad magic,
  unsupported version, oversize doc / tf counts, truncated
  input, trailing bytes, non-finite / negative `tf` weights,
  invalid UTF-8 in `DocumentId`, and `DocumentId` strings that
  fail validation (control chars / bidi / zero-width).
- **Caller-controlled stop list**: `SearchableFields` weight
  ≤ 0, NaN, or infinite is silently dropped — a malicious
  caller cannot cheat the ranker via an unbounded weight.
- **Query bounds**: `MAX_QUERY_LEN` rejects oversized queries
  before they hit the trigram extractor.

## What this crate is NOT

- Not server-side searchable encryption (SSE / OPE).
  The server sees ciphertext only by VLT10; this layer
  doesn't try to be "searchable on the server".
- Not a stemmer / language pipeline. Trigrams are language-
  agnostic.
- Not the wire transport — VLT11 ships parsed `vault search`
  commands; this crate is the local engine they hit.
- Not a secrets-aware classifier. The caller decides which
  fields are safe to index.

## Capabilities

None — pure data structures. See `required_capabilities.json`.

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT13-vault-encrypted-search.md`](../../../specs/VLT13-vault-encrypted-search.md).
