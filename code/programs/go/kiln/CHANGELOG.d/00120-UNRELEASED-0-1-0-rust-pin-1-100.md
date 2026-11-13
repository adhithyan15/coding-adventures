# Rust pin follows the 1.100.0 toolchain bump

- `code/kiln/legacy/toolchains.json`: `rustc` and `cargo` are pinned to
  `1.100.0` (was `1.99.1`) in all three entries that name them, matching
  `mise.toml` after #19269. `TestCommittedPolicyFiles` (the drift test) was
  failing on `main` because the bump did not touch this file and Kiln's tests
  were not selected for that change; the build tool now selects Kiln whenever
  a pin source changes.
- **Windows CI fixes in `internal/legacy`** (they surfaced the first time
  Windows CI built Kiln). `ConstructPATH` and `validHome` now use POSIX rules
  on every host: `path.IsAbs` and `path.Clean`, the `:` separator, and both
  `:` and `;` refused in `HOME`. The PATH they build is for the Linux
  recording environment, and on Windows `filepath.IsAbs("/usr/bin")` is false
  and the separator is `;`, which keyed a different value. The probe splits
  that PATH on `:` to match. The probe-timeout test uses an injectable clock,
  because Windows' coarse timer can report no time passing.
