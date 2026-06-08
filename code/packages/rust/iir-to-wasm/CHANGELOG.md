# Changelog — iir-to-wasm

All notable changes to this crate are documented here.  The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.10.0] — 2026-06-08 (LANG77 / McCarthy L3b-3a-3c — `alloc` actually allocates)

### Fixed

- **`alloc` now emits a real allocation.** It previously lowered to a bare
  `ref.null` (a placeholder), so the cons cell was *null* and the very next
  `field_store` (`struct.set`) trapped on a null reference. It now pushes a
  typed null for each of the `$LispyPair`'s two `anyref` fields and then
  `struct.new`, yielding a real `(null . null)` heap object that the following
  `field_store`s overwrite. Uses only the already-supported `struct.new` /
  `struct.set` / `struct.get` ops (no engine change).

This completes the wasm side of the McCarthy cons end-to-end: with the
structural representation pass (boxing atoms / unboxing the result) in
`iir-builtin-lowering`, `(CAR (CONS 7 9))` now compiles to a `.wasm` that runs
to `7` on the in-repo `wasm-runtime`.

## [0.9.0] — 2026-06-04 (LANG77 / McCarthy L3b-3a — i31ref `box`/`unbox`)

### Added — WasmGC integer boxing

- `box` and `unbox` are no longer in `UNSUPPORTED_OPS`. They now lower to the
  WasmGC i31 reference ops:
  - **`box dest, src`** → `ref.i31` (`GcInstruction::I31New`, bytes `0xFB 0x1C`):
    box an `i32` into an `i31ref` (a tagged 31-bit integer reference).
  - **`unbox dest, src`** → `i31.get_s` (`GcInstruction::I31GetS`, bytes
    `0xFB 0x1D`): read it back as a sign-extended `i32`.
