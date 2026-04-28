# LANG19 — codegen-core: Unified IR-to-Native Compilation Layer

## Overview

Before LANG19, the repository had two completely parallel pipelines for
lowering typed IR to native binary:

**JIT/AOT path** (interpreted languages)

```
IIRModule → specialise() → list[CIRInstr]
  → jit_core.optimizer.run()   (constant fold + DCE)
  → backend.compile(CIR)       [intel4004-backend, ...]
  → bytes
```

**Compiled-language path** (Nib, Brainfuck, Algol-60)

```
Source → frontend → IrProgram
  → ir-optimizer.optimize()    (DCE + constant fold + peephole)
  → backend.compile(IrProgram) [ir-to-wasm-compiler, ir-to-jvm-compiler, ...]
  → bytes
```

Both pipelines do the same thing: take a typed IR, optionally optimize it,
dispatch to a backend, and get native bytes.  They shared no code.

There was also a **backwards dependency**: `aot-core` depended on `jit-core`
only to import `optimizer.run()` and `CIRInstr`.  Ahead-of-time compilation
should not depend on just-in-time infrastructure.

LANG19 introduces `codegen-core` — the single shared layer that defines what
code generation means in this repository.

---

## Architecture

### Generic `CodegenPipeline[IR]`

```python
class CodegenPipeline(Generic[IR]):
    def __init__(self, backend: Backend[IR], optimizer: Optimizer[IR] | None = None)
    def compile(self, ir: IR) -> bytes | None
    def compile_with_stats(self, ir: IR) -> CodegenResult[IR]
    def run(self, binary: bytes, args: list) -> Any
```

Two concrete instantiations:

1. **`CodegenPipeline[list[CIRInstr]]`** — JIT/AOT path
   - Optimizer: `CIROptimizer` (constant fold + DCE, moved from jit-core)
   - Backend: any `Backend[list[CIRInstr]]` (intel4004-backend, …)
   - Used by both `JITCore` and `AOTCore`

2. **`CodegenPipeline[IrProgram]`** — compiled-language path
   - Optimizer: `IrProgramOptimizer` (wraps `IrOptimizer` from ir-optimizer)
   - Backend: any `Backend[IrProgram]` (ir-to-wasm-compiler, …)
   - Available for compiled-language compilers to adopt

### Generic `Backend[IR]` protocol

```python
class Backend(Protocol[IR]):
    name: str
    def compile(self, ir: IR) -> bytes | None: ...
    def run(self, binary: bytes, args: list[Any]) -> Any: ...
```

This replaces the former `BackendProtocol` in `jit-core`, which was
hardcoded to `list[CIRInstr]`.  `BackendProtocol` is kept as an alias for
backwards compatibility.

### `CodegenResult[IR]`

```python
@dataclass
class CodegenResult(Generic[IR]):
    binary: bytes | None
    ir: IR                     # post-optimization IR snapshot
    backend_name: str
    compilation_time_ns: int
    optimizer_applied: bool
    # Derived:
    success: bool
    binary_size: int
```

`compile_with_stats()` returns this dataclass so JIT/AOT caches can store
both the binary and its provenance metadata without separate bookkeeping.

### `BackendRegistry`

A simple name-to-backend map for decoupling backend selection from
pipeline construction:

```python
registry = BackendRegistry()
registry.register(Intel4004Backend())
backend = registry.get("intel4004")
```

---

## What moved into codegen-core

| Source | What | Destination |
|--------|------|-------------|
| `jit-core/cir.py` | `CIRInstr` dataclass | `codegen_core/cir.py` |
| `jit-core/backend.py` | `BackendProtocol` | `codegen_core/backend.py` (now generic `Backend[IR]`) |
| `jit-core/optimizer.py` | constant fold + DCE for CIR | `codegen_core/optimizer/cir_optimizer.py` |
| (new) | `CodegenPipeline[IR]` | `codegen_core/pipeline.py` |
| (new) | `CodegenResult[IR]` | `codegen_core/result.py` |
| (new) | `BackendRegistry` | `codegen_core/registry.py` |
| (new) | `IrProgramOptimizer` (wraps ir-optimizer) | `codegen_core/optimizer/ir_program.py` |

---

## What changed in dependent packages

### jit-core

- `jit_core.cir` re-exports `CIRInstr` from `codegen_core`.
- `jit_core.backend` re-exports `Backend` / `BackendProtocol` from `codegen_core`.
- `jit_core.optimizer` re-exports `run` from `codegen_core.optimizer.cir_optimizer`.
- `JITCore.__init__` builds a `self._pipeline: CodegenPipeline[list[CIRInstr]]`.
- `JITCore._compile_fn` calls `self._pipeline.compile_with_stats(cir)` instead of
  calling `optimizer.run(cir)` + `backend.compile(cir)` separately.
