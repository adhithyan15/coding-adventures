# Reimplementing a third-party numeric crate: dump the oracle, never estimate; disambiguate the `-p` spec

Context: Phase B of the Engram zero-dep program replaced the third-party `fsrs`
crate (FSRS-6 spaced-repetition scheduler; pulls in the `burn` tensor framework)
with a from-scratch zero-dep `code/packages/rust/fsrs`.

**Lessons:**
- **Never hand-estimate frozen numeric snapshots.** The scheduler math is exact
  scalar `f32`, but my mental estimates for the expected `next_states` /
  `memory_state_from_sm2` outputs were off by large margins (e.g. guessed
  difficulty 5.68, real 6.91; guessed stability 18.06, real 15.45). The reliable
  path: run the REAL crate (engram-core already depended on it) via a throwaway
  `#[test] … println!` to dump ground-truth for the exact snapshot inputs, then
  paste those values. Estimating would have shipped a wrong "oracle".
- **Cross-verify against the LIVE crate before deleting it.** Add the upstream
  crate as an aliased `[dev-dependencies]` (`upstream_fsrs = { package = "fsrs",
  version = "…" }`), assert your impl matches across a randomized grid (5,900+
  comparisons here), THEN remove the throwaway test + dev-dep. Same interop-gate
  discipline as the protobuf-vs-prost byte check.
- **Naming a repo crate the same as a crates.io crate makes `cargo -p` ambiguous
  while both are in the graph** (during the dev-dep cross-check). Error: "multiple
  `fsrs` packages … specification `fsrs` is ambiguous." Disambiguate with the
  version: `cargo test -p fsrs@0.1.0 …`. After the upstream dep is dropped the
  bare `-p fsrs` is unambiguous again. Naming the repo crate identically is still
  worth it: the consumer swap becomes a one-line path change with zero `use`
  edits.
- **Transcribe the exact operation order**, not just the formulas — same order of
  `+`/`*`/`.exp()`/`.powf()` gives bit-for-bit `f32` agreement. Clippy will flag
  snapshot literals with `excessive_precision`; `cargo clippy --fix` trims them
  to the f32-canonical form (still within any sane test tolerance).
- Upstream `fsrs` needs `burn` only for TRAINING; the scheduling/inference path
  we consume is pure scalar arithmetic. Reimplementing only the forward half
  dropped the entire `burn` subtree. Check whether a heavy dep's weight is in a
  code path you actually use before assuming it's unavoidable.

Discovered: 2026-07-03, Engram zero-dep Phase B (drop `fsrs`).
