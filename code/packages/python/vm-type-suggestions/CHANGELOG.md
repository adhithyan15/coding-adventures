# Changelog — coding-adventures-vm-type-suggestions

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-23

### Added

**Data model (`vm_type_suggestions.types`)**
- `Confidence(str, Enum)` — three-level enum: `CERTAIN` / `MIXED` / `NO_DATA`.
  - `str` mixin enables direct JSON serialisation.
  - `CERTAIN`: one concrete type on every call — safe to annotate.
  - `MIXED`: multiple types observed (polymorphic) — no safe suggestion.
  - `NO_DATA`: profiler never reached this parameter.
- `ParamSuggestion` — one parameter's observation.
  - Fields: `function`, `param_name`, `param_index`, `observed_type`,
    `call_count`, `confidence`, `suggestion`.
  - `.to_dict()` — JSON-serialisable dict.
- `SuggestionReport` — top-level result of `suggest()`.
  - `.actionable()` — returns only `CERTAIN` suggestions.
  - `.by_function()` — groups suggestions by function name, order-preserving.
  - `.format_text()` — human-readable terminal output with emoji markers
    (✅ CERTAIN, ⚠️ MIXED, ℹ️ NO_DATA).
  - `.format_json()` — pretty-printed JSON (2-space indentation).

**Suggestion engine (`vm_type_suggestions.suggest`)**
- `suggest(fn_list, *, program_name) → SuggestionReport` — the main entry point.
  - For each untyped function parameter (`type_hint == "any"`), finds the
    corresponding `load_mem [arg[N]]` instruction and reads its `observed_type`
    and `observation_count`.
  - Classifies each parameter as CERTAIN / MIXED / NO_DATA.
  - Already-typed parameters are silently skipped.
- `_find_arg_loaders(fn)` — scans a function's instructions for `load_mem`
  instructions matching the `"arg[N]"` source pattern; returns first-match
  per index (highest observation count, always the first load in the body).
  - Handles edge cases: non-string srcs, invalid index syntax (non-integer),
    out-of-range indices, empty srcs, non-load_mem instructions with `"arg[N]"` srcs.
- `total_calls` in `SuggestionReport` counts only `CERTAIN` observations —
  MIXED and NO_DATA do not contribute.

**Package surface (`vm_type_suggestions.__init__`)**
- Public exports: `suggest`, `Confidence`, `ParamSuggestion`, `SuggestionReport`.

### Tests

- 65 unit and integration tests across 2 test modules.
- `tests/conftest.py` — fixtures: `add_fn`, `fibonacci_fn`, `mixed_fn`,
  `never_called_fn`, `typed_fn`, `no_loader_fn`.
  Helper factories `make_load_mem()`, `make_typed_load_mem()`, `make_function()`.
- `tests/test_types.py` — 34 tests: `Confidence` values / JSON; `ParamSuggestion.to_dict()`
  for all three confidence levels; `SuggestionReport` actionable / by_function /
  format_text (all markers, summary, singular noun, zero-count) / format_json.
- `tests/test_suggest.py` — 31 tests: empty fn_list, zero-param function, no-instruction
  function, single CERTAIN param, two CERTAIN params, total_calls accumulation, MIXED
  classification, NO_DATA (never called, no loader), typed params skipped, mixed typed/
  untyped, multi-function, all-confidence-levels, edge cases (non-string src, non-arg src,
  invalid index, duplicate loaders, out-of-range index, empty srcs, non-load_mem with
  arg-pattern src).
