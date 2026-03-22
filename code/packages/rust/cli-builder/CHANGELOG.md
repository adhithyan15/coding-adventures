# Changelog

All notable changes to the `cli-builder` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

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
