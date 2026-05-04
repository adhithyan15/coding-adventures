# Changelog — vm-type-suggestions

## [0.1.0] — 2026-04-28

### Added

- Initial Rust port of the Python `vm-type-suggestions` package (LANG12).
- `Confidence` enum — three-level certainty rating for a type suggestion:
  - `Certain` — all observed calls used the same type; annotation is safe.
  - `Mixed` — the VM observed `"polymorphic"` values; annotation would
    over-specialise; no suggestion is emitted.
  - `NoData` — no calls were observed (or no `load_mem` instruction was
    found); cannot make a recommendation.
- `ParamSuggestion` struct — one parameter's full observation record:
  `function`, `param_name`, `param_index`, `observed_type`, `call_count`,
  `confidence`, and an optional `suggestion` string.
- `SuggestionReport` struct — the top-level result returned by `suggest()`:
  - `actionable()` — iterator over only `Certain` suggestions.
  - `by_function()` — suggestions grouped by function name, preserving
    insertion order (returned as `Vec<(&str, Vec<&ParamSuggestion>)>`).
  - `format_text()` — human-readable ASCII report.
  - `format_json()` — structured JSON string for tooling.
- `suggest(fn_list, program_name)` — main entry point.  For each untyped
  parameter (`type_hint == "any"`), finds the corresponding
  `load_mem arg[N]` instruction, classifies the observation, and emits a
  `ParamSuggestion` with a `"declare 'param: type'"` suggestion string for
  `Certain` cases.
- `find_arg_loaders(fn_)` — scans all instructions in a function for
  `load_mem` whose first source operand matches `"arg[N]"`, returning the
  first match for each index.
- 21 unit tests covering all paths: empty input, unobserved parameters,
  polymorphic parameters, certain suggestions, typed-param skipping,
  multi-param and multi-function scenarios, and missing `load_mem`.

### Notes

- `by_function()` returns `Vec<(&str, Vec<&ParamSuggestion>)>` rather than
  a `HashMap` to preserve insertion order without requiring an external crate.
- No third-party dependencies beyond `interpreter-ir`.
