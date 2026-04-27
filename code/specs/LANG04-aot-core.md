# LANG04 — aot-core: Ahead-of-Time Compilation Path

## Overview

`aot-core` is the **ahead-of-time compilation path** for interpreted languages.
Where `jit-core` (LANG03) compiles hot functions *at runtime* after observing
their behaviour, `aot-core` compiles the *entire program* at build time —
before any user has run it.

The distinction matters:

| | JIT | AOT |
|--|-----|-----|
| When? | At runtime, after observing hot functions | At build time, before first run |
| Input | InterpreterIR + runtime feedback | InterpreterIR alone |
| Output | Binary resident in process memory | Binary written to disk |
| Type information | From profiler (observed) | From type checker (static) |
| Warmup time | Yes (interpreted until threshold) | Zero (always compiled) |
| Portability | Runs on compilation host only | Cross-compilable (target ≠ host) |

AOT makes sense when:
- The source language is fully (or sufficiently) typed
- Cold-start time matters (CLIs, embedded systems, IoT)
- The target hardware cannot host a runtime (Intel 4004 with 160 bytes RAM)
- A snapshot binary must be distributed to a user who has no language toolchain

This spec covers:

1. The AOT pipeline (IIR → CompilerIR → binary)
2. Handling of untyped functions (type inference or deopt stubs)
3. The `vm-runtime` linkable library (for AOT programs that need runtime support)
4. Cross-compilation (host ≠ target)
5. Snapshot format (`.aot` binary)
6. Integration with the existing `ir-optimizer` passes

---

## Why AOT is possible for interpreted languages

An interpreted language does not *require* a runtime interpreter at execution
time — the interpreter is a **convenience tool** for the developer.  The
program's semantics can be fully captured in a compiled binary if:

1. All types are known (statically typed, or inferred to be monomorphic)
2. All function calls can be statically resolved (no dynamic dispatch)
3. All dynamic features used by the program (closures, first-class functions)
   can be reduced to static equivalents by the compiler

For programs that meet these conditions, `aot-core` emits a standalone binary
with no dependency on the interpreter at all.

For programs that use dynamic features, `aot-core` links in the **vm-runtime**
library — a compiled form of the `vm-core` interpreter that is bundled into
the binary.  The vm-runtime handles only the dynamic parts; static parts are
compiled directly.

This is exactly how CPython's `pyc` files work, how Java's AOT compilers work,
and how BASIC implementations on early home computers worked: the BASIC
interpreter was stored in ROM and the BASIC program could call back into it for
dynamic features like `INPUT` and `PRINT`.

---

## The AOT pipeline

```
IIRModule (bytecode)
    │
    ├── Type inference (for UNTYPED functions)
    │     aot-core runs a lightweight Hindley-Milner–style inference pass
    │     over IIRInstr sequences.  Functions that remain polymorphic after
    │     inference are compiled with vm-runtime call stubs (see below).
    │
    ├── Specialization (IIR → CompilerIR)
    │     Same pass as jit-core's specialise.py, but without feedback vectors.
    │     Observes type_hint fields only (not observed_type).
    │     UNTYPED functions with inferred types → specialize normally.
    │     UNTYPED functions that remain "any" → emit vm-runtime call stub.
    │
    ├── ir-optimizer passes (constant folding, DCE, inlining)
    │
    ├── Backend compilation (CompilerIR → native binary)
    │     Same backend-protocol as jit-core.  The backend doesn't know whether
    │     it is being called from JIT or AOT.
    │
    └── Linker (optional)
          If any vm-runtime stubs are present, link the vm-runtime library.
          The linker resolves stub addresses to vm-runtime function pointers.
          Output: a single `.aot` file containing all functions + vm-runtime.
```

---

## Type inference for UNTYPED functions

AOT cannot observe runtime types, so it runs a **static inference pass** over
the `IIRInstr` sequence:

```
Rule: if all srcs of an instruction have a known type, the dest type is inferred.

Examples:
  const 42        → u8  (literals in 0–255 range → u8)
  add u8, u8      → u8
  add u8, u16     → u16  (promotion)
  cmp_lt u8, u8   → bool
  jmp_if_false bool → (control, no result)
```

The inference is flow-insensitive (no SSA phi analysis) for simplicity.  It
is correct for straight-line code and for code where all branches produce the
same type.  For code where branches produce different types (e.g., an `if`
returning `u8` on one branch and `str` on another), the result type is `"any"`
and the function falls back to the vm-runtime stub path.

---

## vm-runtime: the linkable interpreter library

`vm-runtime` is a **compiled, linkable form** of `vm-core`.  It provides the
same dispatch loop as `vm-core`, but compiled to the target architecture as a
static library that can be linked into an AOT binary.

```
vm-runtime.a  (target-arch static library)
    │
    ├── vm_execute(fn_index, args) → result
    ├── vm_call_builtin(name, args) → result
    └── vm_iir_table[]   (the IIR bytecode of all uncompiled functions)
```

The `vm_iir_table` is a compact binary encoding of all functions that were
not fully compiled (i.e., those that had `"any"` types after inference).
When an AOT binary calls a vm-runtime stub, it calls `vm_execute(fn_index, args)`,
which interprets the function from `vm_iir_table` at runtime.

This is analogous to:
- The ROM BASIC interpreter on the Apple II (interpreter in ROM, calls from compiled code)
- The JVM `invokedynamic` bootstrap mechanism
- LLVM's `libFuzzer` runtime stubs for undefined behaviour handlers

---

## AOT snapshot format (`.aot`)

The `.aot` file is a self-contained binary that can be executed on the target
architecture:

