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
- An `input_receipt` binds a normalized repository path to exact SHA-256 and Git
  blob identities. The input bytes receive the same complete source IR as any
  fetched source.
- A `source_ir` partitions every source byte contiguously. Each represented claim
  records its exact byte range, UTF-8 quote, and quote hash; every discarded range
  has a non-empty reason.
- A `text_transform` proves how selected raw response bytes reproduce every byte
  of a rendered-text representation. Copy, HTML-entity decoding, a strict
  MathML-to-infix projection, and reasoned source-byte discards are the only
  accepted operations. Once a transform uses `discard`, its operations form a
  contiguous source partition and each zero-output discard carries a reason and
  the exact claim ID that owns it.
- A `provenance_bundle` binds stable clause IDs to snapshots, byte ranges, source
  IR, receipts, and recursively checked dependency or accepted-root decisions.
  Dependency decisions name the exact exported claim. Accepted roots classify
  the terminal fact, law, definition, or measurement and pin its raw source and
  successful receipt.
  Every clause must also exist in the bundle's designated ADJ input IR, so code
  cannot bypass decomposition while its external citations are checked.
- `manifest.json` pins bundle hashes. Only snapshots reachable from verified
  bundle clauses may be projected for `adj-verify`.

The capture command never opens a URL. A controlled spider writes response bytes
to a file and passes that file plus its receipt facts to the offline CAS tool:

```text
python code/scripts/adj_stdlib_provenance.py capture ...
python code/scripts/adj_stdlib_provenance.py capture-input ...
python code/scripts/adj_stdlib_provenance.py put-rendered ...
python code/scripts/adj_stdlib_provenance.py put-ir ...
python code/scripts/adj_stdlib_provenance.py put-transform ...
python code/scripts/adj_stdlib_provenance.py put-bundle ...
python code/scripts/adj_stdlib_provenance.py verify
python code/scripts/adj_stdlib_provenance.py project --output <directory>
```

## Reviewed roots

The arithmetic primitive, ratio, and percent-of roots and their worked-query
fixtures are rebuilt entirely offline from retained CAS source bodies:

```text
python code/scripts/build_adj_arithmetic_provenance.py
python code/scripts/build_adj_ratio_provenance.py
python code/scripts/build_adj_percent_of_provenance.py
```

Each generator checks retained source hashes and expected source spans before
rebuilding its provenance bundles, IR and transform objects, and CAS linkage.
The ratio generator's reviewed one-time bootstrap accepts captured bytes without
opening the locator; after those exact bytes enter the CAS, ordinary reruns need
no external file:

```text
python code/scripts/build_adj_ratio_provenance.py --captured-source <ratio.html>
python code/scripts/build_adj_ratio_provenance.py
```

The percent-of bootstrap follows the same offline contract. Six controlled
captures of the OpenStax rational-numbers page produced the identical
666,580-byte body with SHA-256
`89ebca7f93281cae7d8791cb6dfc65ff4ff289268fc5d7f03d41bb10adeb4e5e`.
Its formal MathML rule is projected deterministically to
`n% of x items is (n/100)*x.`; both the raw MathML and every rendered byte remain
linked in the CAS.

The reviewed ratio capture was repeated six times with `Accept: text/html`,
identity transfer encoding, no request cookies, and the same user agent. All six
52,191-byte bodies rehashed to
`8eced6f9859e60557b69ec9ef2c1cbaf31c7086cc0d9212edc7b39b52ef52baf`.
The receipt retains only the allow-listed content type; the response's session
cookie is deliberately excluded because it is neither source content nor
provenance evidence.

Per-root generators register only the bundle IDs they own inside a CAS-root
transaction protected by the tracked `cas/lock` file and an OS-released lock.
Because the stable lock identity is inside the CAS, it cannot diverge with
process-specific temporary-directory settings. Authoritative Python and Rust
readers and every CLI writer participate in the same lock, so neither split
publication nor a lost index update is externally visible. Registration
validates both the old and proposed graphs, is additive and idempotent, and
rolls back newly written objects on failure. A different hash for an already
registered bundle ID fails closed until an explicit root-replacement migration
can prune the old unreachable graph.

This separation prevents an untrusted source locator from turning the verifier
into a network or SSRF primitive. Reads are bounded and reject links and Windows
reparse points; object writes are exclusive; index writes are atomic. Existing
hashes are reused, while missing or changed bytes, claims, transforms, graph
edges, receipts, partitions, and projections fail closed.
