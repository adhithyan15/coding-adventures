# Cortex-M3 gate-level simulator (Rust)

This package is the DFF-backed architectural gate companion to
`cortex-m3-simulator`. It implements the checked Armv7-M Thumb boundary from
[Spec 07al2](../../../specs/07al2-cortex-m3-gatelevel.md) with an independent
gate transition engine. Production code does not depend on the functional
simulator; that package is a differential-test oracle only.

All 524,952 persistent bits — 64 KiB memory, integer and special registers,
mode, halt/sleep/event state, the exclusive monitor, and installed-image
metadata — are clocked through repository D flip-flops. Gate-built decoders,
ripple arithmetic, Boolean logic, staged shifters and dividers, branch
predicates, address generation, and memory write enables derive committed
transitions.

```rust
use cortex_m3_gatelevel::{encoding, CortexM3GateLevel};

let mut cpu = CortexM3GateLevel::new();
cpu.load(&encoding::program(&[encoding::movs(0, 7), encoding::bkpt(0)]), 0x100)?;
let result = cpu.run(2)?;
assert_eq!(result.final_state.registers[0], 7);
# Ok::<(), cortex_m3_gatelevel::CortexM3Error>(())
```

The package retains complete before/after state traces under a fixed 8 MiB
budget, uses typed atomic faults, and validates complete state on restore.
Its supported boundary includes the Cortex-M0-compatible Thumb surface plus
IT, CBZ/CBNZ, wide moves and memory transfers, division, bit operations,
table branches, exclusive transfers, barriers, low-power/event behavior,
BASEPRI, FAULTMASK, and special-register views. Release differential tests
compare every implemented decode subfamily with the functional oracle. The
completion baseline was measured separately on the two branches merged here:
93.19% combined Rust line coverage (2,229/2,392) with the APSR MSR mask repair
and 92.06% (2,343/2,545) with the gate-to-gate integration boundary. The merged
total has not been re-measured.

APSR MSR mask classification and NZCVQ write enable are gate-derived. The
non-DSP Cortex-M3 accepts NZCVQ-only writes while G-only, combined, and
reserved masks fault atomically in exact functional lockstep. The default
`msr` encoder remains NZCVQ-compatible; typed `ApsrMask` values expose exact
forms for composition and verification without integer truncation.

Repository gate companions can compose this engine through the narrow
`integration` module. It snapshots into caller-preallocated memory, derives
delegated traces without clocking Q, validates opaque candidates before an
explicit fallible commit, revalidates publicly mutable candidate values at
that final boundary, exposes deterministic allocation checkpoints, and
derives the physical inventory from the live DFF containers. It never exposes
mutable access to the authoritative DFF containers.