```
Header:
  4 bytes  magic     0x41 0x4F 0x54 0x00  ("AOT\0")
  2 bytes  version   0x01 0x00
  4 bytes  flags     bit 0: vm_runtime linked; bit 1: debug info present
  4 bytes  entry_point_offset  (byte offset of the main function's code)
  4 bytes  vm_iir_table_offset (0 if no vm-runtime)
  4 bytes  vm_iir_table_size
  4 bytes  native_code_size

Code section:
  N bytes  native machine code for all compiled functions

IIR table section (optional):
  M bytes  serialised IIRModule (LANG01 serialisation format) for dynamic fns

Debug section (optional):
  Mapping of native instruction offsets → IIR instruction indices → source lines
```

The file is directly executable on the target architecture (or loadable by
the target simulator):

```python
# Execute an .aot file on the Intel 4004 simulator:
from intel4004_simulator import Intel4004Simulator
with open("program.aot", "rb") as f:
    aot = f.read()
sim = Intel4004Simulator()
sim.load_rom(aot[header.native_code_offset:])
result = sim.run()
```

---

## Cross-compilation

`aot-core` supports cross-compilation when the backend supports it.  The
backend protocol (LANG05) includes a `target_arch` field that determines
the instruction set of the emitted binary.  The host machine (running
`aot-core`) can be x86-64 while the target is Intel 4004.

This is how the existing Tetrad JIT already works: the 4004 binary is
generated on a modern laptop, fed to `Intel4004Simulator`, and the result
is returned.  The JIT is doing cross-compilation — it just doesn't call
itself that.

```python
aot = AOTCore(
    backend=intel4004_backend,   # target: Intel 4004
    vm_runtime=None,             # no runtime needed for fully typed programs
)
binary = aot.compile(iir_module)
with open("program.aot", "wb") as f:
    f.write(binary)
```

For programs that need the vm-runtime on a real target (not a simulator),
the vm-runtime must be pre-compiled for that target:

```
vm-runtime/
  prebuilt/
    vm_runtime_intel8080.a    # pre-compiled for Intel 8080
    vm_runtime_riscv32.a      # pre-compiled for RISC-V 32-bit
    vm_runtime_wasm32.a       # pre-compiled for WASM
```

---

## Integration with ir-optimizer

`aot-core` runs the same `ir-optimizer` passes as `jit-core`:

1. **Constant folding** — evaluate `const + const` at compile time
2. **Dead code elimination** — remove instructions whose `dest` is never used
3. **Function inlining** — inline small callees into the caller (AOT-only; too
   expensive to do at JIT time for first-call compilation)
4. **Loop unrolling** — unroll small fixed-iteration loops (AOT-only)

The inlining and loop unrolling passes are AOT-only because they increase
binary size (acceptable for AOT) and take more compile time (acceptable offline;
not acceptable during a hot JIT compile).

---

## Handling dynamic features

Some language features cannot be compiled to static code:

| Feature | AOT treatment |
|---------|---------------|
| `INPUT` (BASIC) | vm-runtime builtin call |
| Eval / dynamic code execution | vm-runtime call |
| First-class functions (closures over mutable state) | vm-runtime closure object |
| `GOTO` with computed target | vm-runtime dispatch table |
| Recursive types (linked lists) | vm-runtime heap allocation |
| Dynamic string operations | vm-runtime string calls |

A fully typed, non-recursive, no-eval program compiles to a pure native binary
with zero vm-runtime dependency.  Tetrad programs are in this category because
Tetrad's type system and restricted feature set guarantee it.

---

## Public API

```python
class AOTCore:
    def __init__(
        self,
        backend: BackendProtocol,
        vm_runtime: VmRuntime | None = None,
        optimization_level: int = 2,        # 0=none, 1=basic, 2=full
        debug_info: bool = False,
    ) -> None: ...

    def compile(self, module: IIRModule) -> bytes:
        """Compile the entire module to a .aot binary. Returns the raw bytes."""

    def compile_to_file(self, module: IIRModule, path: str) -> None:
        """Compile and write to path."""

    def stats(self) -> AOTStats:
        """Return compilation statistics (functions compiled, inlined, etc.)."""
```

---

## Package structure

```
aot-core/
  pyproject.toml
  BUILD
  README.md
  CHANGELOG.md
  src/aot_core/
    __init__.py       # exports AOTCore, AOTStats
    core.py           # AOTCore class
    infer.py          # static type inference for UNTYPED functions
    specialise.py     # IIR → CIR (shared logic with jit-core; extracts to ir-specialise)
    link.py           # linker: merge native code + vm_iir_table into .aot
    snapshot.py       # .aot format writer / reader
    vm_runtime.py     # vm-runtime library loader
    stats.py          # AOTStats dataclass
  tests/
    test_infer.py
    test_compile_full.py     # fully typed program → no vm-runtime
    test_compile_partial.py  # partially typed → vm-runtime linked
    test_snapshot.py         # write/read .aot roundtrip
    test_cross_compile.py    # host != target (mocked backend)
```

---

## When to use JIT vs AOT

| Scenario | Recommendation |
|----------|---------------|
| Interactive REPL | JIT — user expects instant results |
| Long-running server | JIT — warmup acceptable; benefits from runtime feedback |
| CLI tool (cold-start critical) | AOT — warmup unacceptable |
| Embedded target (no host runtime) | AOT — interpreter cannot run on target |
| Full static types | AOT — no feedback needed; compile fully |
| Dynamic / polymorphic code | JIT — better specialization from observed types |
| Distribution without toolchain | AOT — `.aot` is self-contained |

The Tetrad pipeline uses **both**: the `tetrad-jit` package is the JIT path;
`aot-core` + `intel4004-backend` is the AOT path that produces `.aot` files
that run directly on `Intel4004Simulator` with no Python required at run time.
