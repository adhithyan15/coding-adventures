# coding_adventures_cli_builder

Declarative CLI argument parsing for Elixir, driven by directed graphs and state machines.

## What it does

CLI Builder separates *what a CLI tool accepts* (a JSON specification file) from
*what it does* (your application logic). You write a JSON spec describing your
program's subcommands, flags, and positional arguments; this library handles all
parsing, validation, help generation, and error reporting.

## Where it fits in the stack

```
JSON Spec File
     │
     ▼
SpecLoader ─── validates, builds G_flag ──► SpecError on violation
     │
     ▼
Parser.parse/2
     ├── Phase 1: Routing (DirectedGraph — command routing graph G_cmd)
     ├── Phase 2: Scanning (TokenClassifier + ModalStateMachine)
     └── Phase 3: Validation (PositionalResolver + FlagValidator)
          │
          ▼
     ParseResult | HelpResult | VersionResult | ParseErrors
```

This package depends on:
- `coding_adventures_directed_graph` — Graph for cycle detection and transitive closure
- `coding_adventures_state_machine` — DFA and Modal state machine for parse mode tracking
- `jason` — JSON parsing of spec files

## Quick start

```elixir
# 1. Write your spec (my_tool.json):
# {
#   "cli_builder_spec_version": "1.0",
#   "name": "my-tool",
#   "description": "A tool that does things",
#   "version": "1.0.0",
#   "flags": [
#     {"id": "verbose", "short": "v", "long": "verbose",
#      "description": "Enable verbose output", "type": "boolean"}
#   ],
#   "arguments": [
#     {"id": "file", "name": "FILE", "description": "Input file",
#      "type": "path", "required": true}
#   ]
# }

# 2. Parse at runtime:
alias CodingAdventures.CliBuilder
alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

case CliBuilder.parse("my_tool.json", System.argv()) do
  {:ok, %ParseResult{flags: flags, arguments: args}} ->
    run_my_tool(flags["verbose"], args["file"])

  {:ok, %HelpResult{text: text}} ->
    IO.puts(text)

  {:ok, %VersionResult{version: v}} ->
    IO.puts(v)

  {:error, %ParseErrors{message: msg}} ->
    IO.puts(:stderr, msg)
    System.halt(1)
end
```

## Spec format

See `code/specs/cli-builder-spec.md` for the complete specification. A minimal
spec looks like:

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "my-tool",
  "description": "One-line description",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "flags": [ ... ],
  "arguments": [ ... ],
  "commands": [ ... ]
}
```

### Parsing modes

| Mode | Behaviour |
|---|---|
| `gnu` (default) | Flags may appear anywhere; `--` ends flag scanning |
| `posix` | First non-flag token ends flag scanning |
| `subcommand_first` | First token is always a subcommand |
| `traditional` | `argv[1]` without leading `-` is stacked short flags (tar-style) |

## Module overview

| Module | Role |
|---|---|
| `CodingAdventures.CliBuilder` | Thin facade; `parse/2` and `parse_string/2` |
| `CliBuilder.SpecLoader` | Read + validate JSON spec; raises `SpecError` |
| `CliBuilder.TokenClassifier` | Character-level token classification DFA |
| `CliBuilder.PositionalResolver` | Assign positionals to argument slots |
| `CliBuilder.FlagValidator` | Conflict, requires, required, group checks |
| `CliBuilder.HelpGenerator` | Generate formatted help text |
| `CliBuilder.Parser` | Three-phase parse orchestration |
| `CliBuilder.ParseError` | Single parse error struct |
| `CliBuilder.ParseErrors` | Exception wrapping a list of `ParseError` |
| `CliBuilder.SpecError` | Exception for spec load-time failures |
| `CliBuilder.ParseResult` | Successful parse result |
| `CliBuilder.HelpResult` | Help-requested result |
| `CliBuilder.VersionResult` | Version-requested result |

## Type system

| Type | Description |
|---|---|
| `boolean` | Flag presence = `true`, absence = `false`. No value token consumed. |
| `string` | Any non-empty string. |
| `integer` | Whole number; coerced to Elixir `integer`. |
| `float` | Floating-point number; coerced to Elixir `float`. |
| `path` | Filesystem path; syntactic check only (existence not required). |
| `file` | Existing readable file; checked at parse time. |
| `directory` | Existing directory; checked at parse time. |
| `enum` | One of a fixed set of strings declared in `enum_values`. |

## Error handling

Errors are collected (not fail-fast) so users get a complete picture:

```elixir
{:error, %ParseErrors{errors: errors}} = CliBuilder.parse_string(spec, argv)
Enum.each(errors, fn e ->
  IO.puts("#{e.error_type}: #{e.message}")
  if e.suggestion, do: IO.puts("  Did you mean: #{e.suggestion}?")
end)
```

Error types: `unknown_flag`, `unknown_command`, `missing_required_flag`,
`missing_required_argument`, `conflicting_flags`, `missing_dependency_flag`,
`too_few_arguments`, `too_many_arguments`, `invalid_value`, `invalid_enum_value`,
`exclusive_group_violation`, `missing_exclusive_group`, `duplicate_flag`,
`invalid_stack`.

## Running tests

```bash
cd code/packages/elixir/cli_builder
mix deps.get
mix test --cover
```

## Building via the build tool

```bash
cd code/programs/go/build-tool
./build-tool
```

The BUILD file at `code/packages/elixir/cli_builder/BUILD` drives the build:
it compiles all dependencies and runs `mix test --cover`.
