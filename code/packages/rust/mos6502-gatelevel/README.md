# MOS 6502 gate-level simulator

This Rust package models the 1975 NMOS MOS 6502 at instruction level while
routing its arithmetic, logic, shifts, comparisons, address calculation, and
zero detection through the repository's gate primitives. All 528,184
persistent bits are backed by D flip-flops.

## Scope

- All 151 official NMOS 6502 opcodes and all 13 addressing modes.
- Exact 64 KiB memory, A/X/Y/S/PC registers, seven stored status flags, halt
  latch, and 240 input plus 240 output latches.
- Memory-mapped I/O at `$FF00..=$FFEF`.
- NMOS decimal ADC/SBC behavior and the JMP indirect page-wrap quirk.
- Typed atomic errors, transactional bounded runs, immutable full-state
  snapshots, and before/after instruction traces shared with
  `mos6502-simulator`.
- IRQ and NMI entry helpers in addition to the functional simulator contract.

BRK follows the repository simulator convention: it updates the architectural
stack state and halts instead of dispatching through the IRQ vector.

## Quick start

```rust
use coding_adventures_mos6502_gatelevel::GateLevelCpu;

let mut cpu = GateLevelCpu::new();
let result = cpu
    .run(&[0xA9, 10, 0x69, 5, 0x00], 100)
    .unwrap();

assert_eq!(result.final_state.a, 15);
assert!(result.halted);
```

## Persistent topology

| Component | D flip-flops |
|---|---:|
| 64 KiB memory | 524,288 |
| A, X, Y, and S | 32 |
| PC | 16 |
| N, V, B, D, I, Z, and C | 7 |
| Halt latch | 1 |
| 240 input latches | 1,920 |
| 240 output latches | 1,920 |
| **Total** | **528,184** |

`FLIP_FLOP_COUNT` exposes this exact total for topology tests.

## Verification

The completion suite compares the complete gate and functional state for every
possible opcode byte: all 151 official instructions must agree and all 105
undefined bytes must return the same typed error without changing state. It
also covers transactional lifecycle failures, exact topology, multi-instruction
workloads, and memory-mapped I/O.

- 81 unit tests
- 5 integration tests
- 7 documentation tests
- 96.60% core line coverage (994/1,029)
- formatting, Clippy with warnings denied, and rustdoc with warnings denied

## Source layout

| File | Responsibility |
|---|---|
| `bits.rs` | LSB-first bit conversion, ripple adders, gate zero detection |
| `alu.rs` | Binary/BCD arithmetic, logic, compare, shifts, and rotates |
| `decoder.rs` | Gate-assisted decode and the 151-opcode PLA table |
| `registers.rs` | DFF-backed registers, flags, and stack helpers |
| `state.rs` | DFF-backed scalar registers and 64 KiB memory |
| `cpu.rs` | Lifecycle, fetch/decode/execute, memory-mapped I/O, IRQ/NMI |
