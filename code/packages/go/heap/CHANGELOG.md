# Changelog

## 0.1.0

- Add a bounded checked priority queue and bottom-up construction from a slice.
- Preserve the existing `MinHeap` API while clearing removed references and
  guarding child-index arithmetic.
- Add independent model histories, boundary tests, seeded runs, and fuzzing.
