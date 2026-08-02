# Changelog — jit-core (Rust)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.5.1] — 2026-08-02

### Fixed — compiled boolean procedures preserve their runtime type

`GenericCirJit` represented every non-void return with `RET_I64`, including
`ret_bool`. The integer carrier is suitable for arithmetic and branching, but
is not structurally equal to `Value::Bool`: a caller that used a compiled
boolean procedure result in a later boolean comparison could take the wrong
branch. The JIT bytecode now has `RET_BOOL` (`0x62`), and `ret_bool` returns
`Value::Bool` while integer returns continue to use `RET_I64`.

The regression test compiles `neg(p) = not p` with a seeded result slot and
checks both boolean arguments and boolean return values through the compiled
parameter ABI.

## [0.5.0] — 2026-06-14 (LANG-FULL E2 — register width & wrap, backend 2 of 6)

### Added — the compiled tier wraps narrow-width arithmetic

`GenericCirJit`'s bytecode compiler mapped every integer-width arithmetic op
(`add_u8`, `mul_u16`, …) to the same width-erased `ADD_I64`/`MUL_I64`/… opcode
and ran it at full `i64` width, so a JIT-compiled `200u8 + 100u8` produced `300`
instead of `44`.  (The interpreter tier already wrapped via vm-core 0.5.0.)

This adds a `MASK_WIDTH <reg> <bits>` opcode (`0x15`).  `compile_to_bytecode`
now emits it right after a narrow-suffixed (`_u8`/`_u16`/`_u32`) add / sub / mul
/ div / neg, and the run loop applies `regs[reg] &= (1<<bits)-1` — so the
compiled tier wraps mod-2ⁿ exactly like vm-core's `mask_result`.  `u4` is not in
the CIR allowlist, so a `u4`-typed op specialises to the generic path and runs
on the interpreter tier (which masks it); the observable wrap is identical.
Signed narrow types and `i64`/`u64`/`any` keep full machine width.

Unit tests: `add_u8`/`mul_u8`/`sub_u8` wrap, `u16`/`u32` widths, `neg_u8`, and
`i64`-width-does-not-mask.

## [0.4.1] — 2026-06-14 (CIROptimizer constant-propagation soundness fix)

### Fixed — stale constants no longer survive a reassignment or a block boundary

`CIROptimizer::constant_fold` recorded a register → literal binding for every
`const_<t>` and propagated it into later instructions, but it never *removed* a
binding when the register was later **overwritten**.  A function that seeds a
slot with a constant and then reassigns it — exactly how an ALGOL function
procedure lowers its result variable —

```text
const_i64 sq = 0      ; known[sq] = 0
mul_i64   t  = x, x
mov       sq = t      ; sq reassigned, but known still says 0
ret_i64   sq          ; ← the dead 0 is propagated; should return t
```

was silently miscompiled: `ret sq` had the stale `0` substituted, so the
JIT-compiled `sq(7)` returned `0` instead of `49`.  This surfaced when an ALGOL
typed procedure (`integer procedure sq(x); value x; integer x; sq := x*x`) was
called on the JIT backend in `lang-aot`'s executed conformance matrix — every
other backend agreed on `49`; only the JIT disagreed.

The fix adds two soundness rules to the linear constant-propagation pass:

1. **Reassignment kills.** Any instruction that writes a register without
   re-establishing a constant for it now drops that register's known binding.
2. **Block boundaries kill everything.** With no control-flow graph, a constant
   is only valid within its basic block; the map is cleared at every `label`
   (a join point a backward edge may reach) and at every jump/branch. This also
   pre-empts a latent loop-miscompilation where a constant defined before a loop
   would be propagated into iterations where the register had since changed.

Regression tests cover the reassignment case, the across-a-label case, and a
straight-line fold to confirm the rules do not over-clear.

## [0.4.0] — 2026-06-13 (LANG-MATRIX Phase I — compiled functions bind their parameters)

### Fixed — JIT-compiled functions with parameters now read their arguments

