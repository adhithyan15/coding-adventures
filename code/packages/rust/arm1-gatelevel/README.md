# arm1-gatelevel

Complete Rust gate-level simulator for ARM1 / ARMv1. Condition evaluation,
barrel shifting, and the 32-bit ALU route through the repository's logic-gate
and ripple-carry primitives. Every persistent architectural bit has stable DFF
identity.

## Exact topology

- 536,870,912 DFFs for the 64 MiB unified memory
- 864 DFFs for all 27 physical 32-bit banked registers
- 1 DFF for HALT
- **536,871,777 DFFs total** (`FLIP_FLOP_COUNT`)

Memory and registers use a packed stable-Q representation. On every
simulator-owned write, the internal master/slave latch state is reconstructed
and clocked through `logic_gates::sequential::register`.

The HALT DFF Q output is the sole architectural halt authority. The historical
private host Boolean is only a synchronized observation mirror: deliberately
diverging it cannot change snapshots, execution, or fault behavior. A concrete
physical-storage inventory and exact source-transition locks prevent a host
mirror, direct-Q assignment, helper indirection, dead gate anchor, or
post-dispatch overwrite from replacing the clocked path.

## Checked lifecycle

```rust
use arm1_gatelevel::ARM1GateLevel;
use arm1_simulator::{encode_halt, encode_mov_imm, COND_AL};

let words = [encode_mov_imm(COND_AL, 0, 42), encode_halt()];
let code: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
let mut cpu = ARM1GateLevel::new(4096);
let result = cpu.run_checked(&code, 10)?;
assert!(result.halted);
assert_eq!(result.final_state.regs[0], 42);
# Ok::<(), arm1_simulator::Arm1Error>(())
```

The gate and functional crates share complete `Arm1State`, `Arm1Error`,
`StepTrace`, and `ExecutionResult` types. Checked load, restore, register/memory,
step, and run operations fail atomically. The gate simulator also implements
force-user LDM/STM transfers and external mask-aware IRQ/FIQ entry.

## Conformance

Twenty-three unit/program tests, eight lifecycle tests, three structural
integrity tests, and the aggregate 599-vector differential are green. They
cover the concrete topology, both deliberate mirror divergences, every HALT
lifecycle transition, typed rollback, and hostile source mutations as well as
all prior conditions, instructions, and addressing families. Strict
formatting, Clippy, and rustdoc pass; total Rust line coverage is 93.27%
(1,189/1,269).

See `code/specs/07e2-arm1-gatelevel.md` for the normative contract.

## Development

```bash
bash BUILD
```
