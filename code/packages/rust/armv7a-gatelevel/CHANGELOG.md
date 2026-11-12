# Changelog

## Unreleased

- Rewrite the seeded-register loop in the differential test(s) to satisfy Rust 1.100 clippy (`needless_range_loop`) with an iterator;
  behaviour is unchanged.

## 0.1.0

- Add the exact 524,833-DFF gate-level partner for Spec 07x.
- Add an independent mixed-width Thumb-2 gate execution path and the complete
  typed transactional lifecycle of the functional oracle.
- Add all 417 Python/functional full-state and trace differentials, manual Spec
  07x/lifecycle suites, strict checks, and 97.41% package line coverage.