A function compiled by `GenericCirJit` ignored the arguments it was called
with: `Backend::run(&self, binary, _args)` discarded `_args` and started every
call with a zero-initialised register file, so a parameter read as `0`.  A
function like Nib's `double(x) -> x + x`, compiled and invoked as `double(21)`,
returned `0` instead of `42`.  Parameterless functions (e.g. BASIC's `main`)
were unaffected, which is why this lay hidden until the LANG-MATRIX **JIT
column** exercised a parameterised function end-to-end.

The fix threads parameter context through the **existing** `compile_function` /
`FunctionContext` infrastructure (no new trait surface, no bytecode-format
change):

- `JITCore::compile_fn` now calls `Backend::compile_function(&ctx, ir)` instead
  of the bare `compile(ir)`, where `ctx` carries the function's name,
  parameters (in declaration order), and return type.  IR-only backends are
  unaffected — the trait's default `compile_function` forwards to `compile`.
- `GenericCirJit::compile_function` pre-binds the parameter names to registers
  `0, 1, 2, …` in declaration order *before* walking the body, so the
  parameters deterministically occupy the first registers.  A duplicate
  parameter name (malformed IR) makes the function uncompilable rather than
  silently aliasing two params to one register.
- `GenericCirJit::run` seeds registers `0..args.len()` from the incoming call
  arguments (bounded by the 256-register file).  Because `compile_fn` passes
  the arguments in declaration order, argument `i` lands in the register that
  parameter `i` was bound to.

Native backends (which already override `compile_function` for their ABI
prologue) now receive the context from `jit-core`'s compile path too.

New unit tests: `compiled_function_reads_its_argument` (the 21→42 regression,
with the bare-`compile` contrast still returning 0),
`two_params_map_to_args_in_declaration_order` (argument `i` → register `i` even
when the body references the second param first), and
`duplicate_parameter_name_is_uncompilable`.

## [0.3.0] — 2026-05-28 (GenericCirJit — universal bytecode JIT)

### Added — `jit_core::generic_jit::GenericCirJit`

A universal bytecode JIT backend that **any language with typed IIR**
can plug into.  Eliminates the per-language `Backend` duplication
that was emerging with `BrainfuckCirJit` (~918 lines) and
`BasicCirJit` (~640 lines) — ~70% of each is the same logic:
register allocation, typed CIR opcode encoding, branch fixups,
dispatch loop.

#### How languages use it

Three lines instead of ~600:

```rust
let backend = GenericCirJit::new();
backend.register_builtin("print_i64", |args| { /* …captured I/O… */ });
JITCore::new(&mut vm, Box::new(backend))
    .execute_with_jit(&mut vm, &mut module, "main", &[]);
```

#### Supported CIR opcodes

- **Constants**: `const_{i8|i16|i32|i64|u8|u16|u32|u64|bool}` → CONST_I64
- **Move**: `mov` → MOV
- **Arithmetic** (i64 family): `add_*`, `sub_*`, `mul_*`, `div_*`,
  `neg_*` → ADD/SUB/MUL/DIV/NEG_I64
- **Comparisons** (i64 family): `cmp_{eq|ne|lt|le|gt|ge}_*`
- **Control flow**: `label`, `jmp`, `jmp_if_true`, `jmp_if_false`
- **Linear memory** (when configured): `load_mem`, `store_mem`
- **Builtins**: `call_builtin` → CALL_BUILTIN with 2-byte builtin
  index into a per-binary name table
- **Returns**: `ret_*` → RET_I64 / RET_VOID

Float arithmetic refused (returns `None` from `compile()`).

#### Builtin callback registry

`GenericCirJit::register_builtin(name, |args| → Value)` registers
language-specific callbacks.  Compile-time, `compile()` resolves
each `call_builtin "name"` to a 2-byte index and emits a name table
prefix in the bytecode.  At run-time, the index → callback lookup
is O(1).

#### Linear memory

`GenericCirJit::with_linear_memory(tape_size)` allocates a fresh
`Vec<u8>` of `tape_size` bytes per `run()` call.  Brainfuck's tape
model fits directly: `load_mem` returns 0 for OOB addresses
(lazy-infinite-tape convention), `store_mem` errors on OOB writes.

#### Step counter + error slot

`GenericCirJit::steps_handle()` / `error_handle()` expose
`Arc<Mutex<…>>` handles so the wrapping VM can inspect fuel use
and surface execution errors (which `Backend::run`'s signature
can't return as a `Result`).

#### Tests

- 9 unit tests in `generic_jit::tests`: compile + run for const,
  add, cmp+jmp, divide-by-zero, builtin dispatch, unregistered
  builtin rejection, float refusal, load_mem/store_mem with and
  without linear memory.
- 2 end-to-end tests in `tests/generic_jit_e2e.rs`: full JITCore
  flow with a BASIC-shaped print + arithmetic program.

#### What's next

`BrainfuckCirJit` and `BasicCirJit` continue to ship in their own
crates for backwards compatibility, but new languages (Oct, Nib,
Twig) plug into `GenericCirJit` directly — no per-language Backend
impl.  Future PR will migrate Brainfuck + BASIC onto
`GenericCirJit` and delete ~1500 lines of duplicated code.

---

## [0.2.0] — 2026-05-11

### Changed (LANG32 — Operand::Str exhaustiveness)

- `cir.rs`: `From<Operand>` and `From<&Operand>` now handle `Operand::Str(s)` —
  maps it to `CIROperand::Var(s)` (treats the string literal name as an
  identifier in the CIR representation).
- `specialise.rs`: `literal_type` now handles `Some(Operand::Str(_))` — returns
  `"str"` sentinel, consistent with the `Var` path.

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
