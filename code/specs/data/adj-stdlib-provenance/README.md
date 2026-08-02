# ADJ standard-library provenance CAS

This directory is the byte-level evidence store for standard-library clauses.
It starts empty and grows only through reviewed source migrations.

## Contract

- `cas/objects/aa/bb...` stores exact bytes under their SHA-256 fanout path.
- `cas/index.json` is a rebuildable cache of object kinds, byte sizes, links, and
  canonical paths. Content-addressed bundles are the provenance authority.
- A `fetch_receipt` is a content-addressed object, not mutable metadata. It binds
  an HTTPS locator, retrieval time, status, media type, safe response headers,
  and exact response-body hash.
- A `source_ir` partitions every source byte contiguously. Each represented claim
  records its exact byte range, UTF-8 quote, and quote hash; every discarded range
  has a non-empty reason.
- A `text_transform` proves how selected raw response bytes reproduce every byte
  of a rendered-text representation. Copy and HTML-entity decoding are the only
  accepted operations.
- A `provenance_bundle` binds stable clause IDs to snapshots, byte ranges, source
  IR, receipts, and recursively checked dependency or accepted-root decisions.
  Dependency decisions name the exact exported claim. Accepted roots classify
  the terminal fact, law, definition, or measurement and pin its raw source and
  successful receipt.
- `manifest.json` pins bundle hashes. Only snapshots reachable from verified
  bundle clauses may be projected for `adj-verify`.

The capture command never opens a URL. A controlled spider writes response bytes
to a file and passes that file plus its receipt facts to the offline CAS tool:

```text
python code/scripts/adj_stdlib_provenance.py capture ...
python code/scripts/adj_stdlib_provenance.py put-rendered ...
python code/scripts/adj_stdlib_provenance.py put-ir ...
python code/scripts/adj_stdlib_provenance.py put-transform ...
python code/scripts/adj_stdlib_provenance.py put-bundle ...
python code/scripts/adj_stdlib_provenance.py verify
python code/scripts/adj_stdlib_provenance.py project --output <directory>
```

This separation prevents an untrusted source locator from turning the verifier
into a network or SSRF primitive. Reads are bounded and reject links and Windows
reparse points; object writes are exclusive; index writes are atomic. Existing
hashes are reused, while missing or changed bytes, claims, transforms, graph
edges, receipts, partitions, and projections fail closed.
