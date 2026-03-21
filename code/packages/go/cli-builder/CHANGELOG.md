# Changelog — cli-builder

All notable changes to this package are documented here.

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
