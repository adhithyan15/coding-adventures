# LANG03 — jit-core: Generic JIT Specialization Engine

## Overview

`jit-core` is a **language-agnostic JIT compiler** that monitors a `vm-core`
execution, detects hot functions, and compiles them to native code using any
registered backend.

The Tetrad JIT (`tetrad-jit`, TET05) demonstrated the full pipeline:

```
Tetrad bytecode → JIT IR → Intel 4004 binary → Intel4004Simulator
```

`jit-core` lifts that pipeline out of the Tetrad package and makes it generic:

```
InterpreterIR + feedback vectors
    → specialization pass (InterpreterIR → CompilerIR)
    → ir-optimizer (SSA optimization passes)
    → backend (CompilerIR → native binary)
    → execution (ISA simulator or native call)
```

The language-specific parts that `tetrad-jit` contained — nibble-pair
arithmetic, liveness-based register recycling for the 4004's 6-pair limit,
Intel 4004 assembly encoding — move into the `intel4004-backend` package.
`jit-core` itself is pure IR manipulation and dispatch.

This spec covers:

1. Hot-function detection and tiered thresholds
2. The specialization pass (InterpreterIR → CompilerIR)
3. Type guards and deoptimisation
4. The compiled-function cache (`JITCache`)
5. The backend dispatch protocol
6. Integration with `vm-core`'s shadow frames

---

## Tiered compilation

`jit-core` implements three compilation tiers, matching the thresholds
established by `TetradJIT`:

| Tier | Trigger | Rationale |
|------|---------|-----------|
| `FULLY_TYPED` | Before first call | All types are statically known; no feedback needed |
| `PARTIALLY_TYPED` | After N₁ calls (default: 10) | Some types known, some observed |
| `UNTYPED` | After N₂ calls (default: 100) | All types from feedback; polymorphic → deopt |

The thresholds are configurable at construction time:

```python
jit = JITCore(
    vm,
    backend=intel4004_backend,
    threshold_fully_typed=0,      # compile before call 1
    threshold_partial=10,
    threshold_untyped=100,
)
```

A threshold of `0` means "compile eagerly before the first interpreted call".
This is the Tetrad behaviour for FULLY_TYPED functions.

---

## Architecture

```
vm-core running IIRModule
    │
    │  after each interpreted call
    ▼
JITCore._promote_hot_functions()
    │
    │  function crossed its threshold
    ▼
JITCore._compile_fn(fn: IIRFunction)
    │
    ├── specialise(fn, feedback_vectors)
    │       IIRInstr list → CompilerIR block list
    │
    ├── ir-optimizer passes (constant folding, DCE)
    │       CompilerIR → CompilerIR (optimized)
    │
    ├── backend.compile(compiler_ir)
    │       CompilerIR → bytes (native binary)
    │
    └── JITCache.put(fn.name, binary)

JITCore.execute(fn_name, args)
    │
    ├── cache hit  → backend.run(binary, args)
    └── cache miss → vm.execute(fn) [interpreted fallback]
                     _promote_hot_functions() [after fallback]
```

---

## Specialization pass

The specialization pass converts `IIRFunction` → `CompilerIR` using the
observation slots filled by the `vm-core` profiler.

### Pass overview

```python
def specialise(fn: IIRFunction, min_observations: int = 5) -> list[CIRBlock]:
    """
    Walk IIRInstr list; emit CompilerIR instructions.

    For instructions whose type_hint == "any":
      - If observed_type is concrete and observation_count >= min_observations:
          emit a type guard (type_assert), then specialize the operation
      - If observed_type == "polymorphic" or count < min_observations:
          emit a generic (unspecialized) operation
    """
```

### Type guard emission

For an `add` instruction observed as `u8`:

```
IIRInstr:  op="add", dest="v0", srcs=["a","b"], type_hint="any",
           observed_type="u8", deopt_anchor=3

CompilerIR emitted:
    ; type guards
    type_assert a, u8   (deopt_to=3)
    type_assert b, u8   (deopt_to=3)
    ; specialized operation
    v0 = add_u8 a, b
```

For the same `add` instruction with `observed_type="polymorphic"`:

```
CompilerIR emitted:
    ; no guard; generic runtime call
    v0 = call_runtime "generic_add" [a, b]
```

### Instruction mapping

| IIRInstr op | Observed type | CompilerIR op |
|-------------|---------------|---------------|
| `add` | `u8` | `add_u8` |
| `add` | `u16` | `add_u16` |
| `add` | `str` | `call_runtime "str_concat"` |
| `add` | `polymorphic` | `call_runtime "generic_add"` |
| `cmp_lt` | `u8` | `cmp_lt_u8` |
| `jmp_if_false` | `bool` | `br_false_bool` |
| `const` | any | `const_<type>` |
| `ret` | any | `ret_<type>` |

