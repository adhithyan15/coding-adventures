# Changelog

All notable changes to the `coding_adventures_cli_builder` gem will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- **SpecLoader**: Reads and validates a CLI Builder JSON spec file. Enforces all 9 validation rules from the spec (version check, required fields, duplicate IDs, flag name presence, cross-references, exclusive groups, enum values, variadic uniqueness, cycle detection). Uses `CodingAdventures::DirectedGraph::Graph` for flag dependency cycle detection.
- **TokenClassifier**: Classifies a single argv token into a typed event using longest-match-first disambiguation (spec §5.2). Handles `--`, `--name`, `--name=value`, `-classpath` (single_dash_long), `-x` (short), `-xVALUE` (short with inline value), `-lah` (stacked), bare `-` (positional), positional words, and unknown flags.
- **PositionalResolver**: Assigns positional tokens to argument slots using the last-wins algorithm (spec §6.4.1). Handles no-variadic (1-to-1), variadic-only, and variadic+trailing-required layouts. Implements the cp/mv pattern naturally. Type coercion for all supported types (integer, float, path, file, directory, string, enum).
- **FlagValidator**: Validates parsed flag sets against all constraints: duplicate non-repeatable flags, conflicts_with violations, transitive requires (via `CodingAdventures::DirectedGraph::Graph` transitive closure), required flags (with required_unless exemption), and mutually exclusive group violations. Collects all errors rather than stopping at the first.
- **HelpGenerator**: Auto-generates help text from the spec in the format defined by spec §9. Sections: USAGE, DESCRIPTION, COMMANDS, OPTIONS, GLOBAL OPTIONS, ARGUMENTS. Handles all flag and argument formatting variants.
- **Parser**: The main orchestrator. Implements the three-phase parsing algorithm:
  - Phase 1: Directed graph routing to resolve the command path.
  - Phase 2: Modal State Machine driven scanning with `CodingAdventures::StateMachine::ModalStateMachine` (modes: scanning/flag_value/end_of_flags).
  - Phase 3: Validation via FlagValidator and PositionalResolver.
  - Handles all parsing modes: gnu, posix, subcommand_first, traditional.
  - Returns `ParseResult`, `HelpResult`, or `VersionResult`. Raises `ParseErrors` on failure.
- **Error types**: `CliBuilderError` (base), `SpecError` (spec validation), `ParseError` (struct), `ParseErrors` (aggregate).
- **Result types**: `ParseResult`, `HelpResult`, `VersionResult`.
- Comprehensive test suite covering all modules with 100+ assertions.
- Literate programming comments throughout with algorithm explanations, worked examples, and diagrams.

### Notes

- Implements spec `cli_builder_spec_version` "1.0".
- Depends on `coding_adventures_state_machine` (~> 0.1) which depends on `coding_adventures_directed_graph` (~> 0.1).
- All parsing modes specified in the spec are implemented: gnu (default), posix, subcommand_first, traditional.
