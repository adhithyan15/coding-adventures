# Rust pin follows the 1.100.0 toolchain bump

- `code/kiln/legacy/toolchains.json`: `rustc` and `cargo` are pinned to
  `1.100.0` (was `1.99.1`) in all three entries that name them, matching
  `mise.toml` after #19269. `TestCommittedPolicyFiles` (the drift test) was
  failing on `main` because the bump did not touch this file and Kiln's tests
  were not selected for that change; the build tool now selects Kiln whenever
  a pin source changes.