The full mapping table is defined in `specialise.py` and is backend-independent.
Backends see only typed `CompilerIR` operations.

---

## CompilerIR subset used by jit-core

`jit-core` emits a subset of the full `CompilerIR` format — only the
instructions that have direct backend representations:

```python
@dataclass
class CIRInstr:
    op: str                          # typed mnemonic, e.g. "add_u8"
    dest: str | None
    srcs: list[str | int | float | bool]
    type: str                        # always a concrete type; never "any"
    deopt_to: int | None = None      # interpreter instruction index for guards
```

SSA phi-nodes, loop induction variables, and other high-level CompilerIR
constructs are not generated by `jit-core`.  They are generated by the
compiled-language path (SemanticIR → CompilerIR → ir-optimizer → backend).

---

## Type guards and deoptimisation

A type guard is a `type_assert` instruction in the emitted `CompilerIR`.
The backend translates it to a conditional branch:

```
; Backend (Intel 4004 example):
;   R(2p) must equal expected_hi, R(2p+1) must equal expected_lo
;   If not, jump to deopt stub.
FIM P7, 0x00       ; expected hi nibble = 0 (u8 guard: hi must be 0 for values 0–255)
LD  R14            ; A = R14 (hi nibble of value in P7)
XCH R(2p)         ; compare with actual hi nibble
JNZ deopt_3       ; deopt to instruction 3 if hi nibble is wrong
```

The deopt stub saves the current register state, restores the interpreter
shadow frame, and jumps to `vm.resume_frame(frame, at_ip=deopt_anchor)`.

### Deopt frequency tracking

The JIT cache entry tracks how many times each compiled function deopts:

```python
@dataclass
class JITCacheEntry:
    fn_name: str
    binary: bytes
    backend_name: str
    param_count: int
    ir: list[CIRInstr]             # post-optimization IR
    compilation_time_ns: int
    deopt_count: int = 0           # incremented by the deopt stub at runtime
    exec_count: int = 0            # incremented on each compiled execution
```

If `deopt_count / exec_count > 0.1` (more than 10% deopt rate), `jit-core`
**invalidates** the compiled version and marks the function as
`UNSPECIALIZABLE`.  The function then runs interpreted forever.

---

## JITCache

```python
class JITCache:
    """Dictionary-backed cache mapping function names to compiled binaries."""

    def get(self, fn_name: str) -> JITCacheEntry | None: ...
    def put(self, entry: JITCacheEntry) -> None: ...
    def invalidate(self, fn_name: str) -> None: ...
    def stats(self) -> dict[str, dict]: ...
    @staticmethod
    def now_ns() -> int: ...   # monotonic nanosecond timestamp
```

`invalidate` removes the entry and marks the function so it is never
re-compiled (avoids thrashing for fundamentally polymorphic functions).

---

## Hot-function detection

```python
def _promote_hot_functions(self) -> None:
    if self._module is None:
        return
    counts = self._vm.metrics().function_call_counts
    for fn in self._module.functions:
        if self.is_compiled(fn.name):
            continue
        if fn.name in self._unspecializable:
            continue
        threshold = self._thresholds.get(fn.type_status)
        if threshold is None:
            continue
        if threshold == 0:
            continue   # FULLY_TYPED functions are compiled eagerly in execute_with_jit
        if counts.get(fn.name, 0) >= threshold:
            self._compile_fn(fn)
```

This is identical in structure to `TetradJIT._promote_hot_functions()` (TET05),
generalised to use `IIRFunction.type_status` instead of Tetrad-specific enums.

---

## Public API

```python
class JITCore:
    def __init__(
        self,
        vm: VMCore,
        backend: BackendProtocol,          # see LANG05
        threshold_fully_typed: int = 0,
        threshold_partial: int = 10,
        threshold_untyped: int = 100,
        min_observations: int = 5,
    ) -> None: ...

    def execute_with_jit(self, module: IIRModule) -> int | None:
        """Run module under the interpreter; auto-compile hot functions.

        Phase 1: compile all FULLY_TYPED functions before first interpreted call.
        Phase 2: run main() via the interpreter.
        Phase 3: promote any function that turned hot during Phase 2.
        Returns the return value of main, or None if no main function.
        """

    def compile(self, fn_name: str) -> bool:
        """Manually compile fn_name. Returns True on success, False on deopt."""

    def execute(self, fn_name: str, args: list[int]) -> int | None:
        """Execute fn_name (compiled or interpreted). Updates call counts."""

    def is_compiled(self, fn_name: str) -> bool:
        """Return True if fn_name has a cached native binary."""

    def cache_stats(self) -> dict[str, dict]:
        """Return per-function cache statistics."""

    def dump_ir(self, fn_name: str) -> str:
        """Return the post-optimization CompilerIR for fn_name as a string."""

    def invalidate(self, fn_name: str) -> None:
        """Remove the compiled version; mark as unspecializable."""
```

