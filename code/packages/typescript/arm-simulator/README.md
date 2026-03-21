# ARM Simulator

**Layer 4 of the computing stack** -- implements an ARMv7 instruction subset decoder and executor.

## What this package does

Decodes and executes a subset of the ARMv7 instruction set, bridging the gap between raw CPU operations and assembly language:

- **Instruction decoding** -- extracts condition codes, opcodes, register numbers, and immediates from 32-bit ARM instructions
- **Instruction execution** -- MOV, ADD, SUB with register and immediate operands
- **Assembler helpers** -- encode ARM instructions from human-readable parameters
- **Pipeline visibility** -- full fetch-decode-execute trace for every instruction

## Where it fits

```
Logic Gates -> Arithmetic -> CPU -> [ARM Simulator] -> Assembler -> Lexer -> Parser -> Compiler -> VM
```

This package plugs into the CPU simulator via the `InstructionDecoder` / `InstructionExecutor` protocol.

## Installation

```bash
npm install @coding-adventures/arm-simulator
```

## Usage

```typescript
import {
  ARMSimulator,
  assemble,
  encodeMovImm,
  encodeAdd,
  encodeHlt,
} from "@coding-adventures/arm-simulator";

// x = 1 + 2
const sim = new ARMSimulator();
const program = assemble([
  encodeMovImm(0, 1),   // MOV R0, #1
  encodeMovImm(1, 2),   // MOV R1, #2
  encodeAdd(2, 0, 1),   // ADD R2, R0, R1
  encodeHlt(),           // HLT
]);

const traces = sim.run(program);
console.log(sim.cpu.registers.read(2)); // 3
```

## Supported instructions

| Instruction | Encoding | Description |
|-------------|----------|-------------|
| MOV Rd, #imm | Data processing, I=1 | Load immediate into register |
| ADD Rd, Rn, Rm | Data processing, I=0 | Add two registers |
| SUB Rd, Rn, Rm | Data processing, I=0 | Subtract two registers |
| HLT | 0xFFFFFFFF sentinel | Halt execution |

## Spec

See [07b-arm-simulator.md](../../../specs/07b-arm-simulator.md) for the full specification.
