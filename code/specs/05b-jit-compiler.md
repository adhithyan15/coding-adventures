# 05b — JIT Compiler (Shell)

## Overview

A JIT (Just-In-Time) compiler watches bytecode being executed by the Virtual
Machine and, when it detects "hot" code (loops, frequently-called functions),
compiles that bytecode directly to native machine instructions at runtime.

This is a **future package** — the spec exists to document where it fits and
how it will work. No implementation yet.

## Where it fits

```
Source → Lexer → Parser → Bytecode Generator → VM ──→ [JIT Compiler] → Native code
                                                 │
                                                 └─→ (normal interpreted execution)
```

The VM executes bytecode normally (interpreted). The JIT watches. When a section
of bytecode runs many times (e.g., a loop body), the JIT compiles it to native
machine code (RISC-V, ARM, x86) so subsequent executions skip the VM's
fetch-decode-execute loop entirely.

## How it works (conceptual)

```
Step 1: VM interprets normally
    LOAD_CONST 0       ← VM dispatches via switch/case
    LOAD_NAME i
    COMPARE LT
    JUMP_IF_FALSE end
    ...

Step 2: JIT detects a hot loop (ran 100+ times)

Step 3: JIT compiles the loop body to native code
    Original bytecode:        JIT output (RISC-V):
    LOAD_NAME i               lw  a0, 0(sp)
    LOAD_CONST 1              addi a1, x0, 1
    ADD                       add  a0, a0, a1
    STORE_NAME i              sw   a0, 0(sp)

Step 4: Next time the VM reaches this loop, it jumps to native code instead

Step 5: If assumptions are violated (e.g., type changes), deoptimize —
        throw away native code, fall back to interpreter
```

## Key concepts (for future implementation)

- **Profiling**: Count how many times each bytecode offset is executed
- **Tiered compilation**: Interpret first, quick-compile after N executions, optimize after M
- **Type specialization**: If `a + b` always uses integers, emit integer ADD directly
- **Deoptimization**: Fall back to interpreter if assumptions break
- **On-stack replacement (OSR)**: Switch from interpreted to native mid-execution

## Relationship to other layers

- **Input**: Bytecode from the VM (the same instructions the VM interprets)
- **Output**: Native machine code for one of our ISA targets (RISC-V, ARM)
- **The VM** must be JIT-aware: it needs hooks to check "is there native code for this bytecode offset?"
- **The assembler** package can be reused to emit machine code

## Why this is hard (and why it's a shell for now)

JIT compilation is one of the most complex areas in computer science. Real JITs
(V8's TurboFan, HotSpot's C2, LuaJIT's trace compiler) are hundreds of thousands
of lines of code. The challenges include:

- Register allocation (mapping stack values to CPU registers)
- Instruction selection (choosing optimal native instructions)
- Garbage collection integration
- Debugging support (source maps from native code back to original source)
- Platform-specific code generation

We'll start simple when we implement this — a trace compiler that JIT-compiles
straight-line code without branching. Then add complexity incrementally.
