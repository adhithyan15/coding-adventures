# Data General Nova functional simulator

This crate implements the checked common Nova / Nova 1200 integer and control
core from Spec 07ar: four 16-bit accumulators, 15-bit addressing, complete
32-Kword state, all memory-reference forms, the microprogrammed operate space,
pre-update auto-index/decrement indirection, the CPU HALT instruction, strict
transport, typed atomic faults, and deterministic lifecycle APIs.

Later Nova, Eclipse, optional arithmetic, peripheral, interrupt, DMA, and timing
features remain explicit follow-up cells.

The completion suite exercises 17 unit/integration tests and records 91.53%
source line coverage (540 of 590 lines).

```bash
cargo test -p data-general-nova-simulator --release --all-targets
cargo clippy -p data-general-nova-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p data-general-nova-simulator --no-deps
```
