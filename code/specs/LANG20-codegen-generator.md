# LANG20 — CodeGenerator[IR, Assembly]: Generic IR-to-Assembly Protocol

## Overview

LANG19 introduced `Backend[IR]` — a protocol that bundles validate, codegen,
assemble, and run into one interface.  That was correct for the
`intel4004-backend` use-case where the backend owns the full pipeline.

LANG20 introduces a **finer-grained protocol** for the codegen concern alone:
given a typed IR, validate it for a target architecture and generate assembly
code.  What you do with that assembly next (assemble it, package it, JIT-run
it, feed it to a simulator) is the concern of downstream pipelines, not the
code generator.

---

## Why a separate CodeGenerator protocol?

Consider the full pipeline after LANG19:

```
Frontend (Nib, BF, Algol, BASIC, Tetrad)
    ↓ IrProgram  (or list[CIRInstr] — via LANG21 bridge (shipped))
[Optimizer] — optional IR → IR transformation
    ↓
[Backend] — validate + generate + assemble + run   ← LANG19 monolith
```

This conflation means:

- No way to target GE-225 from a Brainfuck frontend without bundling the
  GE-225 simulator inside the same backend object.
- No way to inspect the assembly text before it is assembled.
- No shared interface between the GE-225, JVM, WASM, and CIL compilers.
- AOT and JIT pipelines must re-implement the same validate-then-generate
  logic for each target.

LANG20 splits the pipeline at the assembly boundary:

```
Frontend (Nib, BF, Algol, BASIC, Tetrad)
    ↓ IrProgram  (or list[CIRInstr] — future LANG21 bridge)
[Optimizer] — optional IR → IR transformation
    ↓
[CodeGenerator] — validate + generate assembly     ← LANG20
    ↓ Assembly (str | bytes | WasmModule | CILProgramArtifact | …)
    ├─→ [Assembler → Packager] → executable binary  (AOT pipeline, future)
    ├─→ [JIT runner]           → execute immediately (JIT pipeline, future)
    └─→ [Simulator]            → run directly        (orthogonal, future)
```

The simulator pipeline is **orthogonal**: it receives the assembly output
(packaged or not) and runs it directly on the simulated hardware — no binary
encoding step is needed for a software simulator.  The AOT and JIT pipelines
are nearly identical after the assembly step; AOT adds the package-to-
executable step, JIT executes immediately.

---

## Protocol definition

```python
from typing import Any, Protocol, TypeVar, runtime_checkable

IR = TypeVar("IR")
Assembly = TypeVar("Assembly")


@runtime_checkable
class CodeGenerator(Protocol[IR, Assembly]):
    """Validates IR for a target architecture and generates target assembly.

    Does NOT assemble (text → binary), package, link, or execute.
    The Assembly return type varies by target:
      - str                 for text-assembly backends (Intel 4004, Intel 8008)
      - bytes               for backends that emit binary directly (GE-225, JVM)
      - WasmModule          for WASM (structured; encode separately)
      - CILProgramArtifact  for CIL (structured; CLR simulator accepts directly)
    """

    name: str

    def validate(self, ir: IR) -> list[str]:
        """Validate IR for this target.

        Returns a list of human-readable error strings.  An empty list means
        the IR is compatible with this target.  Does not raise — callers can
        inspect the full list before deciding whether to proceed.
        """
        ...

    def generate(self, ir: IR) -> Assembly:
        """Generate assembly from IR.

        Calls validate() internally.  Raises an exception (backend-specific)
        if the IR is invalid for this target.  If you need to inspect errors
        before generating, call validate() first.
        """
        ...
```

`@runtime_checkable` enables `isinstance(obj, CodeGenerator)` checks.

---

## Assembly type rationale

Each target emits a different "assembly" type:

| Backend | Assembly type | Rationale |
|---------|--------------|-----------|
| Intel 4004 | `str` | Text assembly; no assembler built into the package |
| Intel 8008 | `str` | Text assembly; same rationale |
| GE-225 | `CompileResult` | Packs `binary: bytes` + metadata (halt address, label map) |
| JVM | `JVMClassArtifact` | Packs `class_bytes: bytes` + metadata (callable labels, offsets) |
| WASM | `WasmModule` | Structured 1.0 module; `wasm-module-encoder` encodes to bytes |
| CIL | `CILProgramArtifact` | Structured multi-method artifact; CLR simulator accepts directly |

