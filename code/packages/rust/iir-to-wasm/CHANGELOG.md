# Changelog — iir-to-wasm

All notable changes to this crate are documented here.  The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
