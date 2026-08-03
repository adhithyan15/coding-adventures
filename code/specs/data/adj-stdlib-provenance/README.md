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
- A `formula_derivation` binds one parser-identified export to its exact import
  closure, formula-call sequence, question IR claim, and purpose-built computation
  plan. An `execution_witness` binds that derivation to byte-pinned input facts, an
  exact rational and IEEE-754 result, the nested computation tree, and successful
  formula, input, and recomputation checks. Verification recreates programs from
  CAS bytes and requires the replayed canonical objects to hash identically. Input
  v2 references resolve uniquely across the complete dependency closure and pin their
  exact ADJ source/IR and cited snapshot/IR. Dependency inputs name their owning
  bundle hash; query inputs use the parent bundle relationship because embedding
  that bundle's own hash in its witness would create a content-addressing cycle.
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
python code/scripts/migrate_adj_formula_inventories.py --formula-inventory-binary <adj-formula-inventory-binary> --formula-audit-binary <adj-formula-audit-binary>
python code/scripts/build_adj_arithmetic_provenance.py --formula-inventory-binary <adj-formula-inventory-binary> --formula-audit-binary <adj-formula-audit-binary>
python code/scripts/build_adj_ratio_provenance.py --arithmetic-bundle-sha256 <verified-current-root-sha256> --formula-inventory-binary <adj-formula-inventory-binary> --formula-audit-binary <adj-formula-audit-binary>
python code/scripts/build_adj_percent_of_provenance.py --arithmetic-bundle-sha256 <verified-current-root-sha256> --formula-inventory-binary <adj-formula-inventory-binary> --formula-audit-binary <adj-formula-audit-binary>
```

Each generator checks retained source hashes and expected source spans before
rebuilding its provenance bundles, IR and transform objects, and CAS linkage.
Dependent generators require the verified current primitive-arithmetic root hash
explicitly; they reject malformed hashes and roots with the wrong bundle ID. The
migration command replays the trusted parser against each formula-bearing input
and the formula audit against each query reconstructed from CAS bytes, then
compare-and-swaps all six formula and query roots as one dependency closure. The
current witnesses cover four primitive operations, one ratio, and one percent-of
example; they establish replayable provenance for those executions, not universal
correctness for every possible input.
The ratio generator's reviewed one-time bootstrap accepts captured bytes without
opening the locator; after those exact bytes enter the CAS, ordinary reruns need
no external file:

```text
python code/scripts/build_adj_ratio_provenance.py --arithmetic-bundle-sha256 <verified-current-root-sha256> --formula-inventory-binary <adj-formula-inventory-binary> --formula-audit-binary <adj-formula-audit-binary> --captured-source <ratio.html>
python code/scripts/build_adj_ratio_provenance.py --arithmetic-bundle-sha256 <verified-current-root-sha256> --formula-inventory-binary <adj-formula-inventory-binary> --formula-audit-binary <adj-formula-audit-binary>
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
reparse points; object writes are exclusive; index writes are atomic. Trusted parser
and audit commands use a hard platform containment boundary. Windows assigns a
suspended root to a kill-on-close Job before execution. Linux requires the absolute
`ADJ_PROVENANCE_CGROUP_ROOT` path to an operator-delegated cgroup-v2 subtree; an
external guardian assigns the command to a fresh child before release, survives
verifier death through a private control-pipe EOF, and accepts cleanup only after
`cgroup.kill`, exact `populated 0`, reaping, and child removal. macOS and generic POSIX
strict execution reject before launch because process groups cannot contain a
descendant that creates another session.
Windows Job lifecycle calls are fault-injected in platform-neutral tests and exercised
against real suspended processes in the Windows PR matrix. Native enumeration errors,
truncated thread records, incomplete resumes, termination failures, and handle-close
failures all prevent a verifier result from being accepted.
Lifecycle failures remain human-readable but also expose an immutable recursive record
through `ProvenanceError.lifecycle`: stage, native API, numeric error code, symbolic
status code, message, and ordered cleanup causes. Its `to_dict()` projection is
canonical JSON-ready, so callers
do not need to parse localized operating-system messages to diagnose containment. The
projection is versioned as `adj-stdlib/process-lifecycle-failure/v2`; CLI failures add
it as `lifecycle_failure` while retaining the existing `error` and `valid` fields.
Process-wait, pipe-read, and pipe-close faults are records too; partial output is never
accepted merely because its prefix happens to be valid canonical JSON.
POSIX group termination is fault-injected independently of the host OS: lookup misses,
permission errors, poll failures, root-process fallback, and simultaneous pipe-close
failures retain API-attempt ordering. Repeated termination attempts are never collapsed,
concurrent reader failures use event order, and recovery ends with a bounded root wait.
Linux runs the complete suite; macOS runs the focused unsupported-boundary, guardian-
protocol, process-tree, drain, and raw-close gate before merge.
The Linux gate enters its delegated root before running the provenance suite, then uses
real fixtures to prove cleanup of a `setsid()` descendant and cleanup after verifier
`SIGKILL`. Guardian status travels on a separate bounded canonical pipe, so helper
bytes cannot forge containment success. Stuck pipe readers use independently bounded
raw-owner close attempts; healthy channels close independently, and every unconfirmed
close fails closed. The guardian threat model covers accidental session escape and
abrupt verifier death while the guardian and kernel remain alive. A same-UID helper
that deliberately attacks the guardian or delegated cgroup requires a stronger
privilege boundary.
Existing hashes are reused, while missing or changed bytes, claims, transforms, graph
edges, receipts, partitions, and projections fail closed.