The GE-225 and JVM backends already produce binary-form output — they combine
codegen and assembly in one step.  The WASM and CIL backends return structured
objects that can be encoded separately.  The Intel backends produce text.

This heterogeneity is intentional: each target's natural output form is
preserved.  The `CodeGenerator[IR, Assembly]` protocol is doubly generic so
the static type checker tracks the assembly type end-to-end.

---

## CodeGeneratorRegistry

```python
class CodeGeneratorRegistry:
    """Name → CodeGenerator lookup, independent of IR/Assembly types."""

    def register(self, generator: Any) -> None: ...
    def get(self, name: str) -> Any | None: ...
    def get_or_raise(self, name: str) -> Any: ...
    def names(self) -> list[str]: ...
    def all(self) -> list[Any]: ...
```

Generators are keyed by `generator.name`.  Registering a second generator
with the same name replaces the first.

---

## Concrete implementations (adapter classes)

All six existing `ir-to-*` compilers receive a thin adapter class that
satisfies `CodeGenerator[IrProgram, Assembly]`:

| Package | Class | `name` | Assembly type |
|---------|-------|--------|--------------|
| `ir-to-ge225-compiler` | `GE225CodeGenerator` | `"ge225"` | `CompileResult` |
| `ir-to-jvm-class-file` | `JVMCodeGenerator` | `"jvm"` | `JVMClassArtifact` |
| `ir-to-wasm-compiler` | `WASMCodeGenerator` | `"wasm"` | `WasmModule` |
| `ir-to-cil-bytecode` | `CILCodeGenerator` | `"cil"` | `CILProgramArtifact` |
| `ir-to-intel-4004-compiler` | `Intel4004CodeGenerator` | `"intel4004"` | `str` |
| `ir-to-intel-8008-compiler` | `Intel8008CodeGenerator` | `"intel8008"` | `str` |

**Adapter pattern:**

```python
class GE225CodeGenerator:
    name = "ge225"

    def validate(self, ir: IrProgram) -> list[str]:
        return validate_for_ge225(ir)

    def generate(self, ir: IrProgram) -> CompileResult:
        return compile_to_ge225(ir)   # calls validate internally, raises on error
```

**No new runtime dependency on `codegen-core`** — the adapters are
structurally compatible with `CodeGenerator` without importing it at runtime.
`codegen-core` may be imported under `TYPE_CHECKING` for annotation purposes.
The existing BUILD files are therefore unchanged.

---

## Validation protocol differences

Four backends (`ge225`, `jvm`, `wasm`, `cil`) already expose `list[str]`
validation functions (`validate_for_ge225`, `validate_for_jvm`, etc.).

The two Intel backends use `IrValidator` objects that return
`list[IrValidationError]`.  The adapter converts to `list[str]` by extracting
`error.message` from each `IrValidationError`.

---

## What LANG20 does NOT include

- **Assembler** (text → binary): deferred to the AOT pipeline spec.
- **Packager** (binary → executable): deferred to the AOT pipeline spec.
- **JIT runner** (assemble + execute immediately): deferred to JIT pipeline spec.
- **Simulator pipeline**: orthogonal; takes Assembly and runs it directly.
- **`CIRInstr → IrProgram` bridge**: **LANG21 (shipped)** — the
  `cir-to-compiler-ir` package provides `lower_cir_to_ir_program()`.
  Tetrad programs can now be compiled end-to-end through any of the six
  `CodeGenerator[IrProgram, Assembly]` adapters.

---

## Out of scope (future specs)

| Spec | Topic | Status |
|------|-------|--------|
| LANG21 | `CIRInstr → IrProgram` lowering bridge | ✅ **Shipped** |
| LANG22 | AOT pipeline: `CodeGenerator` → Assembler → Packager → binary | Planned |
| LANG23 | JIT pipeline: `CodeGenerator` → Assembler → execute | Planned |
| LANG24 | Simulator pipeline: Assembly → run directly on simulator | Planned |
