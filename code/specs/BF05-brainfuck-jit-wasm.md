# BF05 — Brainfuck JIT via WASM (in-house runtime)

## Overview

BF04 wired Brainfuck through the LANG pipeline up to `vm-core`.  This spec
turns on the JIT half of the pipeline: hot Brainfuck functions are
specialised by `jit-core` (LANG03), lowered to the static `IrProgram`
(LANG21 — `cir-to-compiler-ir`), compiled to WebAssembly bytes by
`ir-to-wasm-compiler` (LANG20), and executed on the in-house
`wasm-runtime` (no wasmtime — we use the WASM stack already in this repo).

`BrainfuckVM(jit=True)` (which BF04 left raising `NotImplementedError`) is
now a real JIT.

## Scope of this PR

The Brainfuck IIR uses three "interesting" instructions: `load_mem`,
`store_mem`, and `call_builtin "putchar"` / `"getchar"`.  Of these,
`cir-to-compiler-ir` only knows how to lower control flow, arithmetic,
constants, comparisons, calls, and returns.  Memory and host-call
instructions are not yet lowered.

This PR adds:

1. **`load_mem` / `store_mem` lowerings** to `cir-to-compiler-ir`
   (mechanical — they map to existing `IrOp.LOAD_BYTE` / `STORE_BYTE`).
2. **`BrainfuckVM(jit=True)`** wired to `JITCore(vm, WASMBackend(),
   threshold_fully_typed=0)`.  Brainfuck functions are FULLY_TYPED, so
   they tier up on first call.

What this PR does **not** do:

- **`call_builtin "putchar"` / `"getchar"`** is intentionally left
  un-lowered.  When `WASMBackend.compile()` encounters it, the lowering
  raises `CIRLoweringError`; `WASMBackend.compile()` catches and returns
  `None`; `jit-core` marks the function unspecializable and falls back to
  the interpreter.  This is the documented deopt path — no behavioural
  change for the user, just a missed JIT opportunity.

  Wiring `call_builtin` to WASI `fd_write` / `fd_read` so I/O programs
  also JIT is **BF06** — a follow-up PR that touches `compiler-ir`
  (new high-level IrOp), `ir-to-wasm-compiler`, and `cir-to-compiler-ir`.

This means: programs without `.` / `,` JIT successfully on first call.
Programs with I/O run interpreted (correct, just no speedup).

## The lowering work

### `load_mem`

CIR shape (after `jit-core.specialise`):

```
CIRInstr(op="load_mem", dest="v", srcs=["ptr"], type="u8")
```

The static IR's `LOAD_BYTE` is a three-operand op:

```
LOAD_BYTE dst, base, offset   ; dst = mem[base + offset] & 0xFF
```

Brainfuck's tape lives at WASM linear-memory address 0, so the natural
`base` is a register holding 0.  We synthesise this by allocating a
fresh scratch register and loading the constant 0 into it, then emitting
the `LOAD_BYTE`:

```
LOAD_IMM  scratch, 0
LOAD_BYTE dst, scratch, ptr_reg
```

This is two extra instructions per memory access.  An IR optimisation
pass could fold the redundant zero-load across the function, but for V1
mechanical correctness wins over micro-optimisation.

### `store_mem`

CIR shape:

```
CIRInstr(op="store_mem", dest=None, srcs=["ptr", "v"], type="u8")
```

Lowering pattern, symmetric to `load_mem`:

```
LOAD_IMM   scratch, 0
STORE_BYTE val_reg, scratch, ptr_reg
```

### Type-suffix handling

`jit-core.specialise` puts memory ops in `_PASSTHROUGH_OPS` — they keep
their bare names (`load_mem`, `store_mem`) and carry the type in the
`CIRInstr.type` field rather than as an op suffix.  The lowering reads
`instr.type` to verify it is a known integer type.  Non-`u8` widths are
acceptable in the IR (`LOAD_BYTE` masks to byte regardless), so a
permissive check is fine.

## Wiring `BrainfuckVM(jit=True)`

The wrapper instantiates a `JITCore` with `WASMBackend()` and tier-up
threshold 0 (Brainfuck functions are FULLY_TYPED, so the JIT can
compile before the first interpreted call):

```python
from jit_core import JITCore
from wasm_backend import WASMBackend

self._jit = JITCore(
    vm,
    WASMBackend(),
    threshold_fully_typed=0,
    threshold_partial=10,
    threshold_untyped=100,
)
```

Programs whose `main` function compiles successfully run native code
and bypass the interpreter loop entirely.  Programs whose `main` cannot
be compiled (e.g. ones that include `call_builtin`) deopt and run
interpreted with no behavioural change.

`metrics.total_jit_hits > 0` on a successful JIT run — that is the
observable signal that the JIT path actually fired.

## Tests

- `compile_to_iir` then `JITCore.execute_with_jit` on an I/O-free
  program (e.g. `++++[->+++<]>`) succeeds and produces the same final
  cell state as the interpreted run.
- `BrainfuckVM(jit=True).run(...)` on a no-I/O program: returns the
  expected output bytes (empty for programs without `.`).
- `BrainfuckVM(jit=True).run(...)` on a program with `.`: deopts
  gracefully and produces the same byte stream as the interpreted run.
- New `cir-to-compiler-ir` tests for `load_mem` and `store_mem`
  coverage.

## Out of scope (BF06)

- WASI `fd_write` / `fd_read` lowering for `call_builtin "putchar"` /
  `"getchar"` so I/O programs JIT.
- Optimisation pass that hoists the synthesised zero-base register out
  of the inner loop.
- Multi-function JIT (Brainfuck only has one function — `main` —
  so this matters for languages that come later, not for Brainfuck).
