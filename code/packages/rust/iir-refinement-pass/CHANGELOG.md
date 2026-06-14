# Changelog — `iir-refinement-pass`

## [0.1.0] — 2026-05-13

### Added (LANG42)

- Initial release. Implements the pre-codegen refinement obligation pass for
  the Twig AOT pipeline.
- `check_module(module, mode) -> Vec<RefinementError>` — the single public
  entry point. Scans every function for call-site and return-site refinement
  violations.
- `RefinementMode::Lenient` (default) — only `ProvenUnsafe` outcomes become
  compile errors; `Unknown` outcomes are silently accepted (future LANG46 will
  insert runtime checks).
- `RefinementMode::Strict` — both `ProvenUnsafe` and `Unknown` outcomes become
  compile errors, used for `(typed strict)` modules (TW05-A).
- `RefinementError` struct with `function`, `site`, `counter_example`, and
  `description` fields; implements `Display` with the `error[E0042]` prefix.
- `const_prop::build_const_map` — single-pass forward scan that collects
  integer-literal `const` assignments into a `HashMap<String, i128>`, with
  conservative eviction on duplicate writes.
- `call_checker::check_calls` — checks each `call` instruction's arguments
  against the callee's `param_refinements` using resolved evidence.
- `ret_checker::check_returns` — checks each `ret` instruction against the
  function's `return_refinement` using resolved evidence.
- Evidence resolution: `Operand::Int(v)` → `Concrete`, `Operand::Var` in
  ConstMap → `Concrete`, everything else → `Unconstrained`.
- 30+ unit tests across all four source files, including all 10 tests
  specified in the LANG42 spec.
