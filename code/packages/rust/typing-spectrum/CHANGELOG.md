# Changelog — `typing-spectrum`

## [0.1.0] — 2026-05-04

### Added

**LANG22 PR 11d — typing-spectrum: compilation strategy for the optional-typing spectrum.**

- `mode::CompilationMode` — the five LANG22 compilation modes (TreeWalking, AotNoProfile, AotWithPgo, Jit, JitThenAotWithPgo) with:
  - `recommended_for(tier)` — maps `TypingTier` → best mode
  - `requires_deopt()` — whether the deopt mechanism is needed
  - `requires_profile_input()` / `writes_profile()` — `.ldp` I/O flags
  - `expected_speedup_over_interp(tier)` — ballpark speedup range
  - `Display` impl (lowercase kebab-case)

- `threshold::JitPromotionThreshold` — per-tier JIT call-count thresholds:
  - `FullyTyped` → 0 (compile before first call)
  - `Partial(fraction)` → linear interpolation 10–100
  - `Untyped` → 100
  - `should_promote(call_count)` predicate
  - `label()` human-readable string

- `canonical` — canonical IIR type-name constants and per-language mapping table:
  - All canonical constants: `TYPE_I8` … `TYPE_ANY`, `NUMERIC_TYPES`, `PRIMITIVE_TYPES`, `ALL_CANONICAL_TYPES`
  - `map_frontend_type(frontend_type, language)` — maps Twig, TypeScript, Ruby/Sorbet, Hack, Python/mypy, Rust/C annotations to canonical IIR strings
  - `is_canonical(type_str)` predicate

- `advisory::CompilationAdvisory` — module-level compilation plan:
  - `module_tier` (average typed fraction across all functions)
  - `recommended_mode`
  - `warning_count` (from `iir-type-checker`)
  - `functions: Vec<FunctionAdvisory>` with per-function tier, mode, threshold, typed_fraction
  - `fully_typed_functions()` / `fully_untyped_functions()` subsets
  - `requires_deopt()` predicate
  - `summary()` human-readable multi-line report

- `advisory::advise(module)` — the public entry point; requires the module to have been preprocessed by `iir_type_checker::infer_and_check`.

- 117 tests total: 59 unit (inline), 40 integration (tests/), 18 doctests.
