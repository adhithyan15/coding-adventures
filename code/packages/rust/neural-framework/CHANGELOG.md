# Changelog

## Unreleased

- NN49 extends the separate required interchange runner to native C: thirty
  directed pairs, 78 actual children, six native lanes and zero bindings, with
  full baseline/return comparisons through 3002. Exact-head CI/merge pending.

- NN46 extends required interchange to native Ruby: twenty directed pairs,
  five actual native lanes, zero bindings, full baselines/returns through 3002.

- Extend the shared corpus to 55 cases (3 accept, 52 reject); pin and execute
  NN43's six null weight/bias cases in single, mixed and later-layer positions.

- Extend required native checkpoint exchange to Go: all twelve directed pairs,
  full model receipts at steps 3000/3001/3002, four native lanes and no bindings.

- Execute NN40's shared 49-case checkpoint acceptance/rejection corpus through
  the native JSON/model loader, including valid controls and save/reload checks.

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
