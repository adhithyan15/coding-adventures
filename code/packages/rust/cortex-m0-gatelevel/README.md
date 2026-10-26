# Cortex-M0 gate-level simulator (Rust)

This package is the DFF-backed architectural gate companion to
`cortex-m0-simulator`. It implements the checked Armv6-M Thumb boundary from
[Spec 07ak2](../../../specs/07ak2-cortex-m0-gatelevel.md) with an independent
gate transition engine. Production code does not depend on the functional
simulator; that package is a differential-test oracle only.

All 524,898 persistent bits — 64 KiB memory, integer and special registers,
mode, halt/sleep/event state, and installed-image metadata — are clocked
through repository D flip-flops. Gate-built decoders, ripple arithmetic,
Boolean logic, staged shifters, branch predicates, address generation, and
memory write enables derive committed transitions.

```rust
use cortex_m0_gatelevel::{encoding, CortexM0GateLevel};

let mut cpu = CortexM0GateLevel::new();
cpu.load(&encoding::program(&[encoding::movs(0, 7), encoding::bkpt(0)]), 0x100)?;
let result = cpu.run(2)?;
assert_eq!(result.final_state.registers[0], 7);
# Ok::<(), cortex_m0_gatelevel::CortexM0Error>(())
```

The package retains complete before/after state traces under a fixed 8 MiB
budget, uses typed atomic faults, and validates complete state on restore.
Release differential tests cover every implemented decode subfamily. The
completion baseline is 91.19% combined Rust line coverage (1,500/1,645).
