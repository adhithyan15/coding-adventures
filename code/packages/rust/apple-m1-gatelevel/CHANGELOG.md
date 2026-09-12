# Changelog

## 0.1.0 (2026-09-08)

- Add the exact 530,565-DFF gate-level partner for Spec 07z.
- Reuse the independent complete AArch64 gate engine and add independent strict
  Apple scalar-FP, vector-memory, DUP, and NEON decode.
- Add repository-DFF vector storage and gate-network NEON integer lane add,
  subtract, multiply, selection, masking, and broadcast.
- Share structured encoders and the functional simulator's typed atomic state,
  lifecycle, traces, results, and faults.
- Pass eleven Rust tests, all 360 Python full-state vectors in functional
  trace/state lockstep, strict checks, both historical consumers, and 90.52%
  package line coverage (621/686).
