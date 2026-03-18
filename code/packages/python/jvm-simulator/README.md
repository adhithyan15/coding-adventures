# JVM Simulator

**Layer 4e of the computing stack** -- simulates the Java Virtual Machine bytecode instruction set.

## What this package does

Simulates a minimal Java Virtual Machine (JVM) bytecode interpreter. The JVM was introduced by Sun Microsystems in 1995 with the promise of "write once, run anywhere" -- compile your Java source code to platform-independent bytecode, and any machine with a JVM can execute it. Today, the JVM runs not just Java but also Kotlin, Scala, Clojure, and Groovy, making it the most widely deployed virtual machine in history.

This simulator implements:

- **Stack-based execution** with typed opcodes (`iadd`, `isub`, `imul`, `idiv`)
- **Local variable slots** numbered 0-15 (like registers, but accessed through the stack)
- **Variable-width bytecode encoding** matching real JVM opcode values
- **Constant pool** for loading values via `ldc`
- **Control flow** with `goto`, `if_icmpeq`, and `if_icmpgt`
- **Step tracing** showing PC, stack state, and locals at every instruction

## How it compares to other simulators in this project

| Feature | Our VM (cpu-simulator) | WASM | JVM |
|---------|----------------------|------|-----|
| Types | Untyped stack | Typed (i32/i64/f32/f64) | Typed opcodes (i/l/f/d prefix) |
| Variables | Named (hash map) | Numbered slots | Numbered slots |
| Constants | Index into pool | `i32.const` immediate | `iconst_N` shortcuts + pool |
| Encoding | Each instruction is an object | Variable-width bytes | Variable-width bytes |
| Methods | Simple CALL/RETURN | Function index | Full method descriptors |

## Where it fits

```
Logic Gates -> Arithmetic -> CPU -> [JVM Simulator] -> Assembler -> Lexer -> Parser -> Compiler -> VM
```

This package sits alongside the WASM, ARM, and RISC-V simulators as another execution target -- demonstrating how the same computation (e.g., `x = 1 + 2`) looks when compiled to different instruction sets.

## Installation

```bash
uv add coding-adventures-jvm-simulator
```

## Usage

```python
from jvm_simulator import JVMSimulator, assemble_jvm, JVMOpcode

# x = 1 + 2
sim = JVMSimulator()
program = assemble_jvm(
    (JVMOpcode.ICONST_1,),       # push 1
    (JVMOpcode.ICONST_2,),       # push 2
    (JVMOpcode.IADD,),           # pop 2 and 1, push 3
    (JVMOpcode.ISTORE_0,),       # pop 3, store in local 0
    (JVMOpcode.RETURN,),         # halt
)
sim.load(program)
traces = sim.run()
print(sim.locals[0])  # 3
```

## Spec

See [04e-jvm-simulator.md](../../../specs/04e-jvm-simulator.md) for the full specification.
