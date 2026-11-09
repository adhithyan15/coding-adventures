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
