# Cortex-M7 gate-level simulator (Rust)

This package is the DFF-backed architectural gate companion to
`cortex-m7-simulator`. It implements the bounded Armv7E-M boundary from
[Spec 07an2](../../../specs/07an2-cortex-m7-gatelevel.md), reusing the
independent Cortex-M3 gate engine for inherited instructions and adding its
own compiled gate-backend DSP network.

All 524,956 persistent bits are authoritative repository D flip-flops:
524,952 inherited Cortex-M3 bits plus four APSR.GE bits. The added decoder,
QADD/QSUB saturation and sticky-Q path, four independent SADD8/UADD8 lanes,
IT and destination write enables, and GE transport execute through one
backend that production binds to repository gates. Hostile-backend tests run
the identical compiled helpers and prove architectural dependence on those
primitives.

```rust
use cortex_m7_gatelevel::{encoding, CortexM7GateLevel};

let mut cpu = CortexM7GateLevel::new();
let dsp = encoding::qadd(0, 1, 2);
cpu.load(&encoding::program(&[dsp[0], dsp[1], encoding::bkpt(0)]), 0x100)?;
cpu.write_register(1, 4)?;
cpu.write_register(2, 5)?;
let result = cpu.run(2)?;
assert_eq!(result.final_state.core.registers[0], 9);
# Ok::<(), cortex_m7_gatelevel::CortexM7Error>(())
```

The lifecycle preallocates every complete-state buffer before transitions,
reports typed allocation failures, revalidates settled inherited state at the
final fallible commit boundary before clocking inherited or GE DFFs, and rolls
back whole `run` and `execute` calls on fault. Caches, TCM, buses, timing,
NVIC, MPU, FPU, debug, and vendor peripherals remain explicit later CPU/SoC
cells.

The completion suite reports 93.94% package-source line coverage (697/742).
