# DEC PDP-11 functional simulator

This Rust crate implements the complete behavioral surface defined by Spec
07o: eight 16-bit registers, a 64 KiB little-endian memory, the NZVC processor
status word, all eight orthogonal addressing modes, word/byte ALU operations,
branches, subroutines, RTI, SOB, NOP, and HALT.

The API provides reset/load/step/run/execute, immutable owned snapshots,
checked boundary mutators, typed errors, mandatory execution bounds, and small
instruction-encoding helpers. Tests pin the mature Python oracle, whose audit
baseline is 163 passing tests and 98.63% line coverage.

```bash
cargo test -p pdp11-simulator
cargo clippy -p pdp11-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p pdp11-simulator --no-deps
```
