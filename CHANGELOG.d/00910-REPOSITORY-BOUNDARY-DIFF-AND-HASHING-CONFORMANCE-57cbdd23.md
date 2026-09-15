### Repository-boundary diff and hashing conformance

- Defined process-free language-neutral behavior for exact reverse diff
  selection from the digest-pinned repository source-input boundary and for
  package hashing over the caller-supplied local-plus-boundary input union.
- Added two closed fixtures and semantic runner coverage for exact consumers,
  dependent and prerequisite closure, near-path rejection, digest mismatch,
  raw-UTF8 ordering, and deterministic cache digests without granting host
  authority.

