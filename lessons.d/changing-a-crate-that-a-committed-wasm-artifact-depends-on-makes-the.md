---
category: CI & GitHub Actions
---

# Changing a crate that a committed wasm artifact depends on makes the artifact stale

The SEC-001 PR changed `lzss`; `spreadsheet-wasm` depends on it transitively, so
its committed `pkg/spreadsheet_engine.wasm` (and the `.inputs` stamp, and the
visicalc-html vendor bundle derived from it) went stale and
`bash build-wasm.sh --check` failed in CI.

Fix: re-run `bash build-wasm.sh` (needs `rustup target add wasm32-unknown-unknown`),
then `bash scripts/bundle-wasm-engine.sh` in visicalc-html, and commit all three.

Next time: before pushing a change to any shared crate, run
`cargo tree -i <crate> -e normal,build --prefix none` and look for consumers that
ship committed generated artifacts (`pkg/*.wasm`, vendored bundles); regenerate
them after the final source edit, since any later source change re-stales them.

The digest also depends on crates.io versions, not just this repo's sources.
`code/packages/rust/Cargo.lock` is gitignored, so CI resolves dependencies
fresh on every run. Any upstream release (for example `syn` 3.0.9 -> 3.0.10)
therefore changes the inputs digest and makes the committed `.wasm` "stale" in
CI even though no file in the repo changed. Always regenerate the artifact
after a fresh `cargo generate-lockfile` (delete the local lock, then regenerate),
so the stamp matches what CI will compute. The permanent fix is to stop
committing the wasm at all; that is tracked by backlog item BUILD-001.
