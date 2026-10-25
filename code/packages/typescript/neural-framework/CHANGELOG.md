# Changelog

## Unreleased

- Execute NN40's shared 49-case checkpoint acceptance/rejection corpus through
  the public native loader, including valid controls and save/reload checks.

- Extend native checkpoint exchange to Rust alongside Python, checking all
  six directed pairs and full model/loss/prediction state at steps 3001/3002.

- Reuse native numerical validation and checkpoint exchange inside the required
  repository CI gate, rejecting selected jobs without successful evidence.

## 0.1.0 — 2026-10-25

- Add native configurable dense networks above the existing matrix package.
- Add deterministic initialization, linear/sigmoid/tanh/ReLU activations,
  full-network gradients, and exact-count SGD training.
- Add copied closed checkpoints shared with Python and atomic update validation.
- Add paper-oracle, finite-difference, seeded XOR, malformed-input, ownership,
  and checkpoint resume tests with enforced coverage thresholds.
- Add an executable JSON example and cross-language checkpoint exchange gate.
