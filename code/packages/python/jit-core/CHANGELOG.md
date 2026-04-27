# Changelog — coding-adventures-jit-core

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-22

### Added

**Core compilation pipeline**

- `JITCore` — top-level JIT engine that attaches to a `VMCore` instance and
  registers compiled handlers so the VM's dispatch loop bypasses the interpreter
  for hot functions.
- `specialise(fn, min_observations)` — specialization pass translating an
  `IIRFunction` (with type-feedback vectors from `vm-core`'s profiler) into a
  flat `list[CIRInstr]`.  Consults `type_hint` first, then falls back to
  `observed_type` when `observation_count >= min_observations` and the type is
  monomorphic.
- `optimizer.run(cir)` — two-pass optimizer: constant folding (literal-literal
  arithmetic computed at compile time) followed by dead-code elimination (DCE
  removes instructions whose destination register is never read by any
  subsequent instruction).
- `CIRInstr` — typed compiler-IR instruction dataclass.  Fields: `op`, `dest`,
  `srcs`, `type`, `deopt_to`.  Convenience predicates: `is_type_guard()`,
  `is_generic()`.  `__str__` renders a one-line human-readable form.

**Tiered compilation**

- Three compilation tiers driven by `FunctionTypeStatus`:
  - `FULLY_TYPED` — compiled eagerly before the first interpreted call (Phase 1
    of `execute_with_jit`).
  - `PARTIALLY_TYPED` — compiled after `threshold_partial` interpreted calls
    (default 10).
  - `UNTYPED` — compiled after `threshold_untyped` interpreted calls (default
    100).
- A threshold of `0` compiles before any interpreted execution (same as the
  eager path but triggered via the promotion loop instead).

**JIT cache**

- `JITCache` — LRU-less in-memory store mapping function name → `JITCacheEntry`.
- `JITCacheEntry` — records: compiled `binary`, `backend_name`, `param_count`,
  post-optimization `ir`, `compilation_time_ns`, `exec_count`, `deopt_count`,
  and a `deopt_rate` property (`deopt_count / exec_count`).
- `JITCache.now_ns()` — nanosecond wall-clock helper used to time compilation.

**Deoptimization**

- `JITCore.record_deopt(fn_name)` — increments the deopt counter; if
  `deopt_rate > 0.1` the function is immediately invalidated and marked
  unspecializable.
- `JITCore.invalidate(fn_name)` — removes the cached binary, marks the function
  unspecializable, and calls `VMCore.unregister_jit_handler`.
- `UnspecializableError` — raised by `JITCore.compile()` if a previously
  invalidated function is requested for manual compilation.

**Backend protocol**

- `BackendProtocol` — structural protocol (`typing.Protocol`) declaring the
  three methods a backend must implement: `name: str`, `compile(cir)`, and
  `run(binary, args)`.

**Specialization details**

- Binary ops (`add`, `sub`, `mul`, `div`, `mod`, bitwise, all six comparisons)
  emit type-guards + typed CIR on the concrete path, or a `call_runtime
  generic_{op}` on the generic path.
- Unary ops (`neg`, `not`) follow the same guard-then-typed pattern.
- `add` on `str` maps to `call_runtime str_concat` via `_SPECIAL_OPS`.
- Passthrough ops (`label`, `jmp`, `jmp_if_true`, `jmp_if_false`, `call`,
  `call_builtin`, `cast`, `type_assert`, `load_reg`, `store_reg`, `load_mem`,
  `store_mem`, `io_in`, `io_out`) are copied verbatim.
- `const` instructions infer their type from the Python literal when
  `type_hint == "any"`: `bool` → `bool`, small ints → `u8/u16/u32/u64`,
  `float` → `f64`, `str` → `str`, anything else → `any`.

**Test suite**

- 206 tests across 7 files; 98.83 % line coverage (exceeds the 95 % minimum).
- `test_cir.py` — `CIRInstr` construction, `__str__`, `is_type_guard`,
  `is_generic`, deopt_to field.
- `test_specialise.py` — const literal inference, binary ops (generic, typed,
  guard emission, special-ops), unary ops, passthrough ops, min_observations
  threshold, `_spec_type` corner cases.
- `test_optimizer.py` — constant folding (int, float, bool, ZeroDivisionError
  guard, overflow guard), DCE (dead dest removed, side-effectful ops kept,
  labels kept), combined fold+DCE pipeline.
- `test_cache.py` — `JITCacheEntry` (deopt_rate, as_stats), `JITCache`
  (get/put/invalidate/is_invalidated/stats/len/__contains__/now_ns).
- `test_tiers.py` — eager FULLY_TYPED compilation, PARTIALLY_TYPED and UNTYPED
  threshold promotion, JIT handler registration and direct invocation,
  `_promote_hot_functions` edge cases, exception handling in `_compile_fn`.
- `test_deopt.py` — deopt counter increment, rate threshold invalidation,
  manual invalidation, handler unregistration.
- `test_integration.py` — end-to-end pipeline with real `VMCore`; CIR pipeline
  correctness, interpreter fallback, `SummingBackend` result, backend compile
  failure, hot-function promotion after execution, `dump_ir`, `cache_stats`.

**Documentation**

- `README.md` — architecture overview, compilation tiers, deoptimization,
  public API reference, stack position diagram.
- `CHANGELOG.md` — this file.
- `BUILD` — build descriptor for the project's custom build tool.

### Architecture notes

The `jit-core` package is deliberately backend-agnostic.  It produces
`list[CIRInstr]` and delegates native code generation entirely to the
registered `BackendProtocol` implementation.  The specialization and
optimization passes are pure Python functions with no I/O side-effects, making
them straightforward to test in isolation.

The optimizer's constant-folding base-op extraction (`op.split("_")[0]`)
means comparison ops (`cmp_eq`, `cmp_ne`, etc.) are not reachable via the
foldable-ops table; this is documented behavior and left for a future
`ir-optimizer` package that will use a proper opcode taxonomy.
