# DEC PDP-10 KA10 functional simulator

This crate implements the bounded 36-bit KA10 integer/control architecture
defined by Spec 07aq. It provides strict five-byte word transport, 18-bit
indexed and indirect effective addresses, sixteen accumulators, complete owned
state, typed atomic faults, and deterministic reset/load/restore/step/run/
execute lifecycle APIs.

The completion suite exercises 20 unit/integration tests and records 91.53%
source line coverage (713 of 779 lines).

The implemented instruction boundary includes data movement, signed fixed-point
arithmetic, core Boolean families, signed compare/skip/jump families, AOBJ,
JRST/HALT, and stack control. Floating point, byte pointers, block transfer,
execute, APR, and device I/O remain explicit future cells.

```bash
cargo test -p pdp10-simulator --release --all-targets
cargo clippy -p pdp10-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p pdp10-simulator --no-deps
```
