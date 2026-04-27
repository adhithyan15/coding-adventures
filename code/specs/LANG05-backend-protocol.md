# LANG05 — backend-protocol: Code Generation Backend Contract

## Overview

`backend-protocol` defines the **contract** that every code-generation backend
in this repository must satisfy.  A backend is any component that converts
`CompilerIR` (the typed, SSA intermediate representation) into a runnable
binary for a specific instruction set or virtual machine.

Existing backends:

| Backend package | Target | Used by |
|-----------------|--------|---------|
| `intel4004-backend` | Intel 4004 | Tetrad JIT, Nib AOT |
| `wasm-backend` | WebAssembly | Nib WASM, future languages |
| `jvm-backend` | JVM class file | Java-family, Nib JVM |
| `clr-backend` | .NET CIL | C#-family, .NET languages |
| `riscv-backend` | RISC-V 32-bit | future systems languages |

The `backend-protocol` package provides:

1. The `BackendProtocol` abstract base class / Protocol that backends implement
2. The `BackendCapabilities` dataclass describing what a backend supports
3. The `BackendRegistry` for dynamic backend lookup
4. Standard test fixtures that every backend implementation must pass

This spec covers all five items and describes the Intel 4004 backend as a
worked example showing how the Tetrad JIT's `codegen_4004.py` maps onto the
protocol.

---

## BackendProtocol

```python
from abc import ABC, abstractmethod
from typing import Protocol, runtime_checkable

@runtime_checkable
class BackendProtocol(Protocol):
    """Protocol that every code-generation backend must implement."""

    name: str                        # unique identifier, e.g. "intel4004"
    target_arch: str                 # ISA name, e.g. "intel4004", "wasm32", "jvm8"
    capabilities: BackendCapabilities

    def compile(self, ir: list[CIRInstr]) -> bytes | None:
        """
        Compile a list of CompilerIR instructions to a native binary.

        Returns bytes on success, or None if any instruction cannot be encoded
        (deoptimisation — the function continues in the interpreter).

        The returned bytes are in the format expected by run().
        """
        ...

    def run(self, binary: bytes, args: list[int]) -> int:
        """
        Execute a compiled binary with the given arguments.

        args are u8 integers (0–255).  The return value is u8.
        The backend is responsible for loading binary onto the target
        (simulator, native mmap, JVM ClassLoader, etc.).
        """
        ...

    def disassemble(self, binary: bytes) -> str:
        """
        Return a human-readable disassembly of binary.

        Used for debugging and spec validation.  Optional in the sense that
        a backend may return an empty string, but MUST be implemented.
        """
        ...
```

### Protocol vs ABC

`BackendProtocol` is defined as a `typing.Protocol` rather than an `ABC`.
This means:

- A backend class does not need to inherit from `BackendProtocol`
- A backend satisfies the protocol if it has the right attributes and methods
  (structural subtyping, a.k.a. duck typing)
- `isinstance(backend, BackendProtocol)` works at runtime thanks to
  `@runtime_checkable`

This matches the style used by Python's `collections.abc` and makes it easy
to wrap existing backends without modifying their source.

---

## BackendCapabilities

```python
@dataclass
class BackendCapabilities:
    """Describes what a backend can and cannot do."""

    supports_add: bool = True
    supports_sub: bool = True
    supports_mul: bool = False      # Intel 4004 cannot; WASM can
    supports_div: bool = False
    supports_mod: bool = False
    supports_bitwise: bool = False  # and, or, xor, not, shl, shr
    supports_cmp: bool = True       # eq, ne, lt, le, gt, ge
    supports_branches: bool = True  # jmp, jmp_if_true, jmp_if_false
    supports_calls: bool = False    # function calls within compiled code
    supports_io: bool = False       # io_in, io_out
    supports_str: bool = False      # string operations
    supports_u8: bool = True
    supports_u16: bool = False
    supports_u32: bool = False
    supports_u64: bool = False
    max_live_registers: int = 6     # 4004: 6 pairs (P0–P5); WASM: unlimited
    max_code_bytes: int = 4096      # 4004: 4 KB ROM; WASM: unlimited
    is_simulator: bool = True       # True if run() uses a software simulator
    is_cross_compile: bool = True   # True if host != target ISA
```

`jit-core` queries `backend.capabilities` before calling `compile()`.  If the
IR contains an instruction the backend cannot handle (e.g., `mul` on the 4004),
`jit-core` returns `False` immediately without calling `compile()`.

