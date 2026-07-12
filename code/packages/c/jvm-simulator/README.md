# jvm-simulator (C)

A typed stack-based **JVM virtual machine**, in pure ISO C17 — a faithful port
of the Rust `jvm-simulator` crate.

## What it is

The JVM is a *stack machine* like WASM, but a **typed** one: the operand type
lives in the opcode (`iadd` / `ladd` / `fadd` / `dadd`), so a verifier can prove
type safety at class-load time. This VM models the **int subset**: locals are
numbered slots (with compact opcodes for slots 0-3), arithmetic wraps modulo
2³², and branches carry a 16-bit signed offset.

Supported opcodes: `iconst_0..5`, `bipush`, `ldc` (constant pool), `iload` /
`iload_0..3`, `istore` / `istore_0..3`, `iadd` / `isub` / `imul` / `idiv`,
`if_icmpeq` / `if_icmpgt` / `goto`, `ireturn` / `return`.

## API

```c
#include "jvm_simulator.h"

JvmSimulator *s = jvm_sim_new();               /* 16 locals by default */

/* Assemble: x = 1 + 2; return x  */
JvmProgram p; jvm_program_init(&p);
jvm_emit(&p, JVM_OP_ICONST_0 + 1, 0);          /* iconst_1 */
jvm_emit(&p, JVM_OP_ICONST_0 + 2, 0);          /* iconst_2 */
jvm_emit(&p, JVM_OP_IADD, 0);
jvm_emit(&p, JVM_OP_ISTORE_0, 0);
jvm_emit(&p, JVM_OP_ILOAD_0, 0);
jvm_emit(&p, JVM_OP_IRETURN, 0);

size_t len; const uint8_t *bytes = jvm_program_bytes(&p, &len);
jvm_sim_load(s, bytes, len, NULL, 0, 16);

JvmTrace *traces = NULL; size_t count = 0;
jvm_sim_run(s, 100, &traces, &count);          /* one trace per instruction */

int32_t rv;
if (jvm_sim_return_value(s, &rv)) { /* rv == 3 */ }

jvm_traces_free(traces, count);
jvm_program_free(&p);
jvm_sim_free(s);
```

`jvm_emit` appends an opcode plus its operands: a 1-byte operand for
`bipush`/`iload`/`istore`/`ldc`, a 2-byte big-endian offset for
`goto`/`if_icmp*`, nothing for operand-less opcodes.

## Divergence from the Rust crate

Where the Rust **panics** — stepping a halted VM, PC past the end, an unknown or
truncated opcode, stack underflow, a constant-pool index out of range, division
by zero, or an uninitialized / out-of-range local — this port returns a
`JvmStatus` code instead. `idiv` of `INT32_MIN / -1` (undefined in C) is
special-cased to wrap to `INT32_MIN`, matching Rust's `wrapping_div`.

## Building

```sh
sh BUILD             # Unix: builds & runs the tests under every C compiler present
```

Pure ISO C17 — no compiler extensions. Builds clean under GCC, Clang, and MSVC
with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Where it fits

Part of the C/C++ port campaign mirroring the Rust learning packages. A sibling
of [`wasm-simulator`](../wasm-simulator) (an untyped stack machine); together
they contrast the two dominant bytecode designs.
