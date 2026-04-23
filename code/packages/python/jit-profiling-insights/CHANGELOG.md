# Changelog — coding-adventures-jit-profiling-insights

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-23

### Added

**Data model (`jit_profiling_insights.types`)**
- `DispatchCost(str, Enum)` — four-level enum: `NONE` / `GUARD` / `GENERIC_CALL` / `DEOPT`.
  - `str` mixin enables direct JSON serialisation without a custom encoder.
  - `.weight` property returns the cost multiplier (0 / 1 / 10 / 100) used by the ranking formula.
- `TypeSite` — one instruction-level hotspot identified by the insight pass.
  - Fields: `function`, `instruction_op`, `source_register`, `observed_type`, `type_hint`,
    `dispatch_cost`, `call_count`, `deopt_count`, `savings_description`.
  - `.impact` property: `call_count × dispatch_cost.weight` — the ranking key.
  - `.to_dict()` — JSON-serialisable dict including the computed `impact`.
- `ProfilingReport` — top-level result of `analyze()`.
  - `.top_n(n=10)` — first *n* sites by impact.
  - `.functions_with_issues()` — deduplicated, order-preserving list of function names.
  - `.has_deopts()` — True if any DEOPT-level site exists.
  - `.format_text()` — human-readable terminal output with emoji severity icons
    (🚨 CRITICAL, 🔴 HIGH IMPACT, 🟡 MEDIUM IMPACT, 🟢 LOW IMPACT, ✅ clean).
  - `.format_json()` — pretty-printed JSON (2-space indentation) for tooling consumers.

**Classification pass (`jit_profiling_insights.classify`)**
- `_classify_cost(instr)` — classifies an `IIRInstr` into a `DispatchCost` level:
  - `type_hint != "any"` → `NONE`
  - `op == "type_assert"` → `GUARD`
  - `op == "call_runtime"` and `"generic_" in srcs[0]` → `GENERIC_CALL`
  - `observation_count > 0 and deopt_count > 0` → `DEOPT`
  - Uses `getattr(instr, "deopt_count", 0)` for forward compatibility with future
    interpreter-ir versions that add per-instruction deopt counters.
- `_find_root_register(instr, instructions, instr_index)` — traces the SSA data-flow
  chain backward through `load_mem` / `load_reg` / `const` edges to find the furthest-back
  register whose `type_hint == "any"` caused the guard or generic dispatch.
  - Bounded by `instr_index` (never looks past the current instruction — SSA invariant).
  - Cycle-safe via a `visited` set.
  - Falls back to the primary source operand when the chain terminates.
- `_savings_description(cost, call_count, op)` — generates a one-sentence human-readable
  description of what adding a type annotation would eliminate.

**Ranking pass (`jit_profiling_insights.rank`)**
- `rank_sites(sites)` — sorts a `TypeSite` list in-place by descending impact.
  - Tie-breaks by `dispatch_cost.weight` (descending) so DEOPT ties beat GENERIC_CALL
    ties beat GUARD ties.
  - Returns the sorted list for call-chaining.
- `total_instructions(fn_list)` — sums `observation_count` across all instructions in
  all functions; used as `ProfilingReport.total_instructions_executed`.

**Analysis entry point (`jit_profiling_insights.analyze`)**
- `analyze(fn_list, *, program_name, min_call_count) → ProfilingReport` — the main
  public function.  Runs the four-step pipeline:
  1. Compute total instruction count.
  2. Scan every observed instruction (`observation_count >= min_call_count`).
  3. Classify cost and find root register for every non-NONE site.
  4. Rank and wrap in `ProfilingReport`.
- `min_call_count=1` (default) includes all observed instructions; higher values
  filter out rarely-executed noise for CI / performance-gate use cases.

**Package surface (`jit_profiling_insights.__init__`)**
- Public exports: `analyze`, `DispatchCost`, `ProfilingReport`, `TypeSite`.

### Tests

- 107 unit and integration tests across 4 test modules; **100% line coverage** target.
- `tests/conftest.py` — shared fixtures: `guard_instr`, `generic_call_instr`,
  `typed_instr`, `unobserved_instr`, `deopt_instr`, `fibonacci_fn`, `main_fn`.
  Helper factories `make_instr()` and `make_function()` build real `IIRInstr` /
  `IIRFunction` objects (not mocks).
- `tests/test_types.py` — 44 tests: `DispatchCost` values / weights / JSON serialisation;
  `TypeSite` impact formula and `to_dict()`; `ProfilingReport` `top_n`, `functions_with_issues`,
  `has_deopts`, `format_text` (severity icons, percentage, deopt section, summary line,
  zero-total-instructions edge case), `format_json` structure.
- `tests/test_classify.py` — 29 tests: every `_classify_cost` branch (NONE / GUARD /
  GENERIC_CALL / DEOPT) including edge cases (empty srcs, non-string src, missing
  `deopt_count` attribute); `_find_root_register` chain tracing, cycle detection,
  index boundary, and literal fallback; `_savings_description` all four costs.
- `tests/test_rank.py` — 14 tests: `rank_sites` ordering, tie-breaking, in-place
  mutation; `total_instructions` sum across functions, unobserved instructions.
- `tests/test_analyze.py` — 20 tests: empty fn_list, all-typed functions (no sites),
  fibonacci fixture end-to-end (guard detection, ranking, root register, total count,
  `format_text` / `format_json`), multi-function cross-function ranking, `min_call_count`
  filtering, DEOPT site classification.