This is the generalisation of the `tetrad-jit` deopt check:

```python
# tetrad-jit codegen_4004.py (per-function deopt check)
UNSUPPORTED_OPS = frozenset({"mul", "div", "mod", "and", "or", ...})
if any(i.op in UNSUPPORTED_OPS for i in ir):
    return None   # deopt

# jit-core (capability-driven, no magic sets)
def _can_compile(self, ir: list[CIRInstr]) -> bool:
    caps = self._backend.capabilities
    for instr in ir:
        if instr.op.startswith("mul") and not caps.supports_mul:
            return False
        if instr.op.startswith("div") and not caps.supports_div:
            return False
        ...
    return True
```

---

## Worked example: Intel 4004 backend

The existing `codegen_4004.py` in `tetrad-jit` maps directly onto the protocol:

```python
class Intel4004Backend:
    name = "intel4004"
    target_arch = "intel4004"
    capabilities = BackendCapabilities(
        supports_mul=False,
        supports_div=False,
        supports_mod=False,
        supports_bitwise=False,
        supports_calls=False,
        supports_io=False,
        supports_str=False,
        supports_u16=False,
        supports_u32=False,
        supports_u64=False,
        max_live_registers=6,
        max_code_bytes=4096,
        is_simulator=True,
        is_cross_compile=True,
    )

    def compile(self, ir: list[CIRInstr]) -> bytes | None:
        gen = CodeGenerator4004()
        try:
            gen.generate(ir)
            return gen.assemble()   # two-pass assembler → bytes
        except DeoptimizerError:
            return None

    def run(self, binary: bytes, args: list[int]) -> int:
        return run_on_4004(binary, args)   # existing helper

    def disassemble(self, binary: bytes) -> str:
        return disassemble_4004(binary)    # nibble-by-nibble decoder
```

The migration from `tetrad-jit` to this protocol is a rename, not a rewrite.

---

## WASM backend capabilities (contrast)

```python
class WasmBackend:
    name = "wasm32"
    target_arch = "wasm32"
    capabilities = BackendCapabilities(
        supports_mul=True,
        supports_div=True,
        supports_mod=True,
        supports_bitwise=True,
        supports_calls=True,
        supports_io=False,           # WASM I/O goes through WASI imports
        supports_str=False,          # string ops need a WASM runtime
        supports_u8=True,
        supports_u16=True,
        supports_u32=True,
        supports_u64=True,
        max_live_registers=1000,     # effectively unlimited (wasm locals)
        max_code_bytes=2**32,        # 4 GB WASM module limit
        is_simulator=True,           # we use wasm-simulator from this repo
        is_cross_compile=True,
    )
```

A language that targets WASM gets `mul`, `div`, and `bitwise` for free and
never needs to deopt those operations.  The same `jit-core` specialization
pass is used; the capability check is the only difference.

---

## Standard backend tests

Every backend must pass a **standard test suite** provided by
`backend-protocol`:

```python
# backend_protocol.testing
def run_standard_tests(backend: BackendProtocol) -> None:
    """Run the standard backend compliance test suite.

    Call this in your backend's test file:
        from backend_protocol.testing import run_standard_tests
        run_standard_tests(Intel4004Backend())
    """
    _test_add(backend)
    _test_sub(backend)
    _test_cmp(backend)
    _test_branch(backend)
    _test_const(backend)
    _test_ret(backend)
    if backend.capabilities.supports_mul:
        _test_mul(backend)
    if backend.capabilities.supports_bitwise:
        _test_bitwise(backend)
    if backend.capabilities.supports_calls:
        _test_calls(backend)
    _test_deopt_unsupported(backend)   # unsupported ops return None
    _test_disassemble(backend)         # returns non-empty string
```

The standard tests guarantee that any backend interoperates correctly with
`jit-core` and `aot-core`.  A backend that passes these tests is drop-in
compatible.

---

## BackendRegistry

```python
class BackendRegistry:
    """Global registry of available backends."""

    _backends: dict[str, BackendProtocol] = {}

    @classmethod
    def register(cls, backend: BackendProtocol) -> None:
        cls._backends[backend.name] = backend

    @classmethod
    def get(cls, name: str) -> BackendProtocol:
        if name not in cls._backends:
            raise KeyError(f"unknown backend: {name!r}")
        return cls._backends[name]

    @classmethod
    def list_backends(cls) -> list[str]:
        return list(cls._backends.keys())
```

