# read-write-separation

`read-write-separation` is the pure Rust classifier for the Chief of Staff
RWS rule:

> no agent may both ingest untrusted input and produce external actuation.

The crate keeps the first implementation intentionally small:

- classify manifest capabilities by `flavor` and `trust`;
- apply conservative defaults from `code/specs/read-write-separation.md`;
- reject manifests that mix untrusted input with actuation; and
- reject v1 same-resource read/write overlaps for files, vault secrets, and
  channels.

Capability-cage, supervisor, and orchestrator crates can call this crate before
launching agents or wiring pipelines. It performs no I/O.
