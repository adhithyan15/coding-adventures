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

The 21-test release suite differentially checks exact full traces and complete
state for every inherited primary family; representative integer, privilege,
IT, memory, exclusive, branch, power, and fault paths; and every bounded
DSP/FPU family. It also proves typed atomic `run` and `execute` rollback after
one instruction has committed. Exact production source locks plus seven
hostile complete temporary crates reject oracle routing, duplicate executors,
raw host decode, host DSP/float authority, hidden mirrors, and shadow helpers;
every mutation must compile before rejection. The completion baseline is
91.66% combined Rust line coverage (1,627/1,775).
