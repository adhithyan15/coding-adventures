# clr-simulator

CLR bytecode simulator -- Microsoft's Common Language Runtime.

## What is this?

This crate simulates a subset of .NET CLR bytecode. Unlike the JVM (which encodes types in opcodes), the CLR infers types from the stack -- one `add` opcode works for int32, int64, and float.

## Supported Instructions

Includes ldc.i4 (compact -1 through 8, short and full forms), ldc.i8, ldloc/stloc, add, sub, mul,
div, rem, and/or/xor/not, shifts, neg, nop, ldnull, br.s, brfalse.s, brtrue.s,
ret, and two-byte comparison opcodes (ceq, cgt, clt).

`Int64(i64)` preserves a separate 64-bit stack type. Integer arithmetic and
comparisons require matching widths; array sizes and indices remain int32.
Division rejects zero and signed overflow. This subset does not implement
floating-point arithmetic, host input, or full CLR boxing/type verification.

Since 0.2.0 it also executes **reference types**: a stack/local slot is a
`Value` (`Int(i32)`, `Int64(i64)` or `Ref(Option<usize>)` into an object heap), and the
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

### Call token tables

Internal calls accept MethodDef tokens (`0x06` table) with valid one-based
method ordinals. Other tables, including MemberRef (`0x0A`), panic with an
explicit unsupported-table diagnostic before consuming arguments or changing
call frames. A MemberRef row cannot alias the internal method at the same row.
This follows the simulator's existing invalid-bytecode panic convention;
host-call resolution and input readers are not implemented.

### Explicit integer conversions (CLR03)

Explicit conv.i4 truncates Int64 to its signed low 32 bits; conv.i8 sign-extends
Int to Int64. Same-width conversions preserve values. Missing/uninitialized
operands and references refuse before stack or pc mutation. Mixed-width
arithmetic still requires explicit conversion. IIR lowering remains unchanged.

Signed shifts (shl/shr) preserve Int or Int64 width and require an Int count.
Only counts from zero through width minus one are supported; invalid operands
and counts refuse before changing stack or pc. This does not widen IIR lowering.

Bitwise and/or execute on matching Int or Int64 operands and preserve all bits.
They do not coerce values to booleans. Mixed widths, references and missing
operands refuse before changing stack or pc.

Signed remainder and bitwise NOT also preserve matched Int/Int64 widths.
Remainder follows truncation-toward-zero division, rejects zero and MIN/-1,
and keeps the dividend's sign. Invalid operands refuse without state mutation.
