# Changelog — brainfuck-iir-compiler

## [0.2.0] — 2026-05-22 (BF05 — Brainfuck on the LANG VM JIT chain)

### Added — JIT mode (BF05)

`BrainfuckVM::execute_module` now dispatches through
[`jit_core::core::JITCore`] when constructed with `jit=true`, replacing
the previous `"jit=true is not yet supported in BF04"` early-error.
Brainfuck programs now run through the full LANG VM JIT chain
(`vm-core` + `jit-core`) in the same shape as Dartmouth BASIC's
`jit_smoke.rs` did after PR #3888.

#### Why this matters (and the careful caveat)

Brainfuck's IIR is already `FullyTyped` from birth — every instruction
carries a concrete `type_hint` (`"u8"` / `"u32"` / `"void"`).  That
means the JIT chain *would* eagerly tier-promote `main` to a native
backend on the very first call (see `JITCore::execute_with_jit` Phase
1).  In practice, none of the existing JIT backends (NullBackend,
EchoBackend, future WASM/x86_64 ports) know how to lower Brainfuck's
custom `load_mem` / `store_mem` opcodes — the tape memory model is
specific to the Brainfuck wrapper, not part of the standard CIR
vocabulary.

Compiling with a backend that silently can't handle those opcodes
would replace `main` with a stub that returns `Null`, producing no
output.  To prevent that, the wrapper now ships a private
`InterpOnlyBackend` whose `compile()` always returns `None`, telling
`JITCore`:

> "I refuse to compile this function — keep it interpreted forever."

The plumbing is real — the same `VMCore` runs the program with all
five Brainfuck-specific handlers (`putchar`, `getchar`, `load_mem`,
`store_mem`, `label`), the JIT's tier-up path is observable via
`JITCore::cache_stats()`, and the `Phase 3` hot-function promotion
runs (as a no-op against `InterpOnlyBackend`).  When a future backend
learns Brainfuck's tape memory model, swap it in here and BF programs
tier-promote automatically — no other changes needed.

#### What about the JIT vs interpreter parity?

The new `tests/jit_smoke.rs` runs every program twice — once with
`jit=false` (pure interpreter) and once with `jit=true` (JIT chain) —
and asserts byte-identical output.  If they ever diverge, the wiring
is broken.

### Tests

- `tests/jit_smoke.rs` (6 tests): runs through `JITCore::execute_with_jit`
  - `jit_brainfuck_three_increments_print` — `+++.` → chr(3)
  - `jit_brainfuck_pointer_arithmetic` — `>+<.` → chr(0)
  - `jit_brainfuck_cat_with_input` — `,[.,]` echoes `"hello"` → `"hello"`
  - `jit_brainfuck_multiply_2_times_3` — classic BF multiplication idiom → chr(6)
  - `jit_brainfuck_u8_wraparound` — `-.` (0 - 1 wraps to 255)
  - `jit_brainfuck_multiple_outputs` — `+.++.+++.` → chr(1) chr(3) chr(6)
- Updated `vm::tests::jit_true_returns_error_on_run` (which asserted
  the BF04 early-error) into three new tests that verify the JIT path
  produces the same results as the interpreter:
  - `jit_true_runs_simple_program`
  - `jit_and_interp_paths_agree_on_hello_h`
  - `jit_handles_loop_with_input`

### Dependencies

- Added `jit-core` as a runtime dependency (was previously not
  reachable from this crate).

### Compatibility

- **Public API unchanged.**  `BrainfuckVM::new(true, ...)` no longer
  returns an error on subsequent `run` / `execute_module` calls — that
  was the only observable behaviour change.  All existing callers that
  passed `jit=false` continue to behave exactly as before.

## [0.1.2] — 2026-05-11

### Fixed (LANG32 — `Operand::Str` exhaustiveness)

- `resolve_i64` and `resolve_value` in `vm.rs` now handle `Operand::Str`.
  `resolve_i64` maps string literals to `0` (strings have no numeric
  representation in Brainfuck byte-cell semantics). `resolve_value` maps them
  to `Value::Str`, matching the `vm-core` Value type.  Brainfuck programs should
  never produce `Operand::Str` in practice, but the arms are required because
  the `Operand` enum is now non-exhaustive at the binary level after LANG32
  added the `Str` variant.

## [0.1.1] — 2026-05-04

### Fixed (LANG23 PR 23-E compatibility)

- `IIRFunction` struct literals in `compiler.rs` updated to include
  `param_refinements: Vec::new()` and `return_refinement: None` after
  `interpreter-ir` 0.2.0 added those fields.  No behavioural change.

## [0.1.0] — 2026-04-29

### Added

- **BF04 — Rust port** of the Python `brainfuck-iir-compiler` package.
- `compile_source(source, module_name)` — lex + parse + compile Brainfuck to
  `IIRModule` in one call.
- `compile_to_iir(ast, module_name)` — compile an existing `GrammarASTNode`
  from `brainfuck::parser::parse_brainfuck` to `IIRModule`.
- `BrainfuckVM` — high-level wrapper around `vm_core::VMCore` configured for
  Brainfuck semantics:
  - `u8_wrap = true` (cell wraparound on arithmetic)
  - `putchar` / `getchar` builtins wired to per-run byte buffers
  - Bounds-checked `load_mem` / `store_mem` custom opcode handlers
  - `max_steps` label-crossing fuel cap
  - `jit = true` placeholder (errors in BF04; JIT arrives in BF05)
- `BrainfuckError` — dedicated error type for Brainfuck-level failures
  (out-of-bounds, fuel cap exceeded, JIT not available).
- 52 unit tests + 8 doc-tests (60 total).

### Design notes

- Fixed register names (`ptr`, `v`, `c`, `k`) rather than SSA form —
  `vm-core`'s mutable register file means SSA naming would break
  loop-body definitions when the body is skipped.
- All instructions carry concrete `type_hint` (`"u8"` / `"u32"` / `"void"`),
  producing a `FunctionTypeStatus::FullyTyped` module so BF05's JIT tiers
  up immediately on first call.
- Loop shape (`label start` → `load_mem c ptr` → `jmp_if_false c end` →
  body → `jmp start` → `label end`) matches the canonical form expected by
  `ir-to-wasm-compiler` for structured-loop lowering.
