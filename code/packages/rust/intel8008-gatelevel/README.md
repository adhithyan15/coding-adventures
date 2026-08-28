# intel8008-gatelevel

Gate-level Rust simulation of Intel's 1972 8008. It implements the same
instruction and lifecycle contract as `intel8008-simulator`, while routing
decode, arithmetic, logic, parity, increments, rotates, and program-counter
updates through the repository's logic-gate and arithmetic primitives.

## Persistent topology

Every mutable architectural bit is stored in a D flip-flop:

| Component | D flip-flops |
|---|---:|
| 16 KiB unified memory | 131,072 |
| B, C, D, E, H, L, A registers | 56 |
| Eight 14-bit stack words | 112 |
| Stack pointer/depth | 3 |
| CY, Z, S, P flags | 4 |
| Halt latch | 1 |
| Eight input ports | 64 |
| Twenty-four output ports | 192 |
| **Exact total** | **131,504** |

M is a memory pseudo-register and has no extra storage. The live 14-bit PC is
stack word zero, so it is not double-counted as a separate register.

Host integers are used only for control flow, indices, trace formatting, and
owned snapshots. Persistent state is DFF-backed. The public
`FLIP_FLOP_COUNT` constant and `flip_flop_count()` method expose the exact
topology.

## Checked API

```rust
use coding_adventures_intel8008_gatelevel::GateLevelCpu;

let mut cpu = GateLevelCpu::new();
let program = [
    0x06, 5,    // MVI B, 5
    0x0E, 4,    // MVI C, 4
    0x78,       // MOV A, B
    0x81,       // ADD C
    0x76,       // HLT
];
let traces = cpu.run(&program, 100)?;
assert_eq!(cpu.a(), 9);
# Ok::<(), coding_adventures_intel8008_simulator::Intel8008Error>(())
```

`load_program`, `step`, `run`, and I/O port access return the shared
`Intel8008Error`. Invalid ranges, undefined opcodes, instructions that cross
address `0x3FFF`, halted execution, and invalid ports are rejected atomically.
`run` is caller-bounded and commits its new machine only after successful
execution.

`snapshot()` returns the same owned `Intel8008State` shape as the functional
simulator: all registers, 16 KiB of memory, all stack words, stack depth,
flags, halt state, and I/O latches.

## Conformance

The completion suite:

- classifies all 256 possible first opcode bytes;
- compares exact trace and complete state after every defined encoding;
- verifies atomic unknown, truncated, halted, oversized, and port failures;
- compares multi-instruction memory, branch, call/return, ALU, and I/O
  workloads with the functional oracle;
- pins the exact 131,504-DFF topology.

Run the package checks from `code/packages/rust`:

```text
cargo test -p coding-adventures-intel8008-gatelevel --no-fail-fast
cargo clippy -p coding-adventures-intel8008-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding-adventures-intel8008-gatelevel --no-deps
```
