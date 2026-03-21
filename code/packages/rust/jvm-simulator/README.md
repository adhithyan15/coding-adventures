# jvm-simulator

JVM bytecode simulator -- a typed stack-based virtual machine.

## What is this?

This crate simulates a subset of Java Virtual Machine bytecode. The JVM uses typed opcodes (iadd for int, ladd for long) and local variable slots instead of named registers.

## Supported Instructions

Includes iconst_N, bipush, ldc, iload/istore (compact and extended forms), iadd, isub, imul, idiv, if_icmpeq, if_icmpgt, goto, ireturn, and return.

## Usage

```rust
use jvm_simulator::*;

let mut sim = JVMSimulator::new();
let prog = assemble_jvm(&[
    Instr { opcode: OP_ICONST_0 + 3, params: vec![] },
    Instr { opcode: OP_ICONST_0 + 4, params: vec![] },
    Instr { opcode: OP_IMUL, params: vec![] },
    Instr { opcode: OP_IRETURN, params: vec![] },
]);
sim.load(&prog, &[], 16);
sim.run(100);
assert_eq!(sim.return_value, Some(12));
```
