# clr-simulator

CLR bytecode simulator -- Microsoft's Common Language Runtime.

## What is this?

This crate simulates a subset of .NET CLR bytecode. Unlike the JVM (which encodes types in opcodes), the CLR infers types from the stack -- one `add` opcode works for int32, int64, and float.

## Supported Instructions

Includes ldc.i4 (compact and extended forms), ldloc/stloc, add, sub, mul, div, nop, ldnull, br.s, brfalse.s, brtrue.s, ret, and two-byte comparison opcodes (ceq, cgt, clt).

Since 0.2.0 it also executes **reference types**: a stack/local slot is a
`Value` (`Int(i32)` or `Ref(Option<usize>)` into an object heap), and the
reference opcodes `newarr`, `stelem.ref`, `ldelem.ref`, `dup`, and identity
`box`/`unbox.any` run — enough to execute the `System.Object[]` cons cells the
IIR→CIL backend emits for McCarthy Lisp (LANG77 / W6b).

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
