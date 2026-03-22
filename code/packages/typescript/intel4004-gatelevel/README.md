# Intel 4004 Gate-Level Simulator (TypeScript)

A gate-level simulation of the Intel 4004, the world's first commercial microprocessor. Every computation routes through real logic gates -- NOT, AND, OR, XOR -- and state is stored in D flip-flop registers.

## Where it fits in the computing stack

```
Layer 4e: Intel 4004 Gate-Level  <-- this package
Layer 4d: Intel 4004 Behavioral Simulator
Layer 3:  CPU Simulator (generic)
Layer 2:  Arithmetic (half adder, full adder, ALU)
Layer 1:  Logic Gates (AND, OR, NOT, XOR, flip-flops)
```

This package depends on:
- `@coding-adventures/logic-gates` -- fundamental gates and sequential logic
- `@coding-adventures/arithmetic` -- adders and ALU

## What makes it "gate-level"?

When you execute `ADD R3`, the simulator:

1. Reads R3's value from 4 D flip-flops (via `register()`)
2. Reads the accumulator from 4 D flip-flops
3. Feeds both through the ALU, which uses `fullAdder()` chains built from `XOR`/`AND`/`OR` gates
4. Clocks the result back into the accumulator's flip-flops

No behavioral shortcuts. Every bit passes through gate functions.

## Gate count

| Component             | Gates | Transistors (x4) |
|-----------------------|-------|-------------------|
| ALU (4-bit)           | 32    | 128               |
| Register file (16x4)  | 480   | 1,920             |
| Accumulator (4-bit)   | 24    | 96                |
| Carry flag (1-bit)    | 6     | 24                |
| Program counter (12)  | 96    | 384               |
| Hardware stack (3x12) | 226   | 904               |
| Decoder               | ~50   | 200               |
| Control + wiring      | ~100  | 400               |
| **Total**             | **~1,014** | **~4,056**   |

## Usage

```typescript
import { Intel4004GateLevel } from "@coding-adventures/intel4004-gatelevel";

const cpu = new Intel4004GateLevel();

// x = 1 + 2, store in R1
const traces = cpu.run(new Uint8Array([
  0xD1,  // LDM 1
  0xB0,  // XCH R0
  0xD2,  // LDM 2
  0x80,  // ADD R0
  0xB1,  // XCH R1
  0x01,  // HLT
]));

console.log(cpu.registers[1]); // 3
console.log(cpu.gateCount());  // ~8,894
```

## Supported instructions (46 total)

- **Data**: NOP, HLT, LDM, LD, XCH, LDM
- **Arithmetic**: ADD, SUB, INC
- **Accumulator**: CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL
- **Jumps**: JUN, JCN, ISZ, JMS, BBL
- **Register pairs**: FIM, SRC, FIN, JIN
- **I/O**: WRM, WMP, WRR, WPM, WR0-WR3, SBM, RDM, RDR, ADM, RD0-RD3

## Testing

```bash
npx vitest run
```
