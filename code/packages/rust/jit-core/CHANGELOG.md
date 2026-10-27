# Changelog — jit-core (Rust)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## Unreleased — LANG83 PR7e — no float writers

- **Breaking:** `write_f64_py` / `write_f64_js` leave `OP_SUPPORT`, and the
  `CALL_BUILTIN_F64` bytecode (0x51) that handed a float register to their
  handler is removed. 0x51 stays unassigned, so stale bytecode naming it is an
  unknown opcode and fails closed. A `call_builtin` that reads a float
  register now refuses the function like any other non-float use of one.
  `PARAM_F64`, `const_f64` and every float op are unchanged.
- New `GenericCirJit::builtin_names()`: the registered builtins, sorted.
- Tests: the PR6a float-register tests observe the bits through `ret_f64`
  instead of a writer; `run_ieee` returns the result and prints only its sign
  bit; `the_retired_call_builtin_f64_byte_is_an_unknown_opcode` and a
  "retired float writer" refusal row replace the writer tests.

## Unreleased — LANG83 PR7d — `Trap.Unbound`

- No code change: `TRAP 7` reports its kind through
  `exception_kind::trap_kind_name`, so the error slot now reads
  `trap: Trap.Unbound` (was `trap: Trap.ReferenceError`) after LANG83 PR7d
  renamed the kind.

## Unreleased — LANG83 PR7b — the language-neutral float primitives

- **New bytecodes `CLASS_F64` (0x5D), `DIGITS_F64` (0x5E), `EXP10_F64`
  (0x5F)**, shaped like `SIGNBIT_F64`: a float register in, an ordinary
  integer register out, computed by `vm_core::float_format` (the VM's own
  core, so the tiers agree). 0x59–0x5C are the int⇄float conversion
  bytecodes (below). The one-operand float ops now share an `f64_unary`
  table; a dest that is also a float register is refused, as for
  `f64_signbit`. A propagated float literal operand is accepted.
- `OP_SUPPORT`: `f64_class`, `f64_digits`, `f64_exp10` are `Lowered`; the
  declaration test types their result `i64`.
- Tests: `f64_digit_queries_read_a_float_register`, refusal cases, and the
  truncated-bytecode check for the three tags.

## Unreleased — a failed narrowing raises `Trap.ConvRange`

- `F64_TO_I64_TRUNC` / `F64_TO_I64_FLOOR` write `trap: Trap.ConvRange` to the
  error slot when vm-core's `real_to_i64_checked` rejects the value, and
  `take_error` returns the same `VMError::Trap` the interpreter tier raises
  ([LANG83](../../../specs/LANG83-iir-standard-operation-registry.md)). The `real_to_int: ` pass-through in `error_from_message` is
  removed: no slot text uses it any more.

## Unreleased — LANG83 conversions on the JIT

- **`int_to_real`, `real_to_int_trunc` and `real_to_int_floor` are
  `Lowered`.** They were `Refused(INTERPRETER_TIER_ONLY)`, so every function
  that mixed integers and floats ran on the interpreter tier: Python's
  `1 + 2.5`, BASIC's `INT` and ALGOL's `entier`. They now compile to new
  bytecodes, specified in
  [LANG83](../../../specs/LANG83-iir-standard-operation-registry.md),
  "Conversions on the JIT":
  - `I64_TO_F64` reads an integer register (or a folded integer literal) and
    writes a float register's bits with Rust `as f64` (round to nearest, ties
    to even).
  - `F64_TO_I64_TRUNC` / `F64_TO_I64_FLOOR` read a float register (or a
    folded float literal), apply `trunc()` / `floor()`, then narrow with
    vm-core's `real_to_i64_checked`.
- **Both sides are checked.** A float source given to `int_to_real`, an
  integer source given to a narrowing op, a narrowing destination that is
  also a float register, or an `int_to_real` into a non-`f64` parameter
  refuses the function. Bits are never reinterpreted.
- **Non-integer values fail closed (security review).** `run` used to seed
  every non-`f64` parameter with `as_i64().unwrap_or(0)`, and stored every
  `call_builtin` result the same way. So a `Float(2.5)` or string argument,
  or a builtin returning a `Float`, silently read as 0, and with
  `int_to_real` now lowered, `int_to_real` of such a value returned 0.0
  where the interpreter raises. Now:
  - every parameter now has a checked prologue entry. The new `PARAM_I64`
    entry is the integer twin of `PARAM_F64`. An argument that is not `Int`
    or `Bool` fails the call. A missing argument leaves the register 0, as
    before. An argument beyond the declared parameters is ignored, as in the
    interpreter's `handle_call`; it used to be written into a body temporary;
  - a builtin result other than `Int`, `Bool` or a void `Null` errors too.
    A void `Null` stored into a destination is still 0. That is the one
    deliberate leftover, because the CIR does not yet mark a void builtin's
    destination.

  Tests: `f64_parameter_is_loaded_by_a_checked_prologue` now pins the error
  instead of the old 0. New: `int_to_real_of_a_float_argument_fails_closed`
  and `a_non_integer_builtin_result_fails_closed`.