- These are the boxing primitives the **uniform-anyref lisp value model**
  needs: a lisp integer atom becomes an `i31ref` so it can live in a
  `$LispyPair`'s `anyref` field alongside heap pairs, and is unboxed only at
  the numeric boundary (the program's return value) — mirroring the native
  NaN-box `(n << 3)` / arithmetic `>> 3` discipline. The retype/box pass that
  *emits* these ops for a McCarthy module is the next slice (L3b-3a-2).

### Verification note

The repo has no WasmGC runtime or validator (its `wasm-simulator` is MVP-only;
`wasm-validator` is structural-only), so these are verified at the **opcode-byte
level** — the new tests assert the emitted code contains `0xFB 0x1C` / `0xFB
0x1D` and that `box`/`unbox` pass validation. End-to-end execution of WasmGC
output remains out of scope (documented like the macOS-native-exe gap).

## [0.8.0] — 2026-06-01 (G2 — whitelist `call_builtin "print_i64"`)

### Changed — `call_builtin "print_i64"` now reaches real wasm bytecode

Pre-0.8.0, BASIC's `PRINT` lowered to `call_builtin "print_i64"`,
and the validator rejected it with
`UnsupportedOp ... print_i64 ... not in the WASM backend's
host-import whitelist (supported: ["putchar", "getchar"])`.

The host import already existed under a different name: the
`io_out` opcode (a Twig/Lispy mechanism) wires
`env.__print_i64 : (i64) -> ()`.  G2 makes `call_builtin
"print_i64"` reuse that same import — no new host function needed,
no breaking change for existing `io_out` users.

The end-to-end effect is that BASIC programs containing `PRINT`
statements now reach real `.wasm` bytecode through the same single
encoder pipeline as Twig.

### Implementation

- `validate.rs::CALL_BUILTIN_SUPPORTED_NAMES`: `"print_i64"` added.
- `lower.rs::collect_module_features`: a `call_builtin "print_i64"`
  flips `uses_io_out` so the `env.__print_i64` import is wired in
  even when the module never uses the `io_out` opcode.
- `lower.rs::emit_instr`: new `"print_i64"` arm in the
  `call_builtin` branch loads the i64 argument and emits
  `call <print_fn_idx>` — identical lowering to the `io_out`
  opcode.

### Tests

- 4 new tests in `tests/test_backend.rs`:
  - `g2_call_builtin_print_i64_validator_accepts`
  - `g2_call_builtin_print_i64_lowers_to_wasm_bytes`
  - `g2_call_builtin_print_i64_injects_host_import`
  - `g2_unknown_builtin_still_rejected` (regression marker —
    confirms G2 didn't widen the whitelist beyond `print_i64`)
- All 95 existing tests still pass.


## [0.7.0] — 2026-06-01 (G1 — accept `cmp_*`-prefixed comparison opcodes)

### Changed — `cmp_eq` / `cmp_ne` / `cmp_lt` / `cmp_le` / `cmp_gt` / `cmp_ge` now lower

Pre-0.7.0, the `lower_iir_to_wasm` step only recognised the bare
shape (`eq` / `ne` / `lt` / `le` / `gt` / `ge`) — the form
`twig-ir-compiler` emits.  Languages that prefix the mnemonic with
`cmp_` — BASIC, Nib, Oct — would fail at lowering even though the
validator accepted them, surfacing as
`IIR -> WasmModule: UnsupportedOp { op: "cmp_gt" }`.

This release accepts both shapes.  The implementation strips a
leading `cmp_` from the opcode name and routes the bare form
through the existing per-type opcode dispatch (i32/i64 signed/
unsigned + f32/f64).  No new opcodes are added; the wasm
comparison opcode table is unchanged.  Twig's existing bare-form
emissions continue to lower identically.

This unblocks:
- BASIC `IF A > 5 THEN 100` lowering to wasm
- BASIC `FOR I = 1 TO 3 / NEXT I` (cmp_le) lowering to wasm
- Nib `if a < b { ... }` lowering to wasm
- Oct `while x < 10 { ... }` lowering to wasm

### Tests

- 7 new tests in `tests/test_backend.rs`: one per `cmp_*` variant
  asserting `lower_iir_to_wasm` no longer rejects, plus a
  back-compat test for the bare-`eq` form.
- All 95 existing unit tests still pass.

## [0.6.0] — 2026-05-26 (Validator accepts `ref<any>` for `field_load`)

### Changed — `ref<any>` joins `SUPPORTED_REF_TYPES`

Companion to Twig path-A increment 6c.  The Phase 2 heap-lowering
convention is `field_load dest, pair, idx [ref<any>]` — the loaded
value's type is `ref<any>` because cons-cell fields can hold any
Lisp value.  WasmGC lowering already declares cons-cell fields as
`(mut (ref null any))`, so the actual code shape is `struct.get`
returning `anyref`, which matches `ref<any>`.

This release widens the WASM validator:

- `SUPPORTED_REF_TYPES` now includes `ref<any>` (in addition to
  `ref<LispyPair>`).
- `alloc` continues to require `ref<LispyPair>` only (we can't
  allocate an unknown struct shape).
- `field_load` accepts either `ref<any>` (canonical Phase 2) or
  `ref<LispyPair>` (forward-compat).
- Other ops with `ref<any>` type_hint flow through Check 4
  (UnsupportedType) without rejection.

No lowering changes — `struct.get` already produces an `anyref`-
compatible value.

## [0.5.0] — 2026-05-24 (Validator accepts `field_store [void]`)

### Changed — `validate_for_wasm` now accepts `field_store [void]`

Companion to Twig path-A increment 6b.  The Phase 2 heap-lowering
convention is `field_store cell, idx, value [void]` (the store has no
result, so its type_hint is `"void"`).  iir-builtin-lowering emits
this form, and BEAM, JVM, CLR validators all accept it.  WASM
previously required `type_hint == "ref<LispyPair>"` for `field_store`,
which was inconsistent.

This release widens WASM's `field_store` rule: `"void"` is accepted
canonically; `"ref<LispyPair>"` continues to work for forward
compatibility with frontends that propagate the object type onto the
store.

No lowering changes — `lower.rs` already produces `struct.set` from
both shapes.

## [0.4.0] — 2026-05-22 (Brainfuck — linear memory + I/O imports)

### Added — Brainfuck `load_mem` / `store_mem` / `call_builtin` lowering

#### Validator changes

- `validate_for_wasm` now **accepts** `load_mem` and `store_mem` — they
  were previously in `UNSUPPORTED_OPS`.  Both lower to WASM linear-memory
  ops (`i32.load8_u`, `i32.store8`) over a module-defined memory.
- `call_builtin` is now **conditionally** accepted: the builtin name
  carried in `srcs[0]` must be in the new
  `CALL_BUILTIN_SUPPORTED_NAMES` whitelist.  Today's whitelist covers
  Brainfuck's two I/O builtins (`putchar`, `getchar`); extending it
  takes three steps documented in the constant's doc comment.
- Unknown builtin names still produce a clear `UnsupportedOp` error
  with the builtin name and the whitelist included.

#### Lowering changes

- New `ModuleFeatures` struct collected by `collect_module_features`
  (replaces the narrower `collect_globals_and_io`).  Captures
  `uses_io_out`, `uses_putchar`, `uses_getchar`, and `uses_memory`
  flags in a single module walk.
- When `uses_putchar`: inject `env.putchar : (i32) -> ()` host import.
- When `uses_getchar`: inject `env.getchar : () -> i32` host import.
- When `uses_memory`: inject a single 1-page (64 KiB) linear `Memory`
  — the Brainfuck tape.  Programs that don't use memory ops get no
  memory section, preserving binary compatibility with existing
  non-BF callers (Twig, BASIC, Oct, Nib, Lispy).
- Function-index space: imports occupy the first slots in
  declaration order — `env.__print_i64` (LANG32, when used),
  then `env.putchar`, then `env.getchar`.  Defined functions follow.
- New `emit_instr` arms:
  - `load_mem` → `local.get addr; i32.load8_u; local.set dest`
  - `store_mem` → `local.get addr; local.get val; i32.store8`
  - `call_builtin "putchar"` → `local.get val; call <putchar_idx>`
  - `call_builtin "getchar"` → `call <getchar_idx>; local.set dest`

#### Why this matters

After this PR, Brainfuck's IIR — including `+++.` (memory + putchar),
`,[.,]` (cat), and the multiplication idiom — flows through the
*same* `iir-to-wasm` backend that Twig, BASIC, Oct, Nib, and Lispy
use.  Stage 1 of 4 for the BF→{wasm,jvm,clr,beam} story; the JVM,
CLR, and BEAM lowerings are queued behind this PR.

#### Tests

- `validate.rs::tests` — 5 new unit tests for the validator changes:
  - `load_mem_accepted_for_bf`
  - `store_mem_accepted_for_bf`
  - `call_builtin_putchar_accepted`
  - `call_builtin_getchar_accepted`
  - `call_builtin_unknown_name_rejected`
- Existing `unsupported_ops_rejected` updated: `load_mem`, `store_mem`,
  `call_builtin` removed from the unconditional-reject list; comments
  point readers to the new tests.
- Existing `tests/test_backend.rs::validate_memory_ops_rejected`
  renamed → `validate_memory_ops_accepted` and updated to assert the
  promotion.
- Doc-tests unchanged.

Total: 45 lib + 88 integration tests pass.

---

## [0.3.0] — 2026-05-12

### Added (LANG35 — Closure Backend Integration)

#### Improved `ClosureOpcode` validator error

- `validate_for_wasm` now emits a dedicated `ClosureOpcode` error message
  (format: `"[fn_name] ClosureOpcode: alloc_closure/call_closure require the
  BEAM backend — WASM does not support heap-allocated closures"`) when it
  encounters `alloc_closure` or `call_closure`.
- Previously these fell through to the generic `UntypedInstruction` path
  because their type hints are `"closure"` and `"any"` respectively — now the
  closure check runs first so the error message is actionable.

#### Tests

- `lang35_alloc_closure_closure_opcode_error`: asserts `validate_for_wasm`
  returns an error containing "ClosureOpcode" for a module with `alloc_closure`.
- `lang35_call_closure_closure_opcode_error`: same for `call_closure`.
- `lang35_closure_opcode_error_not_untyped`: asserts the error does NOT
  contain "UntypedInstruction", confirming the new code path fires first.

---

## [0.2.0] — 2026-05-11

### Added (LANG32 — Global Variables and I/O)

#### Global variable support via WASM global section

- Pre-pass `collect_globals_and_io` scans all functions to find `global_store` /
  `global_load` instructions and `io_out` instructions before emitting code.
- Each named global maps to a `(global i64 (mut (i64.const 0)))` entry added to
  `WasmModule::globals`.  Slot indices are assigned lazily (first encounter = next
  free slot).
- `global_store "x", %v` → `local.get <slot_of_%v>; global.set <idx_of_x>`.
- `global_load "x" → %r` → `global.get <idx_of_x>; local.set <slot_of_%r>`.

#### I/O support via host import

- If any function uses `io_out`, the host import `env.__print_i64 (func (param i64))`
  is prepended to `WasmModule::imports`.
- `io_out %v` → `local.get <slot_of_%v>; call $__print_i64`.
- **Function index offset**: importing `$__print_i64` assigns it function index 0,
  shifting all defined functions up by 1.  The lowerer applies `fn_idx_base = 1`
  when building `fn_map` and export indices so calls remain correct.

---

## [0.1.0] — 2026-05-11

### Added

- Initial release of the `iir-to-wasm` crate.
- `validate_for_wasm()` — pre-flight validation of `IIRModule` for WASM
  lowering.  Reports human-readable errors for empty modules, empty
  functions, untyped instructions, unsupported types, and unsupported ops.
  Unlike the BEAM backend, float type hints (`f32`, `f64`) and float
  constants (`Operand::Float`) are fully supported.
- `IIRWasmConfig` — configuration struct for the lowering pass.  Carries the
  WASM module name.
- `IIRWasmError` — structured error enum for lowering failures, covering
  `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`, `UndefinedLabel`,
  `UndefinedVariable`, and `InvalidOperand`.
- `lower_iir_to_wasm()` — two-pass lowering from `IIRModule` to `WasmModule`.
  - Pass 1: per-function register allocation and local type inference.
  - Pass 2: instruction code generation — arithmetic, bitwise, comparisons,
    constants (i32/i64/f64), function calls, and control flow.
  - Control flow: dispatch-loop pattern for functions with labels/jumps;
    linear emission for functions without.
  - Every function is exported by name.
- `codegen.rs` — internal encoding helpers for WASM binary opcodes: signed
  and unsigned LEB128 immediates, `local.get`/`local.set`, `br`/`br_if`,
  `i32.const`, `i64.const`, `f64.const`, and the binary opcode table.
- `tests/test_backend.rs` — 40+ integration tests covering validation, module
  structure, FunctionBody correctness, encoding round-trips, and all
  major opcode families.
