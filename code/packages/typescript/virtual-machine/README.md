# Virtual Machine

**Layer 5 of the computing stack** — a general-purpose stack-based bytecode interpreter.

## What this package does

Implements the eval loop that executes compiled bytecode programs. This is what CPython, YARV, and the JVM do:

| Component | Description |
|-----------|-------------|
| Eval Loop | Fetch-decode-execute cycle over bytecode instructions |
| Stack | Operand stack for intermediate values |
| Frames | Call frames for function invocation |
| Globals | Global variable storage |
| Locals | Indexed local variable slots |

## Where it fits

```
Logic Gates -> Arithmetic -> CPU -> ARM -> Assembler -> Lexer -> Parser -> Compiler -> [VM]
```

This package is the **top layer** that ties everything together, consuming bytecode produced by the **bytecode-compiler** package.

## Installation

```bash
npm install @coding-adventures/virtual-machine
```

## Usage

```typescript
import { OpCode, VirtualMachine, assembleCode } from "@coding-adventures/virtual-machine";

// Assemble a simple program: print(1 + 2)
const code = assembleCode(
  [
    { opcode: OpCode.LOAD_CONST, operand: 0 },
    { opcode: OpCode.LOAD_CONST, operand: 1 },
    { opcode: OpCode.ADD },
    { opcode: OpCode.PRINT },
    { opcode: OpCode.HALT },
  ],
  [1, 2],
);

const vm = new VirtualMachine();
const traces = vm.execute(code);
console.log(vm.output); // ["3"]
```

## Instruction Set

| Category | Opcodes | Description |
|----------|---------|-------------|
| Stack | LOAD_CONST, POP, DUP | Push/pop/duplicate values |
| Variables | STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL | Named and indexed variable access |
| Arithmetic | ADD, SUB, MUL, DIV | Integer math and string concatenation |
| Comparison | CMP_EQ, CMP_LT, CMP_GT | Relational operators (push 1/0) |
| Control Flow | JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE | Unconditional and conditional branches |
| Functions | CALL, RETURN | Function invocation via call stack |
| I/O | PRINT | Output capture for testing |
| VM Control | HALT | Stop execution |

## Spec

See [05-virtual-machine.md](../../../specs/05-virtual-machine.md) for the full specification.
