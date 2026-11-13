## Unreleased — Kiln re-runs when a toolchain pin source changes

- **New repository-input consumer `go/programs/kiln`.** Kiln's committed
  `code/kiln/legacy/toolchains.json` is checked against `mise.toml`,
  `code/specs/data/toolchain-pins.json` and `.github/workflows/ci.yml` by a
  drift test. Those files lived outside Kiln's package, so a toolchain bump
  that edited only them (#19269, Rust 1.99.1 -> 1.100.0) did not select Kiln,
  and the drift went unnoticed until a later Kiln change failed. The three pin
  sources are now Kiln's declared repository inputs, so the selector rebuilds
  Kiln and the Kiln graph export includes them.
