# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-08-27

### Added

- Complete 37-opcode Type B and five-prefix Type A IBM 704 v1 instruction set.
- Exact D-flip-flop storage for configurable memory, AC, MQ, index registers,
  PC, halt state, and architectural triggers.
- One-hot decoders, ripple-carry sign-magnitude and effective-address paths,
  AND/shift/add multiplication, and restoring integer division.
- Gate-based floating add, subtract, multiply, divide, normalization, and
  round-to-nearest-even networks with 53-bit oracle-compatible intermediates.
- Strict canonical transport, typed fail-closed lifecycle errors, bounded
  execution, owned snapshots, and topology metrics.
- Whole-state, per-clock differential tests against `ibm704-simulator`, plus
  architecture programs and seeded floating conformance vectors.
