# Changelog — brainfuck-iir-compiler

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
