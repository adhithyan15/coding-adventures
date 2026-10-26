# Cortex-M4 gate-level simulator (Rust)

This package is the educational DFF-backed architectural gate companion to
`cortex-m4-simulator`. It implements the bounded Armv7E-M boundary in
[Spec 07am2](../../../specs/07am2-cortex-m4-gatelevel.md).

The accepted `cortex-m3-gatelevel` machine is the sole inherited executor.
Cortex-M4 DSP decode, saturating and parallel arithmetic, dual-halfword
multiply/add, optional FPv4-SP decode, binary32 arithmetic, and exception
status are reconstructed independently from repository gate networks.
Production code does not depend on the functional simulator and does not use
host floating-point arithmetic; `cortex-m4-simulator` is a test-only oracle.

Exactly 526,013 persistent bits are clocked through repository D flip-flops:
the 524,952-bit Cortex-M3 machine, APSR.GE, the FPU-enable latch, S0-S31, and
FPSCR. Complete snapshots also retain the inherited 64 KiB memory, lifecycle,
exclusive-monitor, privilege, and installed-image state.

```rust
use cortex_m4_gatelevel::{encoding, CortexM4GateLevel};

let mut cpu = CortexM4GateLevel::new();
cpu.set_fpu_enabled(true);
let program: Vec<u8> = encoding::vadd_f32(0, 1, 2)
    .into_iter()
    .flat_map(u16::to_le_bytes)
    .collect();
cpu.load(&program, 0x100)?;
cpu.write_s_register(1, 1.5_f32.to_bits())?;
cpu.write_s_register(2, 2.25_f32.to_bits())?;
cpu.step()?;
assert_eq!(cpu.read_s_register(0)?, 3.75_f32.to_bits());
# Ok::<(), cortex_m4_gatelevel::CortexM4Error>(())
```

The release suite differentially checks exact traces and complete state for
the inherited executor and every bounded DSP/FPU family, checks all inherited
primary classifications and reserved extension neighbours, and proves typed
atomic lifecycle rollback. Full-production source locks plus compile-valid
hostile parsed-syntax fixtures reject oracle routing, duplicate executors, raw
host decode, host DSP/float authority, hidden mirrors, and shadow helpers.
The completion baseline is 90.86% combined Rust line coverage (1,590/1,750).
