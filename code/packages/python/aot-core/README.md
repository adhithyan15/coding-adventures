# aot-core

Ahead-of-time compilation path for the LANG pipeline (LANG04).

`aot-core` compiles an entire `IIRModule` to a self-contained `.aot` binary
*before* any user runs the program.  Where `jit-core` (LANG03) compiles hot
functions at runtime after observing their behaviour, `aot-core` compiles
everything statically using type inference — no runtime feedback required.

## Architecture

```
IIRModule (bytecode)
    │
    ├── infer_types(fn)          → env: dict[str, str]
    │   (static Hindley-Milner-style, flow-insensitive)
    │
    ├── aot_specialise(fn, env)  → list[CIRInstr]
    │   (same structure as jit-core's specialise; uses inferred types)
    │
    ├── optimizer.run(cir)       → list[CIRInstr]
    │   (constant folding + DCE, reused from jit-core)
    │
    ├── backend.compile(cir)     → bytes | None
    │   (architecture-neutral BackendProtocol from jit-core)
    │
    ├── link(fn_binaries)        → (native_code, offsets)
    │
    └── snapshot.write(...)      → .aot binary
```

## The `.aot` binary format

```
Header (26 bytes):
  4 bytes  magic             "AOT\0"
  2 bytes  version           0x01 0x00
  4 bytes  flags             bit 0: IIR table present
  4 bytes  entry_point_offset
  4 bytes  vm_iir_table_offset
  4 bytes  vm_iir_table_size
  4 bytes  native_code_size

Code section:
  N bytes  native machine code (compiled functions concatenated)

IIR table section (optional):
  M bytes  JSON-encoded IIR for functions that could not be compiled
```

## Untyped functions and the vm-runtime

Functions that remain fully polymorphic (`"any"` type) after static inference
are placed in the **IIR table section** instead of the code section.  At
execution time a vm-runtime can interpret them from the IIR table.

```python
rt = VmRuntime(library_bytes=prebuilt_runtime_lib)
aot = AOTCore(backend=my_backend, vm_runtime=rt)
binary = aot.compile(module)
```

## Optimization levels

| Level | Passes applied                                     |
|-------|----------------------------------------------------|
| 0     | None (raw specialised CIR)                         |
| 1     | Constant folding + dead-code elimination            |
| 2     | Same as 1 (inlining + loop unrolling: future work) |

## Usage

```python
from aot_core import AOTCore, VmRuntime

aot = AOTCore(backend=my_backend, optimization_level=2)
binary = aot.compile(iir_module)

# Or write directly to disk:
aot.compile_to_file(iir_module, "program.aot")

# Inspect stats:
s = aot.stats()
print(s.functions_compiled, s.functions_untyped)
```

## Public surface

| Symbol          | Purpose                                           |
|-----------------|---------------------------------------------------|
| `AOTCore`       | Top-level AOT controller                          |
| `AOTStats`      | Compilation statistics snapshot                   |
| `AOTSnapshot`   | Parsed `.aot` binary contents                     |
| `VmRuntime`     | Pre-compiled vm-runtime library wrapper           |
| `infer_types`   | Static type inference (IIRFunction → dict)        |
| `aot_specialise`| AOT specialization pass (IIRFunction → CIRInstr)  |
| `link`          | Linker module (concatenate per-function binaries) |
| `snapshot`      | `.aot` binary writer/reader module                |

## Stack position

```
LANG00  interpreter-ir   ← shared IR types
LANG01  (reserved)
LANG02  vm-core          ← interpreter + profiler
LANG03  jit-core         ← JIT: runtime specialization
LANG04  aot-core         ← this package: AOT compilation
LANG05  backend-protocol ← backend interface specification
```
