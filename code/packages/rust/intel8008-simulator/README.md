# intel8008-simulator

Behavioral simulator for the Intel 8008 — the world's first 8-bit microprocessor (1972).

## Overview

The Intel 8008 was designed by Ted Hoff, Stanley Mazor, and Hal Feeney at Intel, originally for Computer Terminal Corporation's Datapoint 2200 terminal. CTC rejected the chip, but Intel released it commercially in April 1972. The 8008 directly inspired the 8080, the Z80, and the entire x86 architecture — making this 3,500-transistor chip the ancestor of every modern x86 processor.

This is a **behavioral simulator**: it executes 8008 machine code directly using Rust host arithmetic, producing correct results without modeling internal gates. For gate-level simulation (every operation routed through AND/OR/XOR/NOT gates), see `intel8008-gatelevel`.

## Architecture

| Feature          | Value                                               |
|------------------|-----------------------------------------------------|
| Data width       | 8 bits                                              |
| Registers        | A (accumulator), B, C, D, E, H, L + M (memory ptr) |
| Flags            | Carry (CY), Zero (Z), Sign (S), Parity (P)          |
| Program counter  | 14 bits (16 KiB address space)                      |
| Stack            | 8-level internal push-down stack (7 usable)         |
| Memory           | 16,384 bytes                                        |
| I/O              | 8 input ports, 24 output ports                      |
| Transistors      | ~3,500 (PMOS, 10 μm process)                        |

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → Compiler → VM
```

This is Layer 7f, alongside RISC-V (07a), ARM/ARMv7 (07b), WASM (07c), Intel 4004 (07d), and ARM1 (07e).

## Usage

```rust
use coding_adventures_intel8008_simulator::Simulator;

let mut sim = Simulator::new();

// MVI B,1  (0x06 0x01)
// MVI A,2  (0x3E 0x02)
// ADD B    (0x80)
// HLT      (0x76)
let program = &[0x06u8, 0x01, 0x3E, 0x02, 0x80, 0x76];
let traces = sim.run(program, 100);

assert_eq!(sim.a(), 3);
assert!(!sim.flags().carry);
```

### Multiply 4 × 5 via repeated addition

```rust
use coding_adventures_intel8008_simulator::Simulator;

let mut sim = Simulator::new();
// MVI B,5; MVI C,4; MVI A,0; LOOP: ADD B; DCR C; JFZ LOOP; HLT
let mut program = vec![0u8; 20];
program[0] = 0x06; program[1] = 0x05;  // MVI B, 5
program[2] = 0x0E; program[3] = 0x04;  // MVI C, 4
program[4] = 0x3E; program[5] = 0x00;  // MVI A, 0
program[6] = 0x80;                      // ADD B
program[7] = 0x09;                      // DCR C
program[8] = 0x48; program[9] = 0x06; program[10] = 0x00; // JFZ 6
program[11] = 0x76;                     // HLT
sim.run(&program, 200);
assert_eq!(sim.a(), 20);
```

## The Push-Down Stack

The 8008 stack is a hardware push-down automaton — stack entry[0] IS the program counter. On CALL, the stack rotates down and the target loads into entry[0]. On RETURN, the stack rotates up, restoring the saved return address. Programs can nest calls at most 7 levels deep.

## Instruction Set Overview

| Group | Encoding | Instructions |
|-------|----------|-------------|
| 00    | `00 DDD 000/001` | INR, DCR |
| 00    | `00 0RR 010` | RLC, RRC, RAL, RAR |
| 00    | `00 CCC 011/111` | Conditional returns |
| 00    | `00 AAA 101` | RST (restart / interrupt vectors) |
| 00    | `00 DDD 110` | MVI (move immediate) |
| 01    | `01 DDD SSS` | MOV (register-to-register) |
| 01    | `01 110 110` | HLT |
| 01    | `01 CCC T00` | Conditional jumps (JFC, JTC, JFZ, JTZ, ...) |
| 01    | `01 CCC T10` | Conditional calls (CFC, CTC, CFZ, CTZ, ...) |
| 01    | `01 PPP 001` | IN (read input port) |
| 10    | `10 OOO SSS` | ALU register: ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP |
| 11    | `11 OOO 100` | ALU immediate: ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI |

## Notes on Opcode Encoding

The 8008 has some encoding quirks where control flow opcodes overlap with MOV:

- `0x76` (MOV M,M) → **HLT**
- `0x7E` (MOV A,M) → **CAL** (unconditional call, 3 bytes)
- `0x7C` (MOV A,H) → **JMP** (unconditional jump, 3 bytes)
- `0x79` (MOV A,C) → **IN 7** (input from port 7)

These are not bugs — they are intentional design choices in the 8008 encoding.

## How It Fits in the Stack

This package sits between the gate-level CPU simulation and the assembler. The behavioral simulator executes the binary machine code that an assembler produces. The gate-level simulator (`intel8008-gatelevel`) produces identical results but routes every operation through actual logic gate functions.
