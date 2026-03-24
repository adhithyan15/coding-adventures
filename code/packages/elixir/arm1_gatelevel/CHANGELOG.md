# Changelog

## 0.1.0 — 2026-03-23

### Added
- Initial gate-level ARM1 simulator ported from Go
- Gate-level ALU: all 16 operations routed through logic gate functions
- Gate-level barrel shifter: 5-level multiplexer tree
- Gate-level condition evaluation using gate primitives
- Bit conversion helpers (int_to_bits/bits_to_int)
- Cross-validation tests against behavioral simulator
- Full CPU execution: data processing, load/store, block transfer, branch, SWI
