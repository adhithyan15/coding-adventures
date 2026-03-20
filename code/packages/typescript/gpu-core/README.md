# @coding-adventures/gpu-core

A generic, pluggable GPU processing element simulator -- the TypeScript port of the Python `gpu-core` package.

## What is this?

This package implements a single GPU core (Layer 9 of the accelerator computing stack). A GPU core is the smallest independently programmable compute unit on a GPU -- think of one CUDA core, one AMD stream processor, or one ARM Mali execution engine.

The core is **vendor-agnostic**: swap the `InstructionSet` to simulate any GPU architecture.

## Where it fits in the stack

```
Layer 8: Warp / SIMT Engine (schedules threads across cores)
            |
Layer 9: GPU Core  <-- THIS PACKAGE
            |
Layer 10: FP Arithmetic (IEEE 754 operations from logic gates)
```

## Quick Start

```typescript
import { GPUCore, GenericISA, limm, fmul, halt } from "@coding-adventures/gpu-core";

// Create a core with the default GenericISA
const core = new GPUCore({ isa: new GenericISA() });

// Load a program: compute 3.0 * 4.0
core.loadProgram([
  limm(0, 3.0),    // R0 = 3.0
  limm(1, 4.0),    // R1 = 4.0
  fmul(2, 0, 1),   // R2 = R0 * R1
  halt(),
]);

// Run and inspect
const traces = core.run();
console.log(core.registers.readFloat(2)); // 12.0
```

## Components

| Component | Description |
|-----------|-------------|
| `GPUCore` | The main processing element with fetch-execute loop |
| `GenericISA` | Default vendor-neutral instruction set (16 opcodes) |
| `FPRegisterFile` | Configurable FP register file (1-256 registers) |
| `LocalMemory` | Byte-addressable scratchpad with FP load/store |
| `Opcode` / `Instruction` | The 16-opcode instruction set vocabulary |
| `GPUCoreTrace` | Structured execution traces for debugging |

## The 16 Opcodes

| Category | Opcodes |
|----------|---------|
| Arithmetic | `FADD`, `FSUB`, `FMUL`, `FFMA`, `FNEG`, `FABS` |
| Memory | `LOAD`, `STORE` |
| Data movement | `MOV`, `LIMM` |
| Control flow | `BEQ`, `BLT`, `BNE`, `JMP`, `NOP`, `HALT` |

## Pluggable ISA Design

The core accepts any object satisfying the `InstructionSet` interface:

```typescript
import { InstructionSet, ExecuteResult } from "@coding-adventures/gpu-core";

class MyCustomISA implements InstructionSet {
  get name(): string { return "Custom"; }

  execute(instruction, registers, memory): ExecuteResult {
    // Your vendor-specific decode + execute logic here
  }
}

const core = new GPUCore({ isa: new MyCustomISA() });
```

## Running Tests

```bash
cd code/packages/typescript/gpu-core
npm install
npx vitest run
```

## Dependencies

- `@coding-adventures/fp-arithmetic` -- IEEE 754 floating-point arithmetic from logic gates
