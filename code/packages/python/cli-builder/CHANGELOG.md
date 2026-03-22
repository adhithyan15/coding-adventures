# Changelog

All notable changes to `coding-adventures-cli-builder` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.3.0] — 2026-03-22

### Added

- **`validate_spec(spec_path)`**: Standalone function that validates a CLI Builder JSON
  spec file and returns a `ValidationResult` instead of raising exceptions. Ideal for
  linters, CI checks, and editor integrations.

- **`validate_spec_string(json_string)`**: Same as `validate_spec()`, but accepts an
  in-memory JSON string. Useful for testing and programmatic spec generation.

- **`ValidationResult`** dataclass with `valid: bool` and `errors: list[str]` fields.
  Added to `cli_builder.types` and exported from the top-level package.

- New test module `tests/test_validate.py` with comprehensive coverage of both
  validation functions and the `ValidationResult` dataclass.

## [0.2.0] — 2026-03-22

### Changed

- Arguments now use `display_name` instead of `name` for the display label in help text.
  Both fields are accepted for backward compatibility — `display_name` is preferred, with
  `name` as a fallback.

## [0.1.0] — 2026-03-21

### Added

- **`SpecLoader`**: JSON spec validator with full graph-based constraint checking.
  Validates field presence, unique IDs, flag cross-references, `enum_values`
  requirements, variadic-arg-per-scope limits, and circular `requires` detection
  via `DirectedGraph.has_cycle`.

- **`TokenClassifier`**: Longest-match-first token classification with support for
  all token types: `end_of_flags`, `long_flag`, `long_flag_with_value`,
  `single_dash_long`, `short_flag`, `short_flag_with_value`, `stacked_flags`,
  `positional`, and `unknown_flag`. Implements spec §5.2 disambiguation rules.

- **`PositionalResolver`**: Last-wins positional assignment algorithm from spec §6.4.1.
  Handles leading/variadic/trailing argument partitioning for `cp`/`mv`-style CLIs.
  Performs per-token type coercion.

- **`FlagValidator`**: Full constraint validation: `conflicts_with`, transitive
  `requires` (via `DirectedGraph.transitive_closure`), `required` flag checks,
  `required_unless` exemptions, `mutually_exclusive_groups`, and `duplicate_flag`
  detection.

- **`HelpGenerator`**: Auto-generates USAGE / DESCRIPTION / COMMANDS / OPTIONS /
  GLOBAL OPTIONS / ARGUMENTS sections per spec §9 formatting rules.

- **`Parser`**: Three-phase parse engine using `ModalStateMachine` + `DirectedGraph`.
  Phase 1: command routing. Phase 2: token scanning. Phase 3: constraint validation.
  Supports `gnu`, `posix`, `subcommand_first`, and `traditional` (tar-style) parsing
  modes. Returns `ParseResult`, `HelpResult`, or `VersionResult`.

- **Error types**: `CliBuilderError`, `SpecError`, `ParseError` dataclass, and
  `ParseErrors` exception with formatted multi-error rendering.

- **Type system**: Full coercion for `boolean`, `string`, `integer`, `float`,
  `path`, `file`, `directory`, and `enum` types.

- **Fuzzy matching**: Levenshtein distance suggestions for unknown flags and commands
  (suggestion shown when edit distance ≤ 2).

- Comprehensive test suite with >95% coverage across all modules.
