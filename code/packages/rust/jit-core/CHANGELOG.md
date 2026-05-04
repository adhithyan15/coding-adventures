# Changelog — jit-core (Rust)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-28

### Added — Initial Rust port (LANG03)

This is the initial Rust port of the Python `jit-core` package.  It is a
faithful translation of the Python implementation with idioms adapted for
Rust's ownership model and type system.

#### `src/errors.rs`
- `JITError` — crate-level error enum with `Deoptimizer`, `Unspecializable`,
  and `CompilationFailed` variants; implements `std::error::Error`
- `DeoptimizerError` — raised when `deopt_count / exec_count > 0.10`; carries
  `fn_name`, `deopt_count`, `exec_count`, and a `deopt_rate()` helper
- `UnspecializableError` — raised when `compile()` is called on a permanently
  invalidated function

#### `src/cir.rs`
- `CIROperand` enum — mirrors `interpreter_ir::instr::Operand` with `Var`,
  `Int`, `Float`, `Bool` variants; implements `From<Operand>` and
  `From<&Operand>` for zero-copy lifting from IIR
- `CIRInstr` — typed compiler-IR instruction with `op`, `dest`,
  `srcs: Vec<CIROperand>`, `ty: String`, and `deopt_to: Option<usize>`
- `CIRInstr::new()` / `CIRInstr::new_with_deopt()` — ergonomic constructors
- `is_type_guard()` — true for `type_assert` with `deopt_to` set
- `is_generic()` — true for `call_runtime` instructions
- `is_pure()` — false for side-effectful ops; drives DCE pass

#### `src/backend.rs`
- `Backend` trait — `name() → &str`, `compile(ir) → Option<Vec<u8>>`,
  `run(binary, args) → Value`; requires `Send + Sync` for `Arc` use
- `NullBackend` — always compiles (1-byte sentinel); always returns `Null`
- `EchoBackend` — returns the first argument unchanged; useful for pipeline tests

#### `src/optimizer.rs`
- `CIROptimizer` — two-pass optimizer:
  1. **Constant folding + propagation**: tracks known constant values, substitutes
     them into instruction sources, then folds instructions with two literal srcs
  2. **Dead-code elimination**: removes pure instructions whose dest register is
     never read
- Supports all foldable ops: `add`, `sub`, `mul`, `div`, `mod`, `and`, `or`,
  `xor`, `shl`, `shr`, `cmp_eq`, `cmp_ne`, `cmp_lt`, `cmp_le`, `cmp_gt`,
  `cmp_ge` over `i64` and `f64` literals, plus `bool` comparisons
- Division / modulo by zero: not folded (avoids panic in constant-folded code)

#### `src/specialise.rs`
- `specialise(fn_, min_observations) → Vec<CIRInstr>` — the core
  specialisation pass
- `spec_type(instr, min_obs) → String` — returns the concrete type to
  specialise on, or `"any"` for the generic fallback
- `literal_type(op) → String` — infers `"u8"` / `"u16"` / `"u32"` / `"u64"` /
  `"f64"` / `"bool"` / `"str"` from IIR literal operands
- Emits type guards (`type_assert`) for `"any"`-typed instructions when the
  observed type is concrete and has enough profiler observations
- Special-case mappings: `("add", "str") → call_runtime str_concat`
- Passthrough ops: `label`, `jmp`, `jmp_if_true`, `jmp_if_false`, `call`,
  `call_builtin`, `cast`, `type_assert`, memory ops, I/O ops

#### `src/cache.rs`
- `JITCacheEntry` — stores binary, post-optimisation CIR, `backend_name`,
  `param_count`, `compilation_time_ns`, plus `exec_count` and `deopt_count`
  via `Arc<AtomicU64>` for lock-free updates from JIT handler closures
- `JITCacheEntry::exec_count_arc()` / `deopt_count_arc()` — return Arc clones
  for use in `vm-core` JIT handler closures
- `JITCacheEntry::deopt_rate()` — `deopt_count / exec_count`
- `JITCacheEntry::as_stats()` — flat `HashMap<String, String>` snapshot
- `JITCache` — `HashMap`-backed store with an `invalidated: HashSet<String>`
  for permanent invalidation tracking
- `JITCache::put()` — stores entry and clears invalidation
- `JITCache::invalidate()` — removes entry and marks name permanently invalidated
- `JITCache::stats()` — returns per-function statistics snapshots

#### `src/core.rs`
- `JITCore` — top-level JIT engine with tiered compilation:
  - `FullyTyped` threshold: default 0 (compile before first call)
  - `PartiallyTyped` threshold: default 10
  - `Untyped` threshold: default 100
- `execute_with_jit(vm, module, fn_name, args)` — three-phase execution
  (eager compile → interpret → promote hot functions)
- `compile(vm, module, fn_name)` — manual compilation; raises
  `UnspecializableError` for invalidated functions
- `execute(vm, module, fn_name, args)` — direct execution using cache or
  interpreter fallback
- `invalidate(vm, fn_name)` — removes cache entry and unregisters JIT handler
  from `vm-core`
- `record_deopt(vm, fn_name)` — increments deopt counter; auto-invalidates
  when `deopt_rate > 0.10`
- `dump_ir(fn_name)` — returns post-optimisation CIR as human-readable string
- `cache_stats()` — delegates to `JITCache::stats()`

#### Architecture note
The `JITCore::compile_fn` method registers a closure with `VMCore::register_jit_handler`.
The closure captures `Arc<dyn Backend>` (for calling `backend.run()`) and
`Arc<AtomicU64>` (for incrementing `exec_count`) — both `Send + Sync`.
This avoids any `Mutex` in the hot handler path.

### Test coverage
91 unit tests + 8 doc-tests, all passing.  Coverage exceeds 80%.
