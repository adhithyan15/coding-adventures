# Changelog

## Unreleased

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