- **Failures match the VM.** A non-finite or out-of-range narrowing writes
  vm-core's exact `real_to_int: …` text. `error_from_message` returns that
  text as the same `VMError::Custom`, without the `generic-cir-jit:` prefix.
- **Tests:**
  - `int_to_real` on 0, -1, 2⁵³, 2⁵³+1, `i64::MIN` and `i64::MAX`, plus a
    literal;
  - trunc and floor on fractions of both signs and on -2⁶³;
  - NaN, ±∞, 2⁶³, -1e30 and 1e300, checked against the VM's own error;
  - a mixed int/float function;
  - eight wrong-class refusals.
  The declaration test confirms that the JIT compiles all three.

## [0.10.1] — 2026-10-26 (frame register limit on the JIT path)

- **Tests only** (`tests/register_limit.rs`): vm-core 0.32.0 refuses a
  function with more than 65,536 frame registers with
  `VMError::RegisterLimitExceeded` instead of silently dropping the
  65,537th. `GenericCirJit` encodes registers in one byte, so such a
  function is declined (stays interpreted) — and `execute_with_jit` must
  then surface the interpreter's typed error, never `undefined variable`.
  A function at exactly the limit still runs. No code change: the JIT's own
  256-register limit already declines cleanly rather than dropping.

## [0.10.0] — 2026-10-24 (LANG83 PR6c — non-trapping IEEE f64 arithmetic and `f64_signbit`)

- **Bytecodes `ADD_F64` / `SUB_F64` / `MUL_F64` / `DIV_F64` / `NEG_F64` /
  `SIGNBIT_F64` and `RET_F64`** ([LANG83](../../../specs/LANG83-iir-standard-operation-registry.md) PR6c). A float register holds the value's
  bit pattern; the ops decode, compute in Rust `f64` (never a trap), and
  encode. `NEG_F64` flips bit 63; `SIGNBIT_F64` is `bits >> 63` (unsigned).
- The PR6a float-register set widens to the destinations of the five
  arithmetic ops; those ops, `f64_signbit` and `ret_f64` may read one, and a
  float literal folded into an operand is materialised as its bits. Any other
  use of a float register still refuses the function, and `f64_signbit`'s
  integer dest may not also be a float register.
- `RET_F64` returns a float register as `Value::Float`, so a function that
  returns an `f64` it computed is now JIT-compiled (it fell back before).
- `op_support`: all six `Lowered`; the declaration test types
  `f64_signbit`'s result as `i64`.
- **A parameter not typed `f64` that a float op or `const_f64` writes now
  refuses the function** (security review M1). Such a parameter is seeded as
  an integer, so `f64_add x <- x, 1.0` on an `i64`/`any` `x` read the
  integer's bits as a float: `f(Int(5))` and `f(Float(2.5))` both returned
  `Float(1.0)`. The function now falls back to the interpreter; an `f64`
  parameter is still compiled. The `const_f64` case existed since PR6a.
  Regression test `a_non_f64_param_written_by_a_float_op_is_refused`.

## [0.9.0] — 2026-10-20 (LANG83 PR6a — shortest round-trip float output)

- **`write_f64_py` / `write_f64_js` are `Lowered`.** The new `CALL_BUILTIN_F64`
  bytecode (0x51, `<idx:u16> <reg>`) calls the embedder-registered handler with
  one `Value::Float`, rebuilt from the register's bits. The handler formats the
  value; lang-aot's harness uses `vm_core::float_format`, the same code as the
  VM's default builtins.
- **`f64` registers, narrowly.** `const_f64` now compiles to the float's bit
  pattern in an i64 register. An `f64`-typed parameter is loaded by the new
  `PARAM_F64 <reg>` prologue bytecode (0x52), which requires a `Value::Float`
  argument and fails the call closed otherwise. A float passed to any other
  parameter is still coerced to 0, as before, so raw float bits can never
  reach integer code. This last point came from the security review. Only the two float writers may read such a
  register. Any other instruction that touches one refuses compilation, as
  every `f64` function did before: integer arithmetic, a comparison, `mov`,
  `ret`, a plain `call_builtin`, or an integer overwrite. So float bits can
  never be read as an integer, and those functions still run on the
  interpreter tier. `write_f64_*` on a register that is not provably a float
  also refuses.
- `op_support`'s agreement test now compiles each one-op function with its
  parameter list (`compile_function`), as `JITCore` does.

## [0.8.0] — 2026-10-20 (LANG83 PR4 — overflow-checked arithmetic)

