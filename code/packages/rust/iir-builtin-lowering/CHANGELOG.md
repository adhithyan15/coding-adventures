# Changelog — iir-builtin-lowering

All notable changes to this crate are documented here.

---

## [0.2.0] — 2026-05-11

### Added (LANG32 — Global Variables and I/O Phase 3 lowering)

#### New `src/global_io.rs` module

Phase 3 of the builtin-lowering pipeline rewrites three `call_builtin` opcodes
to typed IIR opcodes that all four native backends (`iir-to-beam`,
`iir-to-wasm`, `iir-to-jvm-class-file`, `iir-to-cil-bytecode`) understand
directly.

**Look-back lowering algorithm**

The twig-ir-compiler encodes global variable names as string-as-Var `const`
instructions (`const %n1 = Var("x")`), then passes the register to
`call_builtin "global_set"`.  The Phase 3 pass runs two sub-passes per
function:

1. **Pass 1** — build `const_str_map: HashMap<register, literal_text>` for
   every `const` instruction whose `srcs[0]` is `Operand::Var(text)`.
2. **Pass 2** — rewrite each `call_builtin "global_set"/%"global_get"/%"print"`
   using the resolved name from the map:
   - `call_builtin "global_set", %n, %v` → `global_store Str("name"), Var(%v)`
   - `call_builtin "global_get", %n` → `global_load Str("name")`
   - `call_builtin "print", %v` → `io_out Var(%v)`

Unresolvable instructions (name register not in const_str_map, missing srcs)
are left as `call_builtin` so the backend validator can surface a clear error.

**Exported entry points**

- `lower_global_io_function(fn_: &mut IIRFunction)` — single-function entry point.
- `lower_global_io(module: &mut IIRModule)` — whole-module entry point, wired
  into `lower_builtins()` as Phase 3.

**Tests** — 22 new tests in `src/global_io.rs`:

- `global_set` rewrites with resolvable and unresolvable name registers.
- `global_get` rewrites with resolvable and unresolvable name registers.
- `print` is always rewritten (no look-back needed).
- Multiple globals in one function.
- `call_builtin` for unknown builtins left unchanged.
- Non-`call_builtin` instructions left unchanged.
- Multiple functions in one module.
- Empty function / empty module edge cases.
- Type hints and profiling fields preserved through rewrite.

#### `src/lib.rs` changes

- `pub mod global_io;` added.
- `pub use global_io::lower_global_io;` re-exported from crate root.
- `lower_builtins()` now calls `global_io::lower_global_io(module)` as Phase 3,
  after Phase 1 (numeric) and Phase 2 (heap).

---

## [0.1.0] — 2026-05-11

### Added

- Initial release: Phase 1 numeric builtin lowering pass (LANG31 §1.1).
- `lower_builtins(module: &mut IIRModule) -> Vec<BuiltinLoweringError>` —
  mutating entry point.
- `lower_builtins_cloned(module: &IIRModule) -> (IIRModule, Vec<BuiltinLoweringError>)` —
  non-destructive entry point that preserves the original.
- `lower_builtins_checked(module: &mut IIRModule) -> Result<(), Vec<BuiltinLoweringError>>` —
  convenience wrapper that returns `Err` on any error.
- `BuiltinLoweringError` enum with two variants:
  - `WrongArity` — emitted when a numeric builtin is called with the wrong
    number of arguments.
  - `UntypedBuiltin` — emitted when a numeric builtin's `type_hint` is still
    `"any"`, indicating the pipeline ordering is wrong.
- `src/numeric.rs` — the 18-entry lowering table and in-place instruction
  rewrite logic.
- `src/error.rs` — `BuiltinLoweringError` enum and `Display` / `Error` impls.
- `src/lower.rs` — original simple lowering pass (no arity/type checking),
  kept for backward compatibility.
- `tests/test_lowering.rs` — 50 comprehensive tests covering:
  - All 18 numeric builtins (add, sub, mul, div, mod, neg, cmp_eq, cmp_ne,
    cmp_lt, cmp_le, cmp_gt, cmp_ge, and, or, not, shl, shr, xor).
  - Binary op invariants: dest preserved, srcs stripped, type_hint preserved.
  - Unary op invariants (neg, not).
  - Unknown builtins left unchanged.
  - Non-call_builtin instructions left unchanged.
  - `may_alloc` cleared after lowering.
  - WrongArity and UntypedBuiltin error cases.
  - Multi-function modules.
  - Empty modules and empty functions.
  - Mixed call_builtin / non-call_builtin instruction streams.
  - `lower_builtins_cloned` preserves original.
  - `lower_builtins_checked` returns Ok/Err correctly.
  - Profiling fields (observation_count, observed_type, ic_slot) preserved.
  - Multiple errors accumulated across functions.

### Not yet implemented (Phase 2)

- `src/heap.rs` — heap builtin lowering (`"cons"`, `"car"`, `"cdr"`,
  `"null?"`, `"pair?"`) is tracked in LANG31 Phase 2.
