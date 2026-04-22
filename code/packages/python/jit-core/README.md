# jit-core

Generic JIT specialization engine for the LANG pipeline (LANG03).

`jit-core` sits between `vm-core` (the interpreter) and a native backend.
It monitors execution, detects hot functions, translates typed
`IIRFunction` objects into a flat `list[CIRInstr]`, optimizes them, and
hands the result to a pluggable backend for native compilation.

## Architecture

```
IIRFunction  (with profiler feedback from vm-core)
    │
    ▼  specialise()
list[CIRInstr]    ← typed CompilerIR with guards
    │
    ▼  optimizer.run()
list[CIRInstr]    ← constant-folded, DCE'd
    │
    ▼  backend.compile()
bytes             ← native binary (opaque to jit-core)
    │
    ▼  backend.run()
return value      ← registered as JIT handler in VMCore
```

## Compilation tiers

| Type status      | Threshold (default) | Compiled when              |
|------------------|--------------------:|----------------------------|
| `FULLY_TYPED`    | 0                   | Before first interpreted call |
| `PARTIALLY_TYPED`| 10                  | After 10 interpreted calls  |
| `UNTYPED`        | 100                 | After 100 interpreted calls |

## Deoptimization

When a compiled function's deopt rate exceeds 10 %, `JITCore` invalidates
the cached binary and marks the function unspecializable — it runs
interpreted forever after.

## Usage

```python
from vm_core import VMCore
from jit_core import JITCore

vm = VMCore()
jit = JITCore(vm=vm, backend=my_backend)

result = jit.execute_with_jit(module, fn="main", args=[1, 2, 3])
```

## Public surface

| Symbol              | Purpose                                     |
|---------------------|---------------------------------------------|
| `JITCore`           | Top-level JIT controller                    |
| `CIRInstr`          | Typed CompilerIR instruction                |
| `JITCache`          | Compiled-function cache                     |
| `JITCacheEntry`     | Per-function cache entry with stats         |
| `BackendProtocol`   | Structural protocol for JIT backends        |
| `specialise`        | IIRFunction → list[CIRInstr]               |
| `optimizer`         | Constant-folding + DCE optimizer module     |
| `JITError`          | Base exception                              |
| `UnspecializableError` | Raised when a function cannot be JIT'd  |
| `DeoptimizerError`  | Raised on deoptimizer failures              |

## Stack position

```
LANG00  interpreter-ir   ← shared IR types
LANG01  (reserved)
LANG02  vm-core          ← interpreter + profiler
LANG03  jit-core         ← this package
LANG04  …
```
