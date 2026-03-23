# Changelog — cli-builder

All notable changes to this package are documented here.

## [1.1.0] — 2026-03-22

### Added

- **Count type** (`"type": "count"`): A new flag type that increments an int64 counter
  on each occurrence. Like boolean flags, count flags consume no value token. `-vvv`
  produces `int64(3)`, `--verbose --verbose` produces `int64(2)`. Absent count flags
  default to `int64(0)`. Count flags work correctly in stacked short flags (e.g., `-avvv`
  sets boolean `a` to true and count `v` to 3).

- **Enum optional values** (`"default_when_present"`): A new field on enum flag
  definitions. When an enum flag with `default_when_present` is used without a value
  (e.g., `--color` instead of `--color=always`), the parser uses `default_when_present`
  as the value. Disambiguation: if the next token is a valid enum value, it is consumed;
  otherwise `default_when_present` is used and the token is re-processed. Spec validation
  ensures `default_when_present` is only used on enum flags and its value is in
  `enum_values`.

- **Flag presence detection** (`ExplicitFlags []string` on `ParseResult`): Tracks which
  flags were explicitly set by the user in argv (not just filled with defaults). The slice
  preserves insertion order and may contain duplicate IDs for repeatable/count flags.
  Enables callers to distinguish "user typed `--color`" from "`--color` was filled with
  its default value".

- **int64 range validation**: Integer values outside `[-2^63, 2^63-1]` now produce a
  specific range error message ("integer value X is out of range") instead of the generic
  "not a valid integer" message.

- Help generator updated: count flags show no value placeholder (like booleans), enum
  flags with `default_when_present` show `[=VALUE]` instead of `<VALUE>`.

- 36 new tests covering all four features.

## [0.3.0] — 2026-03-22

### Added

- `ValidationResult` struct — holds `Valid` bool and `Errors []string` for non-panicking validation.
- `ValidateSpec(specFilePath string) ValidationResult` — validates a JSON spec file on disk,
  returning all errors in the result instead of panicking.
- `ValidateSpecBytes(data []byte) ValidationResult` — validates a JSON spec from raw bytes,
  useful for CI linters, editor plugins, and test harnesses.
- Comprehensive test suite for the validation API covering: valid specs, missing/unsupported
  version, missing required fields, invalid JSON, nonexistent files, flags without name forms,
  and circular requires dependencies.

## [0.2.0] — 2026-03-22

### Changed

- Arguments now use `display_name` instead of `name` for the display label in help text.
  Both fields are accepted for backward compatibility — `display_name` is preferred, with
  `name` as a fallback.

## [0.1.0] — 2026-03-21

### Added

- `LoadSpec` / `LoadSpecFromBytes` — reads, validates, and normalizes a JSON CLI spec
  per the eight validation rules in §6.4.3 of the spec
- `TokenClassifier` — classifies argv tokens into nine typed events (END_OF_FLAGS,
  LONG_FLAG, LONG_FLAG_WITH_VALUE, SINGLE_DASH_LONG, SHORT_FLAG, SHORT_FLAG_WITH_VALUE,
  STACKED_FLAGS, POSITIONAL, UNKNOWN_FLAG) with longest-match-first disambiguation
- `PositionalResolver` — assigns positional tokens to argument slots using the
  last-wins algorithm to support `cp`-style `SOURCE... DEST` patterns
- `FlagValidator` — validates flag constraints: `conflicts_with`, transitive `requires`
  via G_flag TransitiveClosure, `required` flags, `required_unless` exemptions, and
  mutually exclusive group constraints
- `HelpGenerator` — renders help text from the spec for root-level and subcommand help;
  auto-injects `--help`/`--version` builtins
- `Parser` — three-phase parser (routing via DirectedGraph, scanning via
  ModalStateMachine + TokenClassifier, validation via FlagValidator + PositionalResolver)
  supporting GNU, POSIX, subcommand_first, and traditional (tar-style) parsing modes
- `ParseResult`, `HelpResult`, `VersionResult` result types
- `ParseError` / `ParseErrors` error types with 14 error type constants
- `SpecError` error type for load-time spec validation failures
- Fuzzy matching (Levenshtein edit distance ≤ 2) for `unknown_command` and
  `unknown_flag` error suggestions
- Eager type coercion: string, integer (int64), float (float64), boolean, path, file,
  directory, enum
- Full test suite with >90% coverage

### Notes

- The `file` and `directory` types perform filesystem access at parse time per the spec.
  Tests skip these types to avoid environment-dependent failures; they are tested via
  integration through the coerce module.
- Go module uses `replace` directives for `directed-graph` and `state-machine`; these
  are resolved at build time by the monorepo layout.
