# @coding-adventures/jvm-simulator

Java Virtual Machine bytecode simulator -- Layer 4e of the computing stack.

## What is this?

A TypeScript implementation of a JVM bytecode simulator. The JVM is a stack-based virtual machine with typed opcodes that runs Java, Kotlin, Scala, and other JVM languages. This simulator executes a subset of real JVM integer bytecodes.

## Supported Instructions

| Instruction | Encoding | Description |
|------------|----------|-------------|
| `iconst_0..5` | 1 byte | Push small integer constants |
| `bipush V` | 2 bytes | Push signed byte value |
| `ldc #N` | 2 bytes | Load from constant pool |
| `iload N` | 1-2 bytes | Load local variable |
| `istore N` | 1-2 bytes | Store to local variable |
| `iadd/isub/imul/idiv` | 1 byte | Integer arithmetic |
| `goto OFF` | 3 bytes | Unconditional branch |
| `if_icmpeq/gt OFF` | 3 bytes | Conditional branch |
| `ireturn/return` | 1 byte | Return from method |

## Usage

```typescript
import { JVMSimulator, JVMOpcode, assembleJvm } from "@coding-adventures/jvm-simulator";

const sim = new JVMSimulator();
sim.load(assembleJvm(
  [JVMOpcode.ICONST_1],
  [JVMOpcode.ICONST_2],
  [JVMOpcode.IADD],
  [JVMOpcode.ISTORE_0],
  [JVMOpcode.RETURN],
));
const traces = sim.run();
console.log(sim.locals[0]); // => 3
```

## How it fits in the stack

This is a TypeScript port of the Python jvm-simulator package. It sits at Layer 4e, demonstrating JVM's typed stack machine with variable-width bytecode.