- `pyproject.toml`: added `coding-adventures-codegen-core` dependency.
- All public APIs unchanged; re-exports ensure zero breakage for callers.

### aot-core

- `aot_core.specialise` imports `CIRInstr` from `codegen_core` (was `jit_core.cir`).
- `AOTCore.__init__` builds a `self._pipeline: CodegenPipeline[list[CIRInstr]]`.
- `AOTCore._compile_fn` calls `self._pipeline.compile(cir)`.
- `AOTCore._optimize()` removed — optimization is now handled by the pipeline.
- `pyproject.toml`: replaced `coding-adventures-jit-core` with `coding-adventures-codegen-core`.
- Backwards dependency on jit-core **eliminated**.

### intel4004-backend

- `backend.py` imports `CIRInstr` from `codegen_core` (was `jit_core.cir`).
- `pyproject.toml`: replaced `coding-adventures-jit-core` with `coding-adventures-codegen-core`.

---

## Updated layer diagram

```
Language frontend (lexer + parser + type-checker + bytecode compiler)
                         ↓
         ┌───────────────────────────────────────────────┐
         │  Interpreted-language path                    │
         │  IIRModule (interpreter-ir)                   │
         │    ↓ vm-core: interpreted execution           │
         │    ↓ jit-core: hot-fn detection + specialise  │  → list[CIRInstr]
         │    ↓ aot-core: static inference + specialise  │  → list[CIRInstr]
         └──────────────────────┬────────────────────────┘
                                ↓ list[CIRInstr]
                   ╔════════════════════════════════════╗
                   ║         codegen-core               ║
                   ║  CodegenPipeline[list[CIRInstr]]   ║
                   ║  OR                                ║
                   ║  CodegenPipeline[IrProgram]        ║
                   ║                                    ║
                   ║  Optimizer.run() → Backend.compile() ║
                   ╚═══════════════════╤════════════════╝
                                       ↓ bytes | None
         ┌─────────────────────────────────────────────────────────┐
         │  Compiled-language path                                  │
         │  Source → frontend → IrProgram (compiler-ir)            │
         │    ↓ IrProgramOptimizer (via codegen-core)              │  → IrProgram
         └─────────────────────────────────────────────────────────┘
                                       ↓
              backend (intel4004, wasm, jvm, clr, riscv32, ...)
```

---

## Package layout

```
codegen-core/
  pyproject.toml      deps: interpreter-ir, compiler-ir, ir-optimizer
  BUILD
  src/codegen_core/
    __init__.py         public re-exports
    cir.py              CIRInstr
    backend.py          Backend[IR] (generic), BackendProtocol (alias)
    pipeline.py         CodegenPipeline[IR], Optimizer[IR]
    result.py           CodegenResult[IR]
    registry.py         BackendRegistry
    optimizer/
      __init__.py
      cir_optimizer.py  constant fold + DCE for list[CIRInstr]
      ir_program.py     IrProgramOptimizer (wraps ir-optimizer)
  tests/
    test_cir.py            13 tests — CIRInstr construction, str, predicates
    test_cir_pipeline.py   27 tests — CodegenPipeline[CIR] all paths
    test_ir_pipeline.py    10 tests — CodegenPipeline[IrProgram] + IrProgramOptimizer
    test_registry.py       12 tests — BackendRegistry
    test_optimizer.py      22 tests — constant folding, DCE, CIROptimizer class
```

---

## Dependency graph (after LANG19)

```
interpreter-ir ◄── vm-core ◄── jit-core ──► codegen-core
compiler-ir    ◄── ir-optimizer ──────────► codegen-core
                                            ╔════════════╗
                                            ║ codegen-core║
                                            ╚════════════╝
                                                  ▲
                               jit-core ──────────┤
                               aot-core ──────────┤
                               intel4004-backend ─┘
```

`aot-core` no longer depends on `jit-core`.

---

## Test coverage

| Package | Tests | Coverage |
|---------|-------|----------|
| codegen-core (new) | 84 | 91.35% |
| jit-core | unchanged | unchanged |
| aot-core | unchanged | unchanged |
| intel4004-backend | unchanged | unchanged |

Zero test changes in any existing package — the refactor is internal
plumbing, with re-exports preserving all public APIs.

---

## Follow-on: LANG20

LANG20 (``CodeGenerator[IR, Assembly]`` Protocol) adds a finer-grained
split within the codegen concern.  Where ``Backend[IR]`` bundles validate +
generate + assemble + run, ``CodeGenerator[IR, Assembly]`` covers only the
validate-and-generate-assembly step.  All six ``ir-to-*`` compiler packages
implement ``CodeGenerator`` as thin adapter classes.  See
``code/specs/LANG20-codegen-generator.md``.
