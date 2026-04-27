# intel8008-gatelevel

Gate-level simulation of the Intel 8008 microprocessor. Every arithmetic
operation routes through ripple-carry adders (built from XOR/AND/OR gates),
every register bit is held in a D flip-flop, and the instruction decoder
expresses all opcode detection as explicit gate calls. Results are
cross-validated against the behavioral `intel8008-simulator` crate.

## What this simulates

The Intel 8008 (1972) was Intel's first 8-bit processor — the architectural
ancestor of the x86 family. This crate models:

| Component         | Gate model                                    |
|-------------------|-----------------------------------------------|
| 8-bit ALU         | Ripple-carry adder (8 full-adders = 40 gates) |
| Parity flag       | 7-input XOR chain (7 XOR gates)               |
| 7×8-bit registers | D flip-flop arrays (7 × 8 = 56 flip-flops)   |
| 8-level stack     | D flip-flop arrays (8 × 14 = 112 flip-flops) |
| 14-bit PC         | Half-adder increment chain (28 gates)         |
| Decoder           | AND/OR/NOT gate network (≈48 gates)           |

## Architecture overview

```text
               ┌─────────────────────────────────────────┐
               │           GateLevelCpu                  │
               │                                         │
  memory[] ───►│  Decoder ──► control signals            │
               │      │                                  │
               │      ▼                                  │
               │  RegisterFile (7×8 flip-flops)          │
               │      │                                  │
               │      ▼                                  │
               │  GateAlu8 (ripple-carry adder)          │
               │      │                                  │
               │      ▼                                  │
               │  ProgramCounter (14-bit half-adder)     │
               │  PushDownStack  (8×14 flip-flops)       │
               └─────────────────────────────────────────┘
```

## Stack model

Unlike modern CPUs, the 8008 uses an 8-level circular push-down stack (no
stack pointer register). Slot 0 is always the live PC:

```text
CALL target:
  slot[7] ← slot[6] ← ... ← slot[1] ← slot[0]  (old return address saved)
  slot[0] ← target                               (jump to target)

RETURN:
  slot[0] ← slot[1] ← ... ← slot[6] ← slot[7]  (restore return address)
  slot[7] ← 0
```

## Dependencies

| Crate                            | Purpose                           |
|----------------------------------|-----------------------------------|
| `arithmetic`                     | Ripple-carry adder (via `alu()`)  |
| `logic-gates`                    | AND/OR/XOR/NOT + D flip-flop      |
| `coding-adventures-intel8008-simulator` | `Flags` and `Trace` types  |

## Usage

```rust
use coding_adventures_intel8008_gatelevel::GateLevelCpu;

let mut cpu = GateLevelCpu::new();

// A simple program: B=5, C=4, A=B+C, then HLT
let program: &[u8] = &[
    0x06, 0x05,  // MVI B, 5
    0x0E, 0x04,  // MVI C, 4
    0x78,        // MOV A, B
    0x81,        // ADD C
    0x76,        // HLT
];
cpu.load_memory(program);

let traces = cpu.run(program, 100);
for t in &traces {
    println!("{:04X}: {}  A={:02X}", t.address, t.mnemonic, t.a_after);
}
// 0000: MVI B,5   A=00
// 0002: MVI C,4   A=00
// 0004: MOV A,B   A=05
// 0005: ADD C     A=09
// 0006: HLT       A=09
```

## Cross-validation

The `test_cross_validate` test runs the same program through both
`GateLevelCpu` and the behavioral `Simulator` and asserts that every trace
entry has identical `a_after` and `flags_after`. This guarantees that the
gate-level simulation produces exactly the same externally observable state
as the reference implementation.

## Gate count

```
cargo test -p coding-adventures-intel8008-gatelevel -- test_gate_count
```

| Component                | Gates |
|--------------------------|-------|
| Ripple-carry adder (8b)  |    40 |
| Parity XOR tree          |     7 |
| PC half-adder chain (14) |    28 |
| Register file (56 FFs)   |   336 |
| Stack (112 FFs × 6)      |   672 |
| Decoder AND/OR/NOT       |    48 |
| **Estimated total**      | **1131** |

(Each D flip-flop = 6 gates; each full-adder = 5 gates.)
