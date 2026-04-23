# @coding-adventures/intel8008-gatelevel

Gate-level simulator for the Intel 8008 microprocessor — all computations route through real logic gate functions (AND, OR, XOR, NOT) from the `logic-gates` and `arithmetic` packages.

## What is this?

The Intel 8008 (April 1972) was the world's first 8-bit microprocessor. It had ~3,500 PMOS transistors, a 14-bit address bus (16 KiB memory), 7 general-purpose registers, a 4-flag status register, and an 8-level on-chip push-down stack.

This package simulates the 8008 at the **logic gate level**: every addition uses the ripple-carry adder chain (8 full-adders built from AND/XOR/OR gates), every register write routes through D flip-flop gate simulation, and the 14-bit program counter increments via a half-adder chain. Nothing uses host arithmetic shortcuts.

## How it fits in the computing stack

```
transistors          CMOS pairs: PMOS + NMOS transistor models
     ↓
logic-gates          AND, OR, XOR, NOT built from CMOS pairs
     ↓
arithmetic           Half-adder, full-adder, ripple-carry adder, ALU
     ↓
intel8008-gatelevel  ← YOU ARE HERE (routes everything through gates)
     ↓
intel8008-simulator  Behavioral reference (host arithmetic, same public API)
```

## Usage

```typescript
import { Intel8008GateLevel } from "@coding-adventures/intel8008-gatelevel";

const cpu = new Intel8008GateLevel();

// Load B=1, A=2, compute A = A + B, halt
const program = new Uint8Array([
  0x06, 0x01,  // MVI B, 1
  0x3E, 0x02,  // MVI A, 2
  0x80,        // ADD B  → A = 3
  0x76,        // HLT
]);

const traces = cpu.run(program);
console.log(cpu.a);  // 3
console.log(traces[2].mnemonic);  // "ADD B"
console.log(traces[2].aBefore);   // 2
console.log(traces[2].aAfter);    // 3
```

## Public API

### `Intel8008GateLevel`

Drop-in replacement for `Intel8008Simulator` with an additional `gateCount()` method.

#### Register accessors

| Property    | Description                          |
|-------------|--------------------------------------|
| `a`         | Accumulator (8-bit, 0–255)           |
| `b`–`l`     | Registers B, C, D, E, H, L          |
| `pc`        | Program counter (14-bit, 0–16383)    |
| `hlAddress` | 14-bit H:L memory address pair       |
| `isHalted`  | True after HLT instruction           |
| `stack`     | All 8 stack entries (entry 0 = PC)   |
| `memory`    | 16 KiB memory array                  |

#### Methods

```typescript
cpu.run(program: Uint8Array, maxSteps?: number, startAddress?: number): Trace[]
cpu.step(): Trace
cpu.reset(): void
cpu.loadProgram(program: Uint8Array, startAddress?: number): void
cpu.setInputPort(port: number, value: number): void   // port 0–7
cpu.getOutputPort(port: number): number               // port 0–23
cpu.gateCount(): number                               // -1 = not yet instrumented
```

### `Trace` record

Each executed instruction produces a `Trace`:

```typescript
interface Trace {
  address: number;      // PC where instruction was fetched
  raw: Uint8Array;      // Raw bytes (1, 2, or 3)
  mnemonic: string;     // "ADD B", "MVI A, 0x05", "JMP 0x0100", etc.
  aBefore: number;      // Accumulator before execution
  aAfter: number;       // Accumulator after execution
  flagsBefore: Flags;   // { carry, zero, sign, parity }
  flagsAfter: Flags;
  memAddress: number | null;  // Address if M register was accessed
  memValue: number | null;    // Value at that address
}
```

## Sub-modules

All sub-modules are exported for educational use and direct testing:

```typescript
import {
  ProgramCounter,   // 14-bit PC with half-adder increment chain
  PushDownStack,    // 8-level push-down stack (D flip-flop registers)
  RegisterFile,     // 7-register file via D flip-flops
  FlagRegister,     // 4-bit flag register (CY, Z, S, P)
  GateALU8,         // 8-bit ALU via ripple-carry adder chain
  decode,           // Combinational opcode decoder (AND/OR/NOT gates)
  intToBits,        // Integer → bit array (LSB first)
  bitsToInt,        // Bit array → integer
  computeParity,    // Parity check via XOR reduction + NOT
} from "@coding-adventures/intel8008-gatelevel";
```

## Intel 8008 instruction set

### Instruction groups

| Group (bits[7:6]) | Instructions                              |
|-------------------|-------------------------------------------|
| `00`              | INR, DCR, MVI, Rotates, RST, RET, OUT    |
| `01`              | MOV, HLT, JMP, CAL, IN                   |
| `10`              | ALU register (ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP) |
| `11`              | ALU immediate (ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI) |

### Register encoding

```
000 = B    001 = C    010 = D    011 = E
100 = H    101 = L    110 = M*   111 = A
```

*M = memory at address `(H & 0x3F) << 8 | L`

### Notable encoding quirks

- `0x76` = HLT (not MOV M,M — intentional 8008 design)
- `0x7C` = JMP (unconditional jump)
- `0x7E` = CAL (unconditional call — NOT MOV A,M)
- `0xFF` = HLT (second halt encoding)

## Gate-level implementation details

### Program counter (pc.ts)

14-bit counter with increment via half-adder chain:

```
HA0:  sum[0]  = XOR(bit0, 1)      carry0 = AND(bit0, 1) = bit0
HA1:  sum[1]  = XOR(bit1, carry0)  carry1 = AND(bit1, carry0)
...
HA13: sum[13] = XOR(bit13, c12)    carry13 = discarded (wrap)
```

Gate count per increment: 14 HAs × 2 gates = **28 gate calls**.

### ALU (alu.ts)

8-bit ALU using the `arithmetic` package's `ALU(8)` class:
- Ripple-carry adder: 8 full-adders × 5 gates = **40 gate calls per add**
- Subtraction via two's complement: 8 NOT gates + ripple-carry = **48 gate calls**
- Bitwise AND/OR/XOR: 8 parallel gate calls each

### Registers (registers.ts)

Each 8-bit register write simulates a D flip-flop rising edge (clock 0→1):
- 8 flip-flops × (master latch call + slave latch call) = **16 dFlipFlop calls per write**
- Each `dFlipFlop` uses ~5 gate calls internally

### Stack (stack.ts)

8 × 14-bit D flip-flop registers:
- Each entry write: 14 flip-flops × rising edge = **28 dFlipFlop calls**
- Push: 8 entries rotated = up to 8 × 28 = **224 dFlipFlop calls**

### Decoder (decoder.ts)

Combinational gate tree: ~40 AND/OR/NOT gate calls per opcode decode.

## Running the tests

```bash
npm test           # Run tests
npm run test:coverage  # Run with coverage report
```

127 tests, 95.77% coverage.

## Historical context

The Intel 8008 was commissioned by Computer Terminal Corporation (CTC) for their Datapoint 2200 terminal. CTC rejected the chip as too slow. Intel sold it commercially in April 1972. The 8008 inspired the 8080 → Z80 → x86 lineage — this rejected terminal chip is the ancestor of essentially every personal computer processor ever made.

Key statistics:
- 3,500 PMOS transistors (vs 4004's 2,300)
- 500–800 kHz two-phase clock (vs 4004's 740 kHz)
- 8-bit data path (vs 4004's 4-bit)
- 16 KiB memory (vs 4004's 4 KiB)
- 7 general-purpose registers (vs 4004's accumulator-only)
- 8-level on-chip stack (vs 4004's no stack)
