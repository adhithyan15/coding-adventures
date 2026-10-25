# Changelog

## Unreleased

- Add native Rust to required NN39 three-language checkpoint exchange, including
  both directions with Python and TypeScript and exact resumed-step receipts.

- Correct the Linux coverage include filter to match Cargo-workspace-relative
  library paths; preserve the 95% threshold and integration-test execution.

## 0.1.0 — 2026-10-25

- Add native configurable dense networks above Rust matrix arithmetic.
- Add deterministic u32 initialization, four activations, full-network MSE
  gradients, exact-count SGD, owned traces, and atomic candidate validation.
- Add closed NN37 checkpoint data and bounded JSON loading with safe step
  counts, duplicate/unknown-field rejection, and non-finite/shape checks.
- Add shared paper/XOR fixtures, every-parameter finite-difference tests,
  malformed-input and overflow boundaries, and native checkpoint resume.
- Add JSON XOR/stdin example, executable native-only evidence validator,
  workspace membership, cross-platform BUILD fronts and 95% Linux coverage gate.
- Keep cross-language conformance and optional C ABI explicitly separate.
