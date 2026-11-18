---
category: Rust
---

# An include_str path is relative to the source file and fixture ancestry must be counted from there

An integration test under `tests/` includes repository fixtures from the source
file's directory, not from the package root or Cargo's working directory. The
first Hamurabi corpus build used three parent steps from
`code/packages/rust/hamurabi/tests/shared_corpus.rs`, which stopped at
`code/packages/rust` and failed to find `specs/fixtures/hamurabi-v1/cases.json`.
Count the path from the test file: four parent steps reach `code/`, then the
fixture path. Compile the exact integration target after adding or changing an
`include_str!` path so the compiler verifies the resolved location.
