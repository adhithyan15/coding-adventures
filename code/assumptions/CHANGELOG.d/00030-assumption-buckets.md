## 0.3.0

### Added
- `buckets-v1.json`: every catalogue id in exactly one VER31 bucket (owner,
  2026-11-23): (a) accepted platform baseline (35 ids: the `cc`, `mc`, `uarch`,
  `gate`, `phys` and `os` layers, Rust language semantics and the compiler that
  builds the verifiers), (b) statement assumptions (the 5 `spec.*` ids, closed by
  a signed sign-off of the Warrant statement), (c) the repository's own gaps
  (44 ids; any open one blocks `proved`). Each entry carries a rationale. Schema
  `code/specs/schemas/ver31-assumption-buckets-v1.schema.json`.

### Unchanged
- `catalogue-v1.json` is byte-identical; its digest, and every report's pin, is
  unchanged. Buckets live beside the catalogue so a bucket decision never forces
  a report regeneration.
