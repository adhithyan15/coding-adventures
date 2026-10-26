# Changelog

## 0.1.0 — 2026-10-26

- Add the reviewed Spec 07bd 64-bit little-endian scalar fixed-point profile.
- Add complete checked state, typed atomic load/step/run/execute lifecycle, and
  bounded complete traces.
- Add table-driven Power ISA classification and the source-locked Microwatt
  integer, branch, condition, special-register, memory, synchronization, and
  termination semantics.
- Pin sticky SO to 64-bit OV only, Microwatt word multiply/divide shaping and
  full-width Rc, and valid-but-unsupported `ptesync`.
- Correct word arithmetic shift sign extension, byte-LSB parity, ignored
  slash-reserved instruction fields, typed `mfocrf`/`mtocrf` exclusion, exact
  `sradi` selectors, `rlwimi` upper
  half preservation, and Microwatt word-multiply result/overflow behavior.
- Add an executed 173-row complete-core GHDL differential corpus with pinned
  tool/source/harness hashes plus independent manifest and functional
  consumers; the documented differential boundary excludes CIA because the
  pinned harness does not export it.
