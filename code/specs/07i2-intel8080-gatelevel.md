# 07i2 — Intel 8080 Gate-Level Simulator

## Purpose

`coding-adventures-intel8080-gatelevel` is the Rust gate-level companion to
`intel8080-simulator`. Both implement the 1974 Intel 8080A ISA and expose the
same observable lifecycle, state, trace, and error types. The functional model
is the behavioral oracle; this model makes persistent state and arithmetic
datapaths explicit in gates.

## Persistent topology

Every persistent bit is represented by `logic_gates::sequential::FlipFlopState`
and changes through `logic_gates::sequential::register`.

| Component | Width | D flip-flops |
|-----------|------:|-------------:|
| Main memory | 65,536 × 8 | 524,288 |
| A, B, C, D, E, H, L | 7 × 8 | 56 |
| Program counter | 16 | 16 |
| Stack pointer | 16 | 16 |
| S, Z, AC, P, CY flags | 5 | 5 |
| Interrupt-enable latch | 1 | 1 |
| Halt latch | 1 | 1 |
| Input ports | 256 × 8 | 2,048 |
| Output ports | 256 × 8 | 2,048 |
| **Total** | | **528,479** |

The M register code does not allocate storage; it addresses memory through HL.
Host-language booleans used while evaluating one instruction are transient
wires. They are loaded from, and clocked back into, the persistent DFF bank at
the instruction boundary.

## Combinational datapaths

- Opcode grouping and register fields are decoded with AND, OR, and NOT gates.
- Eight- and sixteen-bit addition use ripple-carry full-adder chains.
- Subtraction and decrement use gate inversion plus ripple-carry addition.
- Logical operations, parity, sign, zero, carry, and auxiliary carry are
  derived from gate primitives.
- Register, PC, SP, memory, flag, control, and I/O writes clock DFF state.

The Rust control layer sequences fetch, decode, operand routing, and writes. It
does not claim transistor-accurate timing or reproduce the physical 8080A die.

## ISA contract

The simulator accepts the 244 defined first-byte encodings and rejects these 12
undefined bytes atomically:

```text
08 10 18 20 28 30 38 CB D9 DD ED FD
```

It implements data transfer, arithmetic, logical, rotate, branch, call/return,
stack, restart, I/O, interrupt-control, and halt instructions. XTHL (`E3`),
PCHL (`E9`), XCHG (`EB`), and SPHL (`F9`) are special group-3 encodings and must
be dispatched before the regular group handler.

## Public lifecycle

`GateLevelCpu` provides:

- `load`, which rejects oversized images without mutation;
- `step`, which preflights unknown, truncated, and halted states atomically and
  returns a complete before/after `StepTrace`;
- `run`, which is bounded and transactional on error;
- `snapshot`/`state` and `restore` for complete owned architectural state;
- checked memory and port accessors; and
- `reset`, which clears all persistent state.

The shared `Intel8080State` owns registers, flags, all 64 KiB of memory, PC,
halt and interrupt state, and every input/output latch.

## Completion contract

The package is complete when all of the following pass:

1. The topology test pins `FLIP_FLOP_COUNT` to exactly 528,479.
2. All 256 first bytes are classified as 244 defined and 12 undefined.
3. Starting from a non-trivial common snapshot, every defined opcode produces
   the same raw bytes and full before/after state as `intel8080-simulator`.
4. Undefined, truncated, oversized, and halted operations return matching typed
   errors and preserve state.
5. Multi-instruction arithmetic, stack, memory, I/O, and halt workloads match
   the functional simulator at every instruction boundary.
6. Unit, integration, and documentation tests, strict Clippy, strict rustdoc,
   formatting, and the repository package build all pass.

## Example

```rust
use coding_adventures_intel8080_gatelevel::GateLevelCpu;

let mut cpu = GateLevelCpu::new();
let result = cpu
    .run(&[0x3E, 10, 0x06, 5, 0x80, 0x76], 100)
    .unwrap();
assert!(result.halted);
assert_eq!(result.final_state.regs.a, 15);
```
