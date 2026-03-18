# JIT Compiler

**Layer 5b of the computing stack** — compiles hot bytecode to native machine code at runtime.

**Status: Shell package — not yet implemented.**

## What is a JIT compiler?

When the Virtual Machine (Layer 5) executes bytecode, it interprets each instruction one at a time — reading the opcode, dispatching to a handler, performing the operation. This is flexible but slow, because every instruction goes through the VM's switch/case dispatch.

A JIT (Just-In-Time) compiler watches the VM execute. When it notices a piece of code running many times (a "hot path" — typically a loop), it compiles that bytecode directly into native machine instructions (RISC-V, ARM, or x86). The next time that code runs, the VM skips interpretation entirely and runs the native code directly.

```
Without JIT (interpreted):
    Each iteration: read opcode → switch/case → execute → read next opcode → ...
    Overhead: ~10-50 CPU instructions per bytecode instruction

With JIT (compiled):
    First 100 iterations: interpreted (VM collects profiling info)
    JIT compiles the loop to native code
    Remaining iterations: raw native instructions, no VM overhead
    Overhead: ~1 CPU instruction per bytecode instruction (10-50x faster)
```

## Where it fits

```
Source → Lexer → Parser → Bytecode Generator → VM ──→ [JIT Compiler] → Native code
```

The JIT sits alongside the VM. The VM runs normally until the JIT takes over hot paths.

## Why it's a shell

JIT compilation is one of the most complex areas in computer science. We'll implement it after the core stack (lexer → parser → VM) is working. See [05b-jit-compiler.md](../../../specs/05b-jit-compiler.md) for the design.

## Famous JITs

- **V8 (JavaScript)**: TurboFan optimizing compiler — makes JS competitive with C++
- **HotSpot (Java)**: C1 quick compile + C2 optimizing compile — why Java is fast
- **LuaJIT**: Trace compiler — makes Lua competitive with C in some benchmarks
- **PyPy (Python)**: Meta-tracing JIT — often 5-10x faster than CPython
