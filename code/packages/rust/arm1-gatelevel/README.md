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

The original 21 unit/program tests remain green. Eight lifecycle tests cover
topology and typed transactional boundaries. An aggregate 599-vector test
matches every full-state Python/functional hash across all conditions,
instructions, and addressing families. Strict formatting, Clippy, and rustdoc
pass; total Rust line coverage is 93.12% (1,042/1,119).

See `code/specs/07e2-arm1-gatelevel.md` for the normative contract.

## Development

```bash
bash BUILD
```
