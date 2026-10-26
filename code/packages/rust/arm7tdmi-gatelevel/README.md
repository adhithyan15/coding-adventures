# ARM7TDMI gate-level simulator

This package is the educational DFF-backed companion to the checked ARM7TDMI
functional simulator. It implements the complete bounded ARMv4T ARM and Thumb
integer boundary from [Spec 07ax2](../../../specs/07ax2-arm7tdmi-gatelevel.md):
31 physical GPRs, five SPSRs, CPSR, pipeline-visible PC behavior, little-endian
memory, bank and instruction-state exchange, exact traces, and transactional
lifecycle behavior.

Exactly 525,522 stable-Q DFF bits own every persistent architectural bit.
Repository gates and arithmetic primitives form the educational decode,
condition, shifter, ALU/flags, control, and storage networks. The functional
crate is a development-only differential oracle, never a production backend.

This is a repository-owned ISA-level teaching topology. It is not Arm's
proprietary ARM7TDMI netlist, a transistor model, or a cycle-accurate pipeline,
bus, cache, EmbeddedICE, or SoC simulation.

Completion evidence is 59 release tests, including full inherited functional
contracts, independent ARM/Thumb full-state and exact-trace differentials,
every publicly reachable typed functional fault with exact full-state
rollback, explicit narrowing evidence for compatibility-only unreachable
fault variants, exhaustive condition lines, arithmetic/shift/multiply
boundaries, a typed mid-commit rollback seam, and compiler-checked hostile
source mutations. Syntax-normalized whole-file token locks and named parsed-body
source-to-sink locks cover the complete ARM and Thumb decode,
suboperation/fixed-bit selectors, destination enables, mnemonic/trace control,
condition, arithmetic, flags, banks, memory,
BX exchange, stable-Q storage, and public lifecycle routes. Host
arithmetic/decode, oracle substitution, dead anchors, direct Q/memory writes,
post-gate overwrite, and post-commit overwrite are all rejected after their
isolated full-crate mutations type-check. Two-way uncommitted-candidate
divergence tests prove that only stable Q can authorize behavior. Measured
release line coverage is 91.67% (2,168/2,365).

```rust
use arm7tdmi_gatelevel::{Arm7TdmiGateLevel, InstructionSet};

let mut cpu = Arm7TdmiGateLevel::new();
cpu.load(&0xef00_0000u32.to_le_bytes(), 0x100, InstructionSet::Arm)?;
let trace = cpu.step()?;
assert_eq!(trace.mnemonic, "SWI 0");
assert!(cpu.get_state().halted);
# Ok::<(), arm7tdmi_gatelevel::Arm7TdmiError>(())
```
