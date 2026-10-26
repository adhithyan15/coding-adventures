# ARM7TDMI simulator (Rust)

A checked, deterministic instruction-level model of the ARM7TDMI core's
bounded ARMv4T integer architecture. The normative contract is
[Spec 07ax](../../../specs/07ax-arm7tdmi-simulator.md), based on Arm DDI 0100
and DDI 0029G.

## Modeled boundary

- all 31 physical general-purpose registers, seven processor modes, CPSR, and
  the five banked SPSRs;
- the specified ARM data-processing, multiply, transfer, status, swap, branch,
  exchange, and deterministic halt families;
- the complete ARMv4T Thumb-1 integer surface, including both BL halfwords;
- ARM +8/+12 and Thumb +4 programmer-visible PC rules;
- checked little-endian byte, halfword, word, multiple, stack, and swap access;
- exact complete-state traces and transactional `step`, `run`, and `execute`;
  and
- typed rejection of reserved, unsupported, unpredictable, privilege,
  alignment, range, fetch, retention, and lifecycle failures.

The package models a CPU core, not a chip. Exception entry, IRQ/FIQ delivery,
coprocessors, EmbeddedICE, AMBA, caches, MMU/MPU, DMA, peripherals, and timing
remain explicit follow-up work.

## Example

```rust
use arm7tdmi_simulator::{Arm7TdmiSimulator, InstructionSet};

let mut cpu = Arm7TdmiSimulator::new();
// MOV r0,#42 ; SWI 0 (the repository-only halt sentinel)
let program = [0x2a, 0x00, 0xa0, 0xe3, 0x00, 0x00, 0x00, 0xef];
let result = cpu.execute(&program, 0, InstructionSet::Arm, 2)?;
assert_eq!(result.final_state.registers.common_r0_r7[0], 42);
assert!(result.halted);
# Ok::<(), arm7tdmi_simulator::Arm7TdmiError>(())
```

## Verification

The completion contract covers every included ARM/Thumb family, all ARM
conditions, bank selection, shifter and arithmetic boundaries, multiply
widths, status access, state exchange and low target bits, PC visibility,
transfer widths and addressing modes, full-span multiple/stack operations,
exact traces, typed failures, and transactional lifecycle behavior. The
release suite, strict Clippy, rustdoc, Rustfmt, BUILD recipe, and lessons
validation pass across 38 tests. Current package line coverage is
**88.12% (1,387/1,574)**.
