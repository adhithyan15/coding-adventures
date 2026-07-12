# jvm-simulator (C++)

A typed stack-based **JVM virtual machine**, header-only in pure ISO C++17
(namespace `ca::jvm`) — a faithful port of the Rust `jvm-simulator` crate.

## What it is

The JVM is a *stack machine* like WASM, but a **typed** one: the operand type
lives in the opcode (`iadd` / `ladd` / `fadd` / `dadd`), so a verifier can prove
type safety at class-load time. This VM models the **int subset**: locals are
numbered slots (`std::optional`, so a slot may be uninitialized), arithmetic
wraps modulo 2³², and branches carry a 16-bit signed offset.

Supported opcodes: `iconst_0..5`, `bipush`, `ldc` (constant pool), `iload` /
`iload_0..3`, `istore` / `istore_0..3`, `iadd` / `isub` / `imul` / `idiv`,
`if_icmpeq` / `if_icmpgt` / `goto`, `ireturn` / `return`.

## Usage

```cpp
#include "jvm_simulator.hpp"
namespace j = ca::jvm;

j::JVMSimulator sim;                     // 16 uninitialized locals by default

// x = 1 + 2; return x
auto prog = j::assemble_jvm({
    {j::OP_ICONST_0 + 1, {}},            // iconst_1
    {j::OP_ICONST_0 + 2, {}},            // iconst_2
    {j::OP_IADD, {}},
    {j::OP_ISTORE_0, {}},
    {j::OP_ILOAD_0, {}},
    {j::OP_IRETURN, {}},
});
sim.load(prog, /*constants=*/{}, /*num_locals=*/16);
auto traces = sim.run(100);              // one JVMTrace per instruction

if (sim.return_value) { /* *sim.return_value == 3 */ }
```

`assemble_jvm` takes `Instr{opcode, params}`; the convenience `encode_iconst` /
`encode_iload` / `encode_istore` helpers pick the compact opcode form for small
values / slots 0-3.

## Divergence from the Rust crate

Where the Rust **panics** — stepping a halted VM, PC past the end, an unknown or
truncated opcode, stack underflow, a constant-pool index out of range, division
by zero, or an uninitialized / out-of-range local — this port **throws**
(`std::runtime_error` / `std::out_of_range`). `idiv` of `INT32_MIN / -1`
(undefined in C++) is special-cased to `INT32_MIN`, matching Rust's
`wrapping_div`.

## Building

```sh
sh BUILD             # Unix: builds & runs the tests under every C++ compiler present
```

Pure ISO C++17 — standard library only, no compiler extensions. Builds clean
under GCC, Clang, and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors, via the shared [`iso-harness`](../../c/iso-harness).

## Where it fits

Part of the C/C++ port campaign mirroring the Rust learning packages, and a
sibling of [`wasm-simulator`](../wasm-simulator) — together they contrast the
typed (JVM) and untyped (WASM) bytecode designs.
