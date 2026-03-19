# CPU Simulator

**Layer 3 of the computing stack** -- models the core of a CPU.

## What this package does

Models the essential components of a CPU core:

| Component | Description |
|-----------|-------------|
| Registers | General-purpose storage inside the CPU |
| Memory | Addressable byte-array for instructions and data |
| Program Counter | Tracks the address of the next instruction |
| Fetch-Decode-Execute | The core cycle that drives instruction processing |

## Where it fits

```
Logic Gates -> Arithmetic -> [CPU Simulator] -> ARM -> Assembler -> Lexer -> Parser -> Compiler -> VM
```

This package is used by the **arm-simulator** package to model a complete ARM-based processor.

## Installation

```bash
npm install @coding-adventures/cpu-simulator
```

## Usage

```typescript
import { CPU, Memory, RegisterFile, formatPipeline } from "@coding-adventures/cpu-simulator";
import type { InstructionDecoder, InstructionExecutor, DecodeResult, ExecuteResult } from "@coding-adventures/cpu-simulator";

// Implement your own ISA decoder and executor
class MyDecoder implements InstructionDecoder {
  decode(rawInstruction: number, pc: number): DecodeResult {
    // ... decode instruction bits
  }
}

class MyExecutor implements InstructionExecutor {
  execute(decoded: DecodeResult, registers: RegisterFile, memory: Memory, pc: number): ExecuteResult {
    // ... execute the decoded instruction
  }
}

// Create a CPU with your ISA
const cpu = new CPU(new MyDecoder(), new MyExecutor(), 16, 32);

// Load and run a program
cpu.loadProgram([0x93, 0x00, 0x10, 0x00]);
const traces = cpu.run();

// Visualize the pipeline
for (const trace of traces) {
  console.log(formatPipeline(trace));
}
```

## Spec

See [08-cpu-simulator.md](../../../specs/08-cpu-simulator.md) for the full specification.
