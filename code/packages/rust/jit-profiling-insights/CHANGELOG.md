# Changelog — jit-profiling-insights

## [0.1.0] — 2026-04-28

### Added

- Initial Rust port of the Python `jit-profiling-insights` package (LANG11).
- `DispatchCost` enum — four-level dispatch overhead classification:
  - `None` (weight 0) — statically typed instruction, no runtime cost.
  - `Guard` (weight 1) — `type_assert` instruction, one inline check.
  - `GenericCall` (weight 10) — `call_runtime` dispatching through a
    `generic_*` helper, order-of-magnitude slower than a direct call.
  - `Deopt` (weight 100) — instruction that has triggered deoptimisation,
    meaning the JIT had to bail out to the interpreter.
- `TypeSite` struct — a single hot instruction whose dispatch overhead is
  worth reporting.  Carries `call_count`, `weight`, and a human-readable
  `savings_description`.  The `impact()` method returns `call_count × weight`
  — the total "pain score".
- `ProfilingReport` struct — the top-level result returned by `analyze()`.
  - `top_n(n)` — the N highest-impact sites.
  - `functions_with_issues()` — deduplicated list of function names.
  - `has_deopts()` — quick check for the worst class of overhead.
  - `format_text()` — human-readable ASCII report.
  - `format_json()` — structured JSON string for tooling.
- `classify_cost(instr)` — maps a single `IIRInstr` to its `DispatchCost`.
  Uses `deopt_anchor.is_some()` as the Rust-idiomatic equivalent of the
  Python `getattr(instr, "deopt_count", 0) > 0` check.
- `find_root_register(instr, fn_)` — traces SSA def-use chains backward
  through `load_mem` / `load_reg` / `const_*` instructions to find the
  original source variable.
- `savings_description(site)` — generates actionable advice for each
  `DispatchCost` tier.
- `rank_sites(sites)` — sorts `TypeSite` slice in-place by
  `(impact desc, weight desc)`.
- `total_instructions(fn_list)` — sums `observation_count` across all
  instructions in all functions.
- `analyze(fn_list, program_name, min_call_count)` — main entry point.
  Scans every instruction in every function, classifies cost, filters by
  `min_call_count`, ranks results, and returns a `ProfilingReport`.
- 43 unit tests covering all public functions and edge cases.

### Notes

- Rust adaptation: `IIRInstr.deopt_anchor: Option<usize>` (the resume
  instruction index) is used as the deopt indicator, since the Rust IR
  struct has no separate `deopt_count` field.  When `deopt_anchor.is_some()`,
  `deopt_count` in the `TypeSite` is set to `observation_count` (a
  conservative upper bound).
- No third-party dependencies beyond `interpreter-ir`.
