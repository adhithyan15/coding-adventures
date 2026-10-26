# Cortex-M23 gate-level simulator (Rust)

This package is the DFF-backed architectural gate companion to
`cortex-m23-simulator`. It implements the checked Armv8-M Baseline Thumb boundary from
[Spec 07ao2](../../../specs/07ao2-cortex-m23-gatelevel.md) with an independent
gate transition engine. Production code does not depend on the functional
simulator; that package is a differential-test oracle only.

All 524,898 persistent bits — 64 KiB memory, integer and special registers,
mode, halt/sleep/event state, and installed-image metadata — are clocked
through repository D flip-flops. Gate-built decoders, ripple arithmetic,
Boolean logic, staged shifters, branch predicates, address generation, and
memory write enables derive committed transitions.

This is an educational ISA-level topology, not a reconstruction of proprietary
Arm or licensee RTL, a silicon netlist, physical placement, timing, or power.
The bounded processor profile fixes the Armv8-M Security Extension to
`Disabled`, execution to `NonSecure`, SAU attribution to all-Non-secure, and
the excluded Secure banks and SAU descriptors to their canonical constants.

```rust
use cortex_m23_gatelevel::{encoding, CortexM23GateLevel};

let mut cpu = CortexM23GateLevel::new();
cpu.load(&encoding::program(&[encoding::movs(0, 7), encoding::bkpt(0)]), 0x100)?;
let result = cpu.run(2)?;
assert_eq!(result.final_state.registers[0], 7);
# Ok::<(), cortex_m23_gatelevel::CortexM23Error>(())
```

The package retains complete before/after state traces under a fixed 8 MiB
budget, uses typed atomic faults, and validates complete state on restore.
Thirty-four release tests cover exhaustive 16-bit decode plus representative
32-bit second-half decode, every implemented execution family, all condition
and shift-boundary outcomes, arithmetic and multiply edges, memory-addressing
overlaps, complete-state differentials, allocation rollback, concrete DFF
topology, exact critical-body locks, and hostile structural-confuser
rejection. The completion baseline is 93.64% combined Rust line coverage
(1,825/1,949).

```sh
cargo test -p cortex-m23-gatelevel --release --no-fail-fast
cargo clippy -p cortex-m23-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cortex-m23-gatelevel --no-deps
cargo llvm-cov -p cortex-m23-gatelevel --all-targets --release --summary-only
```
