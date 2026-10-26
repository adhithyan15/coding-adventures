# Cortex-M7 simulator

This package is a checked functional model of one bounded Arm Cortex-M7 core.
It composes the completed Cortex-M3 model, preserving its full Armv7-M state
and instruction surface, and adds a focused Armv7E-M DSP slice: QADD, QSUB,
SADD8, UADD8, sticky Q, and GE[3:0]. See
[Spec 07an](../../../specs/07an-cortex-m7-simulator.md).

Loads, restore, step, bounded run, and execute are typed and transactional.
Every successful step retains exact little-endian instruction bytes, width,
mnemonic, and complete before/after state. The public snapshot includes all
inherited registers, status and privilege state, local exclusive monitor,
power/event state, installed-image metadata, owned 64 KiB memory, and GE.
`try_get_state` provides a fallible complete snapshot for callers that require
typed allocation errors. Checked execution uses that path internally and
rolls back if a post-transition snapshot cannot be retained. MRS/MSR transport
GE in architectural APSR bits 19:16, including the composite APSR aliases.

```rust
use cortex_m7_simulator::{encoding, CortexM7Simulator};

let words = encoding::qadd(0, 1, 2);
let program = encoding::program(&words);
let mut cpu = CortexM7Simulator::new();
cpu.load(&program, 0x100)?;
cpu.write_register(1, i32::MAX as u32)?;
cpu.write_register(2, 1)?;
let trace = cpu.step()?;
assert_eq!(trace.mnemonic, "qadd");
assert_eq!(cpu.read_register(0)?, i32::MAX as u32);
# Ok::<(), cortex_m7_simulator::CortexM7Error>(())
```

## Hardware composition boundary

The Cortex-M7 can be integrated with instruction/data caches, ITCM/DTCM,
branch prediction, AXI/AHB buses, an MPU, interrupt control, and vendor
peripherals. Those structures affect traffic and timing, not the scalar
architectural result modeled here. They remain an explicit SoC/gate-level
composition boundary rather than being approximated as hidden functional
state. The optional floating-point and remaining DSP instruction surfaces are
also named follow-up cells.

## Verification

The completion suite covers inherited decode and execution, every added DSP
family, saturation and sticky-Q edges, every parallel lane and GE rule,
IT-block behavior, exact traces, complete-state restore, typed atomic failures,
false-IT reserved-operand rejection, MRS/MSR GE transport, deterministic
allocation-failure rollback, bounded rollback, and the documented SoC boundary. Release tests, strict
Clippy, rustdoc warnings, BUILD integration, and line coverage above 80% are
required before publication.

The completion baseline is **97.29% line coverage (323/332 lines)** under the
LLVM coverage engine.
