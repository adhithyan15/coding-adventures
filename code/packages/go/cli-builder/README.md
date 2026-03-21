# cli-builder

A Go package that implements the CLI Builder specification — a declarative,
JSON-driven runtime library for CLI argument parsing built on directed graphs
and modal state machines.

## What it does

CLI Builder separates two concerns that most CLI libraries conflate:

1. **What the tool accepts** — described in a JSON spec file
2. **What the tool does** — the business logic the caller implements

Write a spec once, get parsing, validation, and help generation for free.

## How it fits in the stack

```
cli-builder
  ├── directed-graph   (command routing graph G_cmd; flag dependency graph G_flag)
  └── state-machine    (ModalStateMachine for parse-mode tracking; DFA for token classification)
```

The directed graph drives routing (Phase 1): each command is a node, each
subcommand relationship is an edge. Cycles in the flag dependency graph
are detected at load time and reported as spec errors.

The modal state machine drives scanning (Phase 2): it tracks which parse
mode the parser is in — SCANNING, FLAG_VALUE, or END_OF_FLAGS — and switches
modes as tokens are consumed.

## Usage

```go
import clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"

func main() {
    parser, err := clibuilder.NewParser("my-cli.json", os.Args)
    if err != nil {
        fmt.Fprintln(os.Stderr, err)
        os.Exit(1)
    }

    result, err := parser.Parse()
    if err != nil {
        fmt.Fprintln(os.Stderr, err)
        os.Exit(1)
    }

    switch r := result.(type) {
    case *clibuilder.ParseResult:
        fmt.Printf("Command: %v\n", r.CommandPath)
        fmt.Printf("Flags:   %v\n", r.Flags)
        fmt.Printf("Args:    %v\n", r.Arguments)
    case *clibuilder.HelpResult:
        fmt.Println(r.Text)
        os.Exit(0)
    case *clibuilder.VersionResult:
        fmt.Println(r.Version)
        os.Exit(0)
    }
}
```

## Spec file format

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "mytool",
  "description": "Does something useful",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "global_flags": [ ... ],
  "flags": [ ... ],
  "arguments": [ ... ],
  "commands": [ ... ],
  "mutually_exclusive_groups": [ ... ]
}
```

See `code/specs/cli-builder-spec.md` for the complete specification.

## Parsing modes

| Mode | Behavior |
|---|---|
| `gnu` (default) | Flags may appear anywhere; `--` ends flag scanning |
| `posix` | Flags must appear before positional arguments |
| `subcommand_first` | First non-flag token is always a subcommand |
| `traditional` | `argv[1]` without `-` is treated as stacked short flags (tar-style) |

## Result types

| Type | Trigger |
|---|---|
| `*ParseResult` | Normal successful parse |
| `*HelpResult` | User passed `--help` or `-h` |
| `*VersionResult` | User passed `--version` |

## Error handling

`Parse()` returns `*ParseErrors` on failure, containing a slice of `ParseError`
values. Each error has an `ErrorType` (snake_case, machine-readable), a human-readable
`Message`, an optional fuzzy-match `Suggestion`, and the `Context` (command path).

Errors are collected (not fail-fast) so users see all problems at once.

## Running tests

```bash
go test ./... -v -cover
```
