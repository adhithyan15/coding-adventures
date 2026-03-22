# Changelog

All notable changes to the `cli-builder` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.1.0] — 2026-03-22

### Added

- **Count flag type** (`"type": "count"`) — a new flag type that consumes no
  value token. Each occurrence increments a counter: `-vvv` produces 3,
  `--verbose --verbose` produces 2. Absent count flags default to 0. In stacked
  short flags, each character increments independently.

- **Enum optional values** (`default_when_present` field on `FlagDef`) — when
  an enum flag has this field set and the user provides the flag without a value
  (e.g., `--color` instead of `--color=always`), `default_when_present` is used
  as the value. When the flag is used with `=value` syntax, the explicit value
  takes precedence. The token classifier treats such flags as boolean-like (no
  value consumption), and the parser uses `default_when_present` as the fallback.
  Validated at spec load time: must be `"enum"` type, value must be in
  `enum_values`.

- **Flag presence detection** (`explicit_flags: Vec<String>` on `ParseResult`)
  — tracks which flags were explicitly set by the user on the command line.
  Every time a flag token is consumed from argv, its ID is appended. A flag
  that appears multiple times appears multiple times in the list (e.g., `-vvv`
  adds `"verbose"` three times for a count flag).

- **int64 range validation** — improved error messages for integer values that
  are out of the i64 range. Rust's `i64::parse()` already validates the range,
  but the error message now distinguishes "looks numeric but out of range" from
  "not a valid integer at all."

- **Help output for count flags** — count flags display with no value
  placeholder (like boolean flags).

- **Help output for enum flags with `default_when_present`** — these flags
  display `[=VALUE]` instead of `<VALUE>` to indicate the value is optional.

- 30+ new tests covering all four v1.1 features: count flag stacking, count
  defaults, `default_when_present` bare flag vs `=value`, `explicit_flags`
  tracking for boolean/count/value/stacked/repeatable flags, integer range
  errors, and spec validation of `default_when_present`.

### Changed

- `FlagDef` struct gains `default_when_present: Option<String>` field
  (serde-optional, backward compatible).
- `ParseResult` struct gains `explicit_flags: Vec<String>` field.
- `FlagInfo` (token classifier) gains `is_count` and `has_default_when_present`
  fields. `is_boolean` is now `true` for count flags and enum flags with
  `default_when_present` (since they consume no value token).
- `coerce_value` for `"integer"` type now provides a more descriptive error
  message when the value is numeric but out of i64 range.

## [0.3.0] — 2026-03-22

### Added

- **`validate`** — standalone spec validation module with `ValidationResult`,
  `validate_spec_str()`, and `validate_spec_file()`. Returns a simple
  `{ valid, errors }` struct instead of `Result`, making it convenient for
  CI linting, editor integration, and dry-run workflows. Re-exported at the
  crate root for ergonomic access.

## [0.2.0] — 2026-03-22

### Changed

- Arguments now use `display_name` instead of `name` for the display label in help text.
  Both fields are accepted for backward compatibility — `display_name` is preferred, with
  `name` as a fallback. In the `ArgumentDef` struct, the field is renamed from `name` to
  `display_name` with `#[serde(alias = "name")]` for deserialization compatibility.

## [0.1.0] — 2026-03-21

### Added

- **`spec_loader`** — load and validate JSON CLI specs from strings or files.
  - Validates `cli_builder_spec_version` is `"1.0"`.
  - Detects duplicate flag IDs, argument IDs, command IDs within each scope.
  - Verifies each flag has at least one form (`short`, `long`, or `single_dash_long`).
  - Validates `conflicts_with`, `requires`, and `required_unless` cross-references.
  - Verifies `enum_values` non-empty for `type: "enum"` flags and arguments.
  - Enforces at most one variadic argument per scope.
  - Detects cycles in the flag dependency graph G_flag using `directed_graph::Graph::has_cycle()`.

- **`types`** — serde-deserializable schema types for the full spec format:
  `CliSpec`, `FlagDef`, `ArgumentDef`, `CommandDef`, `ExclusiveGroup`, `BuiltinFlags`.
  Output types: `ParseResult`, `HelpResult`, `VersionResult`, `ParserOutput`.

- **`errors`** — error hierarchy:
  - `ParseError` — individual argv error with `error_type`, `message`, `suggestion`, `context`.
  - `ParseErrors` — collected list of `ParseError` objects, implements `std::error::Error`.
  - `CliBuilderError` — top-level error wrapping spec errors, parse errors, IO errors, JSON errors.

