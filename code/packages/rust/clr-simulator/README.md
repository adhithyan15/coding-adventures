# clr-simulator

CLR bytecode simulator -- Microsoft's Common Language Runtime.

## What is this?

This crate simulates a subset of .NET CLR bytecode. Unlike the JVM (which encodes types in opcodes), the CLR infers types from the stack -- one `add` opcode works for int32, int64, and float.

## Supported Instructions

Includes ldc.i4 (compact and extended forms), ldloc/stloc, add, sub, mul, div, nop, ldnull, br.s, brfalse.s, brtrue.s, ret, and two-byte comparison opcodes (ceq, cgt, clt).

## Usage

```rust
use clr_simulator::*;

let mut sim = CLRSimulator::new();
let prog = assemble_clr(&[
    encode_ldc_i4(7),
    encode_ldc_i4(3),
    vec![OP_SUB],
    vec![OP_RET],
]);
sim.load(&prog, 16);
sim.run(100);
assert_eq!(sim.stack[0], Some(4));
```
