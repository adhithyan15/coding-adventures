# Changelog

## Unreleased

- Extend the shared corpus to 55 cases (3 accept, 52 reject); pin and execute
  NN43's six null weight/bias cases in single, mixed and later-layer positions.

- Extend required native checkpoint exchange to Go: all twelve directed pairs,
  full model receipts at steps 3000/3001/3002, four native lanes and no bindings.

- Execute NN40's shared 49-case checkpoint acceptance/rejection corpus through
  the public native loader, including valid controls and save/reload checks.

- Extend native checkpoint exchange to Rust alongside TypeScript, checking all
  six directed pairs and full model/loss/prediction state at steps 3001/3002.

- Make native numerical validation and portable checkpoint exchange a selected,
  fail-closed dependency of the required repository CI gate.

## 0.1.0 — 2026-10-25

- Add configurable dense networks over the existing native matrix foundation.
- Add seeded LCG initialization, four activations, full-network MSE gradients,
  exact-count full-batch SGD, and non-mutating gradient inspection.
- Add closed V1 checkpoints compatible with the TypeScript implementation.
- Reject invalid shapes, metadata, and non-finite arithmetic atomically.
- Add shared paper/XOR fixtures, exhaustive numerical gradient checks, native
  training examples, resume tests, and a 95% coverage gate.
