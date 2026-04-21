# tetrad-vm

Tetrad bytecode interpreter — executes `CodeObject` programs emitted by
[tetrad-compiler](../tetrad-compiler) using the generic register-based VM
chassis from [register-vm](../register-vm).

## Architecture

```
tetrad source
    │
    ▼
tetrad-compiler  →  CodeObject (instructions, functions, feedback_slot_count)
    │
    ▼
TetradVM  →  registers 39 opcode handlers on GenericRegisterVM
    │           (GenericRegisterVM lives in register-vm package)
    │
    ▼
GenericRegisterVM  →  fetch-decode-execute loop, trace hook, frame management
```

`TetradVM` is a **thin language backend** — it registers one handler per Tetrad
opcode and exposes the metrics API.  The execution chassis (dispatch loop,
recursive call frames, per-instruction tracing) lives in `GenericRegisterVM`
and is shared by every language in this repo.

## Why GenericRegisterVM?

Building a dedicated dispatch loop for every language produces 50 slightly
different VMs that each need their own debugger, profiler, and tracing hooks.
The generic approach:

- **One debugger** — the `trace_builder` hook works for any language backend.
- **Thin backends** — TetradVM is handler registration + metrics API, ~700 lines.
- **Shared regression coverage** — bugs in the dispatch loop are fixed once.

## Installation

```bash
pip install coding-adventures-tetrad-vm
```

## Quick start

```python
from tetrad_compiler import compile_program
from tetrad_vm import TetradVM

code = compile_program("""
    fn add(a: u8, b: u8) -> u8 { return a + b; }
    let result = add(10, 20);
""")

vm = TetradVM()
vm.execute(code)
print(vm._globals["result"])   # 30
```

## Tracing

```python
result, trace = vm.execute_traced(code)
for step in trace:
    print(f"[{step.fn_name}] ip={step.ip:2d}  acc: {step.acc_before!r} → {step.acc_after!r}")
```

## Metrics API

```python
# Which functions ran > 100 times?
vm.hot_functions(threshold=100)

# Type profile at a feedback slot
slot = vm.type_profile("add", slot=0)   # SlotState(kind=MONOMORPHIC, ...)

# Branch statistics
stats = vm.branch_profile("loop_fn", slot=3)  # BranchStats(taken=10, not_taken=0)

# Loop iteration counts
loops = vm.loop_iterations("loop_fn")   # {6: 100}  → JMP_LOOP at ip=6 ran 100×

# Immediate-JIT queue (fully-typed functions eligible for AOT)
vm.metrics().immediate_jit_queue   # ["add", "double"]

# Reset everything
vm.reset_metrics()
```

## u8 Semantics

All arithmetic is taken mod 256.  Values wrap — there are no overflow errors.
This mirrors the Intel 4004's 4-bit accumulator extended to 8 bits.

## Call Stack Limit

Maximum 4 frames (depth 0–3), matching the 4004's 4-level call stack.
Recursive calls beyond depth 3 raise `VMError("call stack overflow")`.

## I/O

```python
vm = TetradVM(
    io_in=lambda: int(input("? ")),
    io_out=lambda v: print(f"→ {v}"),
)
```

## Stack

| Layer | Package |
|-------|---------|
| Source | `*.tet` files |
| Lexer | `tetrad-lexer` |
| Parser | `tetrad-parser` |
| Type checker | `tetrad-type-checker` |
| Compiler | `tetrad-compiler` |
| **VM** | **`tetrad-vm`** ← you are here |
| JIT | `tetrad-jit` (spec TET05) |
| AOT | `tetrad-aot` (spec TET06) |