---

## Integration with vm-core shadow frames

When `jit-core` compiles a function, it registers a **JIT handler** with
`vm-core`:

```python
vm.register_jit_handler(fn_name, lambda args: backend.run(binary, args))
```

`vm-core`'s dispatch loop checks for a registered JIT handler before running
the interpreted path:

```python
def _dispatch_call(self, frame, instr):
    fn_name = resolve_fn_name(instr)
    handler = self._jit_handlers.get(fn_name)
    if handler is not None:
        result = handler(resolved_args)
        self._metrics.total_jit_hits += 1
        return result
    return self._dispatch_call_interpreted(frame, instr)
```

This means the compiled path requires **zero overhead** in the dispatch loop for
functions that are still interpreted — the `dict.get` check is O(1) and returns
`None` most of the time.

---

## Compiler pipeline

```python
def _compile_fn(self, fn: IIRFunction) -> bool:
    t0 = JITCache.now_ns()
    try:
        cir = specialise(fn, min_observations=self._min_observations)
        cir = ir_optimizer.run(cir)      # constant folding, DCE
        binary = self._backend.compile(cir)
    except Exception:
        return False

    if binary is None:
        return False

    t1 = JITCache.now_ns()
    self._cache.put(JITCacheEntry(
        fn_name=fn.name,
        binary=binary,
        backend_name=self._backend.name,
        param_count=len(fn.params),
        ir=cir,
        compilation_time_ns=t1 - t0,
    ))
    self._vm.register_jit_handler(fn.name, lambda args: self._backend.run(binary, args))
    return True
```

---

## Package structure

```
jit-core/
  pyproject.toml
  BUILD
  README.md
  CHANGELOG.md
  src/jit_core/
    __init__.py       # exports JITCore
    core.py           # JITCore class — top-level API + hot-function promotion
    specialise.py     # IIRFunction → list[CIRInstr] specialization pass
    cir.py            # re-exports CIRInstr from codegen_core (LANG19)
    backend.py        # re-exports Backend / BackendProtocol from codegen_core (LANG19)
    optimizer.py      # re-exports run() from codegen_core.optimizer.cir_optimizer (LANG19)
    cache.py          # JITCache, JITCacheEntry
    errors.py         # DeoptimizerError, UnspecializableError
  tests/
    test_specialise.py
    test_tiers.py
    test_deopt.py
    test_cache.py
    test_integration.py  # end-to-end: IIRModule → JIT → native result
```

---

## Relationship to tetrad-jit

`tetrad-jit` implements every piece of this spec for the Tetrad language and
Intel 4004 backend specifically.  The generalisation is:

| `tetrad-jit` | `jit-core` equivalent |
|--------------|----------------------|
| `translate.py` (bytecode → JIT IR) | `specialise.py` (IIR → CIR) |
| `passes.py` (constant folding, DCE) | `ir-optimizer` package |
| `codegen_4004.py` (IR → 4004 binary) | `intel4004-backend` package |
| `cache.py` | `jit_core.cache` |
| `__init__.py` TetradJIT class | `jit_core.core` JITCore class |

After `jit-core` is implemented:
- `tetrad-jit` becomes a 30-line compatibility shim that constructs
  `JITCore(vm_core_instance, backend=intel4004_backend)`
- All the logic moves into `jit-core` and `intel4004-backend`
- The existing `tetrad-jit` tests are re-run against the new implementation
  with zero changes to the test assertions

---

## Relationship to codegen-core (LANG19)

LANG19 extracted the shared parts of the compilation pipeline from `jit-core`
into `codegen-core` so that `aot-core` no longer depends on `jit-core`:

- `CIRInstr` → moved to `codegen_core.cir`; `jit_core.cir` re-exports it.
- `BackendProtocol` → moved to `codegen_core.backend` as generic `Backend[IR]`;
  `jit_core.backend` re-exports it.
- `optimizer.run()` → moved to `codegen_core.optimizer.cir_optimizer`;
  `jit_core.optimizer` re-exports it.
- `JITCore.__init__` now builds a `CodegenPipeline[list[CIRInstr]]` internally.
  `_compile_fn` delegates to `pipeline.compile_with_stats(cir)` instead of
  calling `optimizer.run()` + `backend.compile()` separately.

**All public APIs are unchanged.** The re-exports ensure existing callers
of `from jit_core.cir import CIRInstr` etc. continue to work without
modification.