- **`GenericCirJit` bytecodes `ADD_OVF_I64` (0x18), `SUB_OVF_I64` (0x19),
  `MUL_OVF_I64` (0x1A), `NEG_OVF_I64` (0x1B)** for the LANG83 PR4 ops
  `i64_add_ovf`, `i64_sub_ovf`, `i64_mul_ovf`, `i64_neg_ovf`. They use Rust's
  `checked_*`; `None` writes `trap: Trap.Overflow` to the error slot and stops
  the function, and `JITCore` drains the slot into `VMError::Trap` (fail
  closed). A truncated checked op in a hostile binary is a `malformed
  bytecode` error, not a panic.
- The compile arms match the full type-first names before any prefix arm, so
  the wrapping `ADD_I64` family (and its narrow-width masking) can never claim
  them. A test pins the opcode bytes.
- **`CIRInstr::is_pure`** treats the four ops as impure: dead-code elimination
  keeps an unused checked op, whose trap is its only observable effect.
- `op_support`: all four `Lowered`, checked by compiling a one-op function per
  registry entry.
- New `tests/fail_closed.rs` case: each checked op, JIT-compiled in a callee
  and overflowing, stops its interpreted caller before the next builtin runs,
  and the error slot is drained.

## [0.7.0] — 2026-10-19 (LANG83 PR1b — floor division, `trap`, `write_i64`)

- **`GenericCirJit` compiles the LANG83 PR1b ops** ([LANG83](../../../specs/LANG83-iir-standard-operation-registry.md)):
  new bytecodes `DIV_FLOOR_I64` / `MOD_FLOOR_I64` (floor quotient / remainder;
  zero divisor → `Trap.DivZero`, `MIN / -1` → `Trap.Overflow`, `MIN mod -1` → 0)
  and `TRAP <kind id>` for `trap "<kind>"` (an unregistered kind refuses
  compilation). A trap writes `trap: <kind>` (`generic_jit::trap_message`, the
  same text `vm_core::VMError::Trap` displays) to the error slot and stops.
- **`write_i64`** is `Lowered` on the same footing as `print_i64`/`putchar`:
  `CALL_BUILTIN` to the embedder-registered handler.
- **`CIRInstr::is_pure`** now treats `trap`, `i64_div_floor` and
  `i64_mod_floor` as side-effecting, so `CIROptimizer`'s dead-code pass keeps a
  floor divide whose quotient is unused (removing it would remove its trap).
- `op_support`: the four PR1b ops flip to `Lowered`; the
  `declared_support_matches_what_generic_jit_compiles` check registers
  `write_i64` like the other output builtins.
- **Fixed — JIT failures fail closed (security review, MEDIUM).** A trap,
  floor-division edge or `div` by zero in compiled code used to write the
  error slot and return `Value::Null`; the vm-core handler passed that `Null`
  to the interpreted caller, which kept running, and `execute_with_jit`
  returned `Ok` (the reviewer's repro: `Ok(Some(Int(0)))` with two `mark`s run
  after the trap). Now: new `Backend::take_error() -> Option<VMError>`
  (default `None`); `GenericCirJit` implements it by draining its slot through
  `generic_jit::error_from_message` (`trap: <registered kind>` →
  `VMError::Trap`, `DIV_BY_ZERO_MESSAGE` → `VMError::DivisionByZero`, anything
  else → `VMError::Custom`); the handler `JITCore` registers and
  `JITCore::execute` call it after every run and return `Err`. Draining also
  stops a stale message from shadowing a later failure (`set_error` keeps only
  the first). New `tests/fail_closed.rs` (reviewer's repro now
  `Err(Trap.Overflow)` with 0 marks; trap through an interpreted call chain;
  `div` by zero from an interpreted caller; direct `execute` + no stale slot)
  and `take_error_maps_and_drains_the_slot`.

## [0.6.0] — 2026-10-19 (LANG83 PR1a — standard operation support)

- **LANG83 op support.** New `op_support` module (`OP_SUPPORT`, `op_support`).
  `Lowered` means `GenericCirJit` compiles the op to JIT bytecode — integer
  `add sub mul div neg`, the `cmp_*` family, and `print_i64`/`putchar` via
  `CALL_BUILTIN` to the embedder-registered handler. Ops that only run because
  `JITCore` falls back to the vm-core interpreter (`mod`, strings, `f64` math,
  conversions, `print_str`) are declared `Refused(INTERPRETER_TIER_ONLY)`, so
  interpreter fallback cannot masquerade as JIT support ([LANG83](../../../specs/LANG83-iir-standard-operation-registry.md)). PR1b ops are
  refused.
- `declared_support_matches_what_generic_jit_compiles` builds a one-op function
  per registry entry, specialises it and asks `GenericCirJit` to compile it, so
  every row is checked against the real compiler rather than restated.

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
