# codegen-core

The universal IR-to-native compilation layer for all LANG pipeline paths (LANG19).

## What it does

`codegen-core` is the single shared package that defines what "code generation" means in this repository. Every compilation path — JIT, AOT, and compiled languages — passes through it:

```
JIT/AOT path:   list[CIRInstr]  → optimizer → Backend[CIR] → bytes
Compiled langs: IrProgram       → optimizer → Backend[Ir]  → bytes
```

## Key types

- **`CIRInstr`** — the typed instruction shared by jit-core and aot-core specialisation passes.
- **`Backend[IR]`** — universal structural protocol for any backend (generic over IR type).
- **`CodegenPipeline[IR]`** — composes optimizer + backend into a single `compile(ir) → bytes` call.
- **`CodegenResult[IR]`** — result of `compile_with_stats()`, with timing and IR snapshot.
- **`BackendRegistry`** — name-to-backend lookup map.

## Usage

```python
from codegen_core import CodegenPipeline, CIRInstr, CIROptimizer
from intel4004_backend import Intel4004Backend

pipeline = CodegenPipeline(
    backend=Intel4004Backend(),
    optimizer=CIROptimizer(),
)

binary = pipeline.compile(cir_list)
result = pipeline.compile_with_stats(cir_list)
```

## Package layout

```
codegen_core/
  cir.py           # CIRInstr
  backend.py       # Backend[IR], BackendProtocol (alias)
  pipeline.py      # CodegenPipeline[IR]
  result.py        # CodegenResult[IR]
  registry.py      # BackendRegistry
  optimizer/
    cir_optimizer.py   # constant fold + DCE for list[CIRInstr]
    ir_program.py      # wraps ir-optimizer for IrProgram
```

## Where it fits

```
jit-core  ──► codegen-core ──► intel4004-backend
aot-core  ──►               ──► wasm-backend
nib       ──►               ──► jvm-backend
```
