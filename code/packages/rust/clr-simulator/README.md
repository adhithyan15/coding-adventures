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

Since 0.3.0 it runs McCarthy's **predicates** (W7): `isinst` (the `pair?` type
test — keep a heap ref, else `null`), `xor` (logical `not` = `x^1`), and
**reference-aware** `ceq`/`cgt`/`clt` (so `pair?`/`is_null` can compare a
reference against `ldnull`). This release also fixes `ldnull` to its real CIL
opcode `0x14` (was `0x01`), a latent bug the cons path never exercised.

Since 0.4.0 it runs McCarthy **lambda** (W8b) via an inter-method **call-frame**
model: `load_program(methods, entry)` registers a method table, `call
<MethodDef>` pops the callee's args + pushes a frame + transfers control, `ret`
pops the frame (or halts at the entry), and `ldarg.N` reads a parameter.
Recursion depth is DoS-capped at `MAX_CALL_DEPTH`. The operand stack + heap are
shared across frames; single-method programs still use `load` unchanged.

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
