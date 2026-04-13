# Intel 8008 Simulator

Behavioral simulation of the Intel 8008 — the world's first 8-bit microprocessor (1972).

## What is the Intel 8008?

Released in April 1972, one year after the 4-bit Intel 4004, the 8008 was designed for
Computer Terminal Corporation's Datapoint 2200 terminal. CTC rejected it as too slow; Intel
sold it commercially. The 8008 inspired the 8080, which inspired the Z80 and x86 — making
this rejected terminal chip the ancestor of modern desktop processors.

Specifications: ~3,500 PMOS transistors, 500–800 kHz, 8-bit data, 14-bit address space,
7 general-purpose registers, 4 flags, 8-level push-down stack.

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → Lexer → Parser → Compiler
```

This is Layer 4f alongside RISC-V (07a), ARM/ARMv7 (07b), WASM (07c), Intel 4004 (07d), ARM1 (07e).

## Usage

```typescript
import { Intel8008Simulator } from "@coding-adventures/intel8008-simulator";

const sim = new Intel8008Simulator();

// Compute 1 + 2 = 3, then halt
// MVI B, 0x01  (0x06, 0x01)
// MVI A, 0x02  (0x3E, 0x02)
// ADD B        (0x80)
// HLT          (0x76)
const program = new Uint8Array([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76]);
const traces = sim.run(program);
console.log(sim.a);                    // 3
console.log(sim.currentFlags.carry);   // false
console.log(sim.currentFlags.zero);    // false
console.log(sim.currentFlags.parity);  // true (0x03 has 2 ones = even parity)
```

## Architecture

### Registers

| Register | Index | Purpose |
|----------|-------|---------|
| B | 0 | General purpose |
| C | 1 | General purpose |
| D | 2 | General purpose |
| E | 3 | General purpose |
| H | 4 | High byte of memory address pair |
| L | 5 | Low byte of memory address pair |
| M | 6 | Pseudo-register: memory at [H:L] |
| A | 7 | Accumulator (ALU destination) |

M is not a physical register. When used as source/destination, it reads/writes
memory at address `(H & 0x3F) << 8 | L`.

### Flags

| Flag | Set when... |
|------|-------------|
| CY (Carry) | Addition overflows 8 bits, or subtraction borrows |
| Z (Zero) | Result is exactly 0x00 |
| S (Sign) | Bit 7 of result is 1 |
| P (Parity) | Result has an even number of 1-bits |

### Stack

8-level hardware push-down stack. Entry 0 is always the current PC.
CALL rotates entries down; RETURN rotates entries up. Programs can
nest at most 7 calls deep.

## Instruction Set Summary

| Group | Instructions |
|-------|-------------|
| MOV D, S | Register-to-register transfer (1 byte) |
| MVI D, d | Move immediate (2 bytes) |
| INR D / DCR D | Increment/decrement (1 byte; preserves CY) |
| ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP S | ALU register (1 byte) |
| ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI d | ALU immediate (2 bytes) |
| RLC/RRC/RAL/RAR | Accumulator rotate (1 byte) |
| JMP/JFC/JTC/JFZ/JTZ/JFS/JTS/JFP/JTP | Jump (3 bytes) |
| CAL/CFC/CTC/CFZ/CTZ/CFS/CTS/CFP/CTP | Call (3 bytes) |
| RET/RFC/RTC/RFZ/RTZ/RFS/RTS/RFP/RTP | Return (1 byte) |
| RST N | Restart to N*8 (1 byte) |
| IN P / OUT P | I/O (1 byte) |
| HLT | Halt (0x76 or 0xFF) |

## API

```typescript
class Intel8008Simulator {
  // Registers (read-only)
  get a(): number;  get b(): number;  get c(): number;
  get d(): number;  get e(): number;  get h(): number;  get l(): number;
  get pc(): number;
  get hlAddress(): number;  // (H & 0x3F) << 8 | L

  // Flags
  get currentFlags(): Flags;  // { carry, zero, sign, parity }

  // Memory
  get memory(): Uint8Array;  // 16,384 bytes

  // Stack
  get stack(): number[];  // 8 entries; [0] = PC
  get depth(): number;    // 0–7

  // Execution
  loadProgram(program: Uint8Array, startAddress?: number): void;
  step(): Trace;
  run(program: Uint8Array, maxSteps?: number, startAddress?: number): Trace[];
  reset(): void;

  // I/O
  setInputPort(port: number, value: number): void;  // port 0–7
  getOutputPort(port: number): number;              // port 0–23
}
```
