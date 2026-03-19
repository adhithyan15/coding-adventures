# RISC-V Simulator

**Layer 4b of the computing stack** (alternative to ARM) -- implements a minimal RISC-V RV32I instruction subset decoder and executor.

## What this package does

Decodes and executes a minimal subset of the RISC-V RV32I instruction set. RISC-V offers a cleaner encoding than ARM and is a fully open standard:

- Instruction decoding (just 3 instructions to start: `addi`, `add`, `ecall`)
- Register file management (x0-x31)
- Clean, regular instruction encoding
- Memory-mapped I/O simulation

## Where it fits

```
Logic Gates -> Arithmetic -> CPU -> [RISC-V Simulator] -> Assembler -> Lexer -> Parser -> Compiler -> VM
```

This package is used by the **assembler** package to execute assembled RISC-V instructions.

## Installation

```bash
npm install @coding-adventures/riscv-simulator
```

## Usage

```typescript
import {
  RiscVSimulator,
  assemble,
  encodeAddi,
  encodeAdd,
  encodeEcall,
} from "@coding-adventures/riscv-simulator";

// x = 1 + 2
const sim = new RiscVSimulator();
const program = assemble([
  encodeAddi(1, 0, 1),  // x1 = 1
  encodeAddi(2, 0, 2),  // x2 = 2
  encodeAdd(3, 1, 2),   // x3 = x1 + x2 = 3
  encodeEcall(),         // halt
]);
const traces = sim.run(program);
console.log(sim.cpu.registers.read(3)); // 3
```

## Spec

See [07a-riscv-simulator.md](../../../specs/07a-riscv-simulator.md) for the full specification.
