# intel4004-gatelevel (Rust)

Gate-level Intel 4004 CPU simulator where **every computation routes through real logic gates**.

## What makes this "gate-level"?

Every arithmetic operation flows through the same gate chain that the real Intel 4004 used:

```text
NOT/AND/OR/XOR -> half_adder -> full_adder -> ripple_carry_adder -> ALU
D flip-flop -> register -> register file / program counter / stack
```

When you execute `ADD R3`, the value in R3 is read from flip-flops, the accumulator is read from flip-flops, both are fed into the ALU (which uses full adders built from gates), and the result is clocked back into the accumulator's flip-flops.

All persistent architectural bits (other than read-only program ROM) are held
in D flip-flops, and every arithmetic, logical, decode, increment, and selector
datapath passes through gate functions. Host values are used only for control,
trace bookkeeping, and ROM addressing.

## How it fits in the stack

This package sits at the top of the computing stack:

```text
logic-gates (NOT, AND, OR, XOR, D flip-flop, register)
    |
arithmetic (half_adder, full_adder, ripple_carry_adder, ALU)
    |
intel4004-gatelevel (this package)
```

## Architecture

| Module      | Description                                          | Gate Count |
|-------------|------------------------------------------------------|-----------|
| `gate_alu`  | 4-bit ALU (add, subtract, complement, AND, OR)       | 32        |
| `registers` | 16x4-bit register file + accumulator + carry flag     | 510       |
| `pc`        | 12-bit program counter with half-adder incrementer    | 96        |
| `stack`     | 3-level stack plus pointer (38 flip-flops)            | 238       |
| `ram`       | Main/status RAM plus outputs (1,296 flip-flops)       | 7,976     |
| `decoder`   | Combinational instruction decoder (AND/OR/NOT gates)  | ~50       |
| `cpu`       | 1,428 exact persistent D flip-flops, excluding ROM    | >8,000    |

## Usage

```rust
use intel4004_gatelevel::Intel4004GateLevel;

let mut cpu = Intel4004GateLevel::new();

// LDM 5 (load immediate 5), HLT (halt)
let traces = cpu.run(&[0xD5, 0x01], 100).unwrap();
assert_eq!(cpu.accumulator(), 5);
assert!(cpu.halted());

// Inspect gate count
println!("Total gates: {}", cpu.gate_count());
```

## Supported Instructions

All 46 Intel 4004 instructions are implemented:

- **Data movement**: NOP, HLT, LDM, LD, XCH, FIM, SRC, FIN, JIN
- **Arithmetic**: ADD, SUB, INC, IAC, DAC
- **Logic**: CMA (complement), KBP (keyboard process)
- **Control flow**: JUN, JCN, ISZ, JMS, BBL
- **Carry operations**: CLB, CLC, CMC, STC, TCC, TCS, DAA
- **Rotate**: RAL, RAR
- **RAM I/O**: WRM, RDM, WR0-WR3, RD0-RD3, SBM, ADM
- **Port I/O**: WRR, RDR, WMP, WPM
- **Bank select**: DCL

## Dependencies

- `logic-gates` -- fundamental gates and sequential circuits
- `arithmetic` -- adder circuits and ALU
- `intel4004-simulator` -- typed execution errors and the differential oracle

## Testing

```bash
cargo test -p intel4004-gatelevel
```

The suite contains 88 component/unit tests and four full-machine integration
tests. It differentially executes every one of the 254 specified first-byte
encodings plus RAM, port, branch, indirect, stack, reset, topology, and atomic
failure workloads against `intel4004-simulator`. Core line coverage is 95.15%
(686/721).
