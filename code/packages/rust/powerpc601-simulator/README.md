# PowerPC 601 simulator

Complete Rust functional simulator for the integer subset of the 1992
PowerPC 601 defined by Layer 07u.

## Machine

- 32 unsigned 32-bit GPRs plus LR, CTR, XER, CR, and CIA
- exact 64 KiB big-endian byte-addressed memory
- fixed-width 32-bit big-endian instructions
- all-zero instruction word as the halt sentinel

The instruction surface covers immediate and register arithmetic, carry,
signed and unsigned division, logic, compare/CR fields, shifts, aligned
byte/halfword/word memory transfers, relative/absolute/link/conditional
branches, LR/CTR branches, and special-register transfers.

## Checked lifecycle

```rust
use powerpc601_simulator::encoding::{assemble, d_form, halt};
use powerpc601_simulator::PowerPc601Simulator;

let program = assemble(&[d_form(14, 1, 0, 42), halt()]);
let mut cpu = PowerPc601Simulator::new();
let result = cpu.run_checked(&program, 8)?;
assert!(result.halted);
assert_eq!(result.final_state.gpr[1], 42);
# Ok::<(), powerpc601_simulator::PowerPcError>(())
```

`PowerPcState` owns every architectural register and memory byte, halt, and
the exact installed-program range. `restore` validates before mutation.
Checked loading, direct access, stepping, and bounded runs use typed errors.
Instruction faults and failed runs roll the complete machine back, and every
trace owns the raw word plus complete before/after states.

## Verification

The suite includes unit, lifecycle, workload, and reproducible Python-oracle
tests. The 239-vector differential exercises every implemented decode family
over four seeded full states and hashes CIA, all registers, all 64 KiB, and
halt. Defined nonzero-divisor behavior stays differential; zero divisors are
explicit typed atomic Rust faults instead of the Python oracle's synthetic
zero result.

```sh
cargo test --package powerpc601-simulator
cargo clippy --package powerpc601-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --package powerpc601-simulator --no-deps
```
