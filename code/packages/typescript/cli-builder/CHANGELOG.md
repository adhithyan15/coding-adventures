# Changelog — @coding-adventures/cli-builder

All notable changes to this package are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-03-21

### Added

- **`SpecLoader`** — reads and validates CLI Builder JSON spec files (v1.0).
  Validates all required fields, cross-references (conflicts_with, requires,
  mutually_exclusive_groups), enum values, variadic argument counts. Builds
  the flag dependency graph (G_flag) using `@coding-adventures/directed-graph`
  and calls `hasCycle()` to reject circular `requires` dependencies.

- **`TokenClassifier`** — classifies argv tokens into typed events:
  `END_OF_FLAGS`, `LONG_FLAG`, `LONG_FLAG_WITH_VALUE`, `SINGLE_DASH_LONG`,
  `SHORT_FLAG`, `SHORT_FLAG_WITH_VALUE`, `STACKED_FLAGS`, `POSITIONAL`,
  `UNKNOWN_FLAG`. Implements longest-match-first disambiguation for
  `single_dash_long` flags (e.g., `-classpath` is never decomposed as stacked
  short flags when declared as a single-dash-long flag).

- **`PositionalResolver`** — implements §6.4.1 partitioning algorithm: assigns
  a flat list of positional tokens to named argument slots. Handles the
  "last-wins" pattern for variadic arguments with required trailing arguments
  (the `cp SOURCE... DEST` pattern).

- **`FlagValidator`** — validates §6.4.2 flag constraints after scanning:
  `conflicts_with` (bilateral pairwise), `requires` (transitive via G_flag
  using `transitiveClosure`), `required` flags with `required_unless` exemption,
  and `mutually_exclusive_groups` (both violation and missing required group).

- **`HelpGenerator`** — auto-generates formatted help text from the spec.
  Supports root help and scoped subcommand help. Sections: USAGE, DESCRIPTION,
  COMMANDS, OPTIONS, ARGUMENTS, GLOBAL OPTIONS. Formats flag signatures with
  value names, appends `[default: X]` for flags with defaults.

- **`Parser`** — the main entry point. Implements the three-phase algorithm:
  Phase 1 (routing via command graph traversal), Phase 2 (scanning via
  `ModalStateMachine` from `@coding-adventures/state-machine`), Phase 3
  (validation). Returns `ParseResult`, `HelpResult`, or `VersionResult`.
  Collects all errors and throws `ParseErrors` on failure.

- **Error hierarchy**: `CliBuilderError` → `SpecError` (fatal load-time) and
  `ParseErrors` (runtime, holds `ParseError[]` for multi-error reporting).

- **Type system** (`types.ts`): `CliSpec`, `CommandDef`, `FlagDef`, `ArgDef`,
  `ExclusiveGroup`, `ParseResult`, `HelpResult`, `VersionResult`, `ParserResult`,
  `ValueType`, `ParsingMode`.

- **All four parsing modes** implemented:
  - `gnu` (default): flags anywhere, `--` ends flag scanning
  - `posix`: first non-flag positional ends flag scanning
  - `subcommand_first`: first non-flag token is always a subcommand
  - `traditional`: first token without `-` treated as stacked flag chars (tar-style)

- **Type coercion**: integer and float values coerced at parse time, not returned
  as raw strings. Enum values validated against `enum_values`.

- **Fuzzy matching**: `unknown_command` and `unknown_flag` errors include a
  `suggestion` field when a valid token has Levenshtein distance ≤ 2.

- **Repeatable flags**: flags marked `repeatable: true` accumulate into arrays.
  Non-repeatable flags produce a `duplicate_flag` error on second occurrence.

- **Test suite**: 90%+ coverage target. Four test files covering SpecLoader,
  TokenClassifier, HelpGenerator, and Parser integration tests using embedded
  JSON specs for echo, ls, cp, grep, tar, git, and java.

### Dependencies

- `@coding-adventures/state-machine`: `file:../state-machine` — provides
  `ModalStateMachine`, `DFA`, and `transitionKey` for parse mode tracking.
  Transitively provides `@coding-adventures/directed-graph` for graph operations
  (cycle detection, transitive closure).