- **`token_classifier`** — token classification DFA (§5):
  - Classifies argv tokens into `TokenEvent` variants:
    `EndOfFlags`, `LongFlag`, `LongFlagWithValue`, `SingleDashLong`,
    `ShortFlag`, `ShortFlagWithValue`, `StackedFlags`, `Positional`, `UnknownFlag`.
  - Implements longest-match-first disambiguation (§5.2):
    single-dash-long flags checked before char stacking.
  - Traditional mode support (§5.3): first token can be a dash-less flag stack.
  - Built-in `--help` and `--version` always recognized.

- **`positional_resolver`** — assigns positional tokens to argument definitions (§6.4.1):
  - Handles no-variadic (one-to-one) and variadic (partition around variadic argument) cases.
  - Implements the `cp SOURCE... DEST` trailing-dest pattern correctly.
  - Validates `variadic_min`/`variadic_max` bounds.
  - Respects `required_unless_flag` for conditional optional arguments.
  - Applies default values for absent optional arguments.
  - Type coercion: `integer`, `float`, `string`, `path`, `file`, `directory`, `enum`, `boolean`.
  - Filesystem existence checks for `file` and `directory` types.

- **`flag_validator`** — flag constraint validation (§6.4.2):
  - `conflicts_with`: detects pairwise flag conflicts (reported once per pair).
  - `requires` (transitive): uses `directed_graph::Graph::transitive_closure()` to find all transitively required flags.
  - `required`: reports missing required flags (with `required_unless` exemption).
  - Mutually exclusive groups: validates `at_most_one` and `required` exactly-one semantics.

- **`help_generator`** — generates formatted help text from the spec (§9):
  - `generate_root_help(spec)` — USAGE, DESCRIPTION, COMMANDS, OPTIONS, ARGUMENTS, GLOBAL OPTIONS sections.
  - `generate_command_help(spec, path)` — help for a specific subcommand.
  - Formats flag signatures: `-s, --long <VALUE>` or `-s, --long` for boolean.
  - Formats argument signatures: `<NAME>`, `[NAME]`, `<NAME...>`, `[NAME...]`.
  - Shows default values as `[default: X]`.
  - Injects builtin `--help` / `--version` stubs into GLOBAL OPTIONS.

- **`parser`** — the three-phase CLI parser (§6):
  - Phase 1 (routing): walks argv via `directed_graph::Graph` successors to find the deepest matching command node. Skips flags and resolves command aliases to canonical names.
  - Phase 2 (scanning): re-walks argv using `state_machine::ModalStateMachine` with `SCANNING`, `FLAG_VALUE`, `END_OF_FLAGS` modes. Dispatches token events from the classifier.
  - Phase 3 (validation): positional resolution + flag constraint validation. Collects all errors before returning.
  - Handles `--help` / `-h` early-return as `HelpResult`.
  - Handles `--version` early-return as `VersionResult`.
  - Levenshtein fuzzy matching (edit distance ≤ 2) for `unknown_flag` suggestions.
  - Repeatable flag accumulation into JSON arrays.
  - Duplicate non-repeatable flag detection.
  - POSIX mode: first positional token terminates flag scanning.
  - Traditional mode: first token may be a dash-less flag stack.
  - Global flags included in active flag set when `inherit_global_flags` is true.
  - Populates default values for absent optional flags in `ParseResult`.

- **Tests** — over 120 tests across inline unit tests and four external test files:
  - `tests/spec_loader_tests.rs` — spec validation coverage.
  - `tests/token_classifier_tests.rs` — token classification for all token types.
  - `tests/parser_tests.rs` — full integration tests for echo, ls, cp, grep, tar, git, head.
  - `tests/help_generator_tests.rs` — help text structure and content.

### Architecture notes

- Depends on `directed-graph` (path dep) for G_cmd routing and G_flag cycle detection / transitive closure.
- Depends on `state-machine` (path dep) for `ModalStateMachine` (parse mode tracking) and `DFA` (used as trivial sub-machines inside the modal machine).
- Uses `serde` + `serde_json` for JSON deserialization and typed output values.
- Zero use of `.unwrap()` in non-test production code; all errors propagated via `Result`.

### Divergences from spec

- **`-h` builtin takes precedence over user-defined `-h` short flag**: in `ls`, the spec defines `-h` as `human-readable`. When `builtin_flags.help` is true, the scanner intercepts `-h` before looking up user flags. Users can disable this with `"builtin_flags": {"help": false}`.
- **Routing uses conservative flag skipping**: during Phase 1, the parser cannot always know whether a flag is boolean (it doesn't yet have the active flag set resolved). It skips flags conservatively. Phase 2 re-reads all tokens, so no tokens are lost.
