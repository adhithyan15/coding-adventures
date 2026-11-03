---
category: Testing & coverage
---

# Run Rust validation from its workspace before interpreting a red test

During P006a's test-first work, `cargo test -p ...` was initially launched
from the repository root. It failed because that directory has no Cargo.toml,
before collecting or compiling the new artifact tests. That was a command
location error, not evidence the intended missing-artifact tests were red.

Run Rust package checks from code/packages/rust, or pass that workspace's
manifest explicitly. Re-run the red check in that context and inspect its
actual diagnostic before claiming falsification; then validate the completed
source normally. Do not interpret any nonzero shell exit as the desired test.
