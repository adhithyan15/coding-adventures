# Changelog — coding_adventures_cli_builder

All notable changes to this package are documented here.

## [0.3.0] — 2026-03-22

### Added

- `Validator` module with standalone, non-raising spec validation:
  - `validate_spec/1` — validate a spec file on disk, returns `%{valid, errors}`.
  - `validate_spec_string/1` — validate a spec from a JSON string.
- 15 tests covering valid specs, invalid JSON, missing version, unsupported
  version, missing required fields, flag with no short/long, nonexistent file,
  invalid parsing mode, and missing flag type.

## [0.2.0] — 2026-03-22

### Changed

- Arguments now use `display_name` instead of `name` for the display label in help text.
  Both fields are accepted for backward compatibility — `display_name` is preferred, with
  `name` as a fallback.

## [0.1.0] — 2026-03-21

### Added

- Initial implementation of the CLI Builder spec v1.0.

#### `SpecLoader`
- `load!/1` — reads, parses (via Jason), and validates a JSON spec file.
- `load_from_string!/1` — validates a spec from a JSON string (for tests).
- Full structural validation per §6.4.3: required fields, type checks, enum
  consistency, duplicate ID detection, single-variadic-per-scope rule.
- Cross-reference validation for `conflicts_with`, `requires`, and exclusive
  group `flag_ids`.
- Cycle detection in each scope's flag dependency graph (G_flag) using
  `CodingAdventures.DirectedGraph.Graph.has_cycle?/1`.
- Raises `SpecError` (a `defexception`) on any violation.

#### `TokenClassifier`
- `classify/2` — classifies one argv token given the active flags.
- Handles all token types: `:end_of_flags`, `{:long_flag, n}`,
  `{:long_flag_with_value, n, v}`, `{:single_dash_long, n}`,
  `{:short_flag, c}`, `{:short_flag_with_value, c, v}`,
  `{:stacked_flags, [c]}`, `{:positional, v}`, `{:unknown_flag, t}`.
- Longest-match-first disambiguation: SDL > short > stacked.

#### `PositionalResolver`
- `resolve/4` — assigns positional tokens to argument definition slots.
- Last-wins partition algorithm for variadic arguments (§6.4.1).
- Handles leading/variadic/trailing argument slices.
- Type coercion via `coerce/2` for all types: boolean, string, integer, float,
  path, file, directory, enum.
- `required_unless_flag` exemption support.

#### `FlagValidator`
- `validate/4` — validates parsed flags against all constraint types.
- `conflicts_with`: bilateral conflict detection (de-duplicated).
- `requires` transitive closure via `Graph.transitive_closure/2`.
- `required` flag checking with `required_unless` exemption.
- Mutually exclusive group checking (violation and missing-required).

#### `HelpGenerator`
- `generate/2` — generates help text from a spec and command path.
- Sections: USAGE, DESCRIPTION, COMMANDS, OPTIONS, ARGUMENTS, GLOBAL OPTIONS.
- Argument formatting: `<NAME>`, `[NAME]`, `<NAME>...`, `[NAME...]`.
- Flag formatting with value name, default, and required annotations.
- Builtin `--help` and `--version` appear in GLOBAL OPTIONS.

#### `Parser`
- `parse/2` — main entry point; reads spec file and parses argv.
- `parse_string/2` — parses from an embedded JSON string.
- **Phase 1 Routing**: builds command_path by walking the spec's `commands`
  tree; skips flags to avoid misidentifying flag values as subcommands.
- **Phase 2 Scanning**: modal state machine with three modes (`:scanning`,
  `:flag_value`, `:end_of_flags`); handles all token types from TokenClassifier.
- **Phase 3 Validation**: PositionalResolver + FlagValidator + enum validation.
- GNU, POSIX, traditional, and subcommand_first parsing modes.
- Traditional mode (tar-style): `argv[0]` without leading dash treated as
  stacked short flags if all chars are known boolean shorts.
- Fuzzy flag suggestion via Levenshtein distance ≤ 2.
- Returns `{:ok, ParseResult}`, `{:ok, HelpResult}`, `{:ok, VersionResult}`,
  or `{:error, ParseErrors}`.
- Automatic argv[0] stripping when it matches the spec's program name.

#### Error types
- `ParseError` struct with `error_type`, `message`, `suggestion`, `context`.
- `ParseErrors` exception with `errors` list and joined `message`.
- `SpecError` exception for spec-load-time failures.

#### Result types
- `ParseResult` — flags map + arguments map + program name + command_path.
- `HelpResult` — rendered help text + command_path.
- `VersionResult` — version string from spec.

### Dependencies

- `coding_adventures_directed_graph` (local path) — Graph for G_flag cycle
  detection and transitive closure; LabeledGraph for Modal machine internals.
- `coding_adventures_state_machine` (local path) — DFA and ModalStateMachine
  for parse mode tracking.
- `jason ~> 1.4` — JSON parsing of spec files.