Usage:

```python
from backend_protocol import BackendRegistry
from intel4004_backend import Intel4004Backend

BackendRegistry.register(Intel4004Backend())

# Later, in jit-core or aot-core:
backend = BackendRegistry.get("intel4004")
jit = JITCore(vm, backend=backend)
```

The registry is populated at import time by each backend package's
`__init__.py`.

---

## CompilerIR subset (CIRInstr)

The `CIRInstr` type used by backends is the same subset defined in `jit-core`
(LANG03):

```python
@dataclass
class CIRInstr:
    op: str                          # typed mnemonic: "add_u8", "cmp_lt_u8", …
    dest: str | None
    srcs: list[str | int | float | bool]
    type: str                        # always a concrete type
    deopt_to: int | None = None      # interpreter index if guard fails
```

The full `CompilerIR` from the `compiler-ir` package is a superset of
`CIRInstr`.  For compiled languages (Nib, etc.), `ir-optimizer` produces
the full CompilerIR.  For interpreted languages (Tetrad, BASIC), `jit-core`
produces only the `CIRInstr` subset.

Backends must handle at minimum the subset.  Backends that also serve the
compiled-language path (e.g., `wasm-backend` used by both Nib and BASIC)
must handle the full CompilerIR.

---

## Adding a new backend

Steps to add a new backend (e.g., `arm32-backend`):

1. Create `code/packages/<language>/arm32-backend/`
2. Implement `class Arm32Backend` satisfying `BackendProtocol`
3. Fill in `BackendCapabilities` accurately
4. Call `run_standard_tests(Arm32Backend())` in the test suite
5. Register: `BackendRegistry.register(Arm32Backend())`
6. Write a spec `07h-arm32-backend.md` (follows the ISA simulator spec naming)
7. Add to the BUILD file with its dependencies

No changes to `jit-core`, `aot-core`, or any language package are needed.
The new backend becomes available to all languages immediately.

---

## Package structure

```
backend-protocol/
  pyproject.toml
  BUILD
  README.md
  CHANGELOG.md
  src/backend_protocol/
    __init__.py       # exports BackendProtocol, BackendCapabilities, BackendRegistry
    protocol.py       # BackendProtocol (Protocol class)
    capabilities.py   # BackendCapabilities dataclass
    registry.py       # BackendRegistry
    cir.py            # CIRInstr dataclass (shared with jit-core; one source of truth)
    testing.py        # run_standard_tests() + individual _test_* helpers
  tests/
    test_protocol.py      # isinstance checks, capability validation
    test_registry.py
    test_standard_suite.py  # runs standard tests against a mock backend
```

---

## Existing backend migration

The backends that already exist but are not yet protocol-compliant:

| Package | Status | Migration effort |
|---------|--------|-----------------|
| `codegen_4004.py` in `tetrad-jit` | Working, not protocol | Extract `Intel4004Backend` class; ~50 lines |
| `ir_to_wasm_compiler` | Existing | Wrap in `WasmBackend`; ~30 lines |
| `ir_to_jvm_class_file` | Existing | Wrap in `JvmBackend`; ~30 lines |
| `ir_to_cil_bytecode` | Existing | Wrap in `ClrBackend`; ~30 lines |
| `riscv_simulator` | Simulator, not a compiler | Add `compile()` pass; larger effort |

All wrappers preserve the existing package's internal implementation.
The `BackendProtocol` wrapper is a thin adapter (the Adapter pattern).

---

## Design decisions

### Why a Protocol, not inheritance?

Existing backend packages (`ir-to-wasm-compiler`, `ir-to-jvm-class-file`) were
written before `backend-protocol` was specified.  Requiring them to inherit
from a new base class would require modifying each package.  Structural
subtyping (Protocol) lets them satisfy the contract without modification.

### Why is `run()` part of the protocol?

The alternative is a separate `Executor` protocol.  But separating `compile`
from `run` means the registry would need to track two objects per backend.
Every actual backend always needs both — the Intel 4004 compiler always uses
`Intel4004Simulator` to run; the WASM compiler always uses `wasm-simulator`.
Combining them keeps the API simple.

### Why u8 for args/return in `run()`?

`jit-core` currently targets Tetrad's u8 domain.  As languages with wider types
are added (u16, u32), the `run()` signature will be extended to
`run(binary, args: list[int], arg_types: list[str]) -> int`.  For now, u8 is
the correct scope.
