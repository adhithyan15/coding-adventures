## [Unreleased]

### Security

- Swift runtime traps are uncatchable, and `decoder`/`demux` could trap
  (`1 << n` with n >= 62, `Array(repeating:count:)` with a huge or negative
  count, `outputs[idx]` on an empty array, index overflow at bit 63). They
  now reject oversized widths with a thrown error before any shift or
  allocation.
- New public limits `maxSelectBits = 16`, `maxWidth = 1 << 16`,
  `maxCounterWidth = 64` (parity with the Go/Haskell/Python/Rust lanes).
- New `LogicGateError.invalidWidth(name:value:)` and `.invalidShape(String)`.
- `decoder`, `demux`, `muxN`: more than 16 select bits rejected.
- `muxN`: input count must be a power of two and equal `1 << sel.count`
  (excess select bits are no longer silently ignored).
- `encoder`, `priorityEncoder`: length must be a power of two <= `maxWidth`.
- `register`: `q`/`masterQ` length mismatches are rejected (previously padded
  with 0) and their bits are validated; width capped at `maxWidth`.
- `counter`: width 0 (previously reported overflow=1) and width > 64 rejected.

Behaviour change for callers passing non-power-of-two or mismatched shapes.
