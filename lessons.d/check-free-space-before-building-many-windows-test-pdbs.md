---
category: Rust
---

# Check free space before building many Windows test PDBs

A focused Cargo test without `--lib` still linked every integration-test binary.
With less than 1 GiB free, PDB generation exhausted the disk and reported
LNK1318 LIMIT. Use `--lib` for a library-only check, and disable debug symbols and
incremental artifacts for disposable validation when space is constrained.
Clean only the current isolated checkout's regenerable target with `cargo clean`.
Run the required integration tests separately; this is not permission to omit
their acceptance coverage or delete another checkout's artifacts.
