# coding_adventures_cli_builder

Declarative CLI argument parsing driven by directed graphs and state machines.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) computing stack.

## What It Does

CLI Builder separates *what a tool accepts* from *what it does*.

A developer writes one JSON specification file describing their CLI's structure — flags, subcommands, positional arguments, constraints. CLI Builder reads this file at startup and handles everything else: routing, parsing, validation, help generation, error messages. The developer focuses entirely on business logic.

```
JSON spec file  →  SpecLoader  →  Parser  →  ParseResult
                                          →  HelpResult
                                          →  VersionResult
                               ↑ raises ParseErrors on bad input
```

## How It Fits in the Stack

| Layer | Package | Role |
|-------|---------|------|
| L1 | `coding_adventures_directed_graph` | Graph data structure and algorithms |
| L2 | `coding_adventures_state_machine` | DFA, Modal State Machine |
| L3 | `coding_adventures_cli_builder` | This package — uses L1 and L2 |

CLI Builder uses directed graphs in two ways:
- **G_cmd**: the command routing graph (nodes = commands, edges = subcommand relationships)
- **G_flag**: the flag dependency graph (edges = "A requires B" constraints)

It uses the Modal State Machine to drive token-by-token scanning with three modes:
- `scanning` — normal parsing
- `flag_value` — waiting for a non-boolean flag's value
- `end_of_flags` — after `--`, everything is positional

## Installation

```ruby
gem "coding_adventures_cli_builder"
```

## Quick Start

1. Write a JSON spec file for your tool (see `code/specs/cli-builder-spec.md` for the full schema):

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "greet",
  "description": "Say hello",
  "version": "1.0.0",
  "flags": [
    {
      "id": "shout",
      "short": "s",
      "long": "shout",
      "description": "Uppercase the output",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "name",
      "name": "NAME",
      "description": "Who to greet",
      "type": "string",
      "required": true
    }
  ]
}
```

2. Parse `ARGV` at program startup:

```ruby
require "coding_adventures_cli_builder"

begin
  result = CodingAdventures::CliBuilder::Parser.new("greet.json", ARGV).parse

  case result
  when CodingAdventures::CliBuilder::ParseResult
    greeting = "Hello, #{result.arguments["name"]}!"
    greeting = greeting.upcase if result.flags["shout"]
    puts greeting

  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    exit 0

  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    exit 0
  end

rescue CodingAdventures::CliBuilder::ParseErrors => e
  e.errors.each { |err| warn err.message }
  exit 1

rescue CodingAdventures::CliBuilder::SpecError => e
  warn "CLI spec error (this is a bug): #{e.message}"
  exit 2
end
```

3. Run:

```
$ greet Alice
Hello, Alice!

$ greet --shout Alice
HELLO, ALICE!

$ greet --help
USAGE
  greet [OPTIONS] <NAME>

DESCRIPTION
  Say hello

OPTIONS
  -s, --shout          Uppercase the output.

GLOBAL OPTIONS
  -h, --help     Show this help message and exit.
  --version      Show version and exit.

ARGUMENTS
  <NAME>          Who to greet. Required.
```

## Supported Features

### Parsing Modes

| Mode | Behavior |
|------|----------|
| `gnu` (default) | Flags anywhere in argv. `--` ends flag scanning. |
| `posix` | First positional token ends flag scanning (like POSIX `getopt`). |
| `subcommand_first` | First non-flag token is always a subcommand. |
| `traditional` | First token may be stacked short flags without `-` (tar-style). |

### Flag Types

`boolean`, `string`, `integer`, `float`, `path`, `file`, `directory`, `enum`

### Flag Variants

```
--long-name            long form
-x                     short form
-classpath             single-dash-long (Java/X11 style)
-xyz                   stacking (boolean flags only)
-fvalue                short with inline value
--output=file.txt      long with inline value
```

### Constraints

- `required: true` — flag must be present
- `required_unless: [ids]` — required unless one of the listed flags is present
- `conflicts_with: [ids]` — cannot be used with these flags
- `requires: [ids]` — these flags must also be present (transitively enforced)
- `repeatable: true` — may appear multiple times; result is an array
- `mutually_exclusive_groups` — at most one (or exactly one if required) of a set of flags

### Subcommands

Unlimited nesting depth. Each command has its own flags and arguments. Global flags apply everywhere.

```
git remote add <name> <url>
git remote remove <name>
git commit -m <message>
```

### Variadic Arguments

The last-wins algorithm handles patterns like `cp <source>... <dest>`:

```
cp a.txt b.txt c.txt /dest/
  source = ["a.txt", "b.txt", "c.txt"]
  dest   = "/dest/"
```

### Auto-Generated Help

`--help` / `-h` always returns a `HelpResult` with formatted help text. The help is derived entirely from the spec — no manual text needed.

## Modules

| Module | Responsibility |
|--------|---------------|
| `SpecLoader` | Read and validate the JSON spec file |
| `TokenClassifier` | Classify one argv token into a typed event |
| `PositionalResolver` | Assign positional tokens to argument slots |
| `FlagValidator` | Validate parsed flags against spec constraints |
| `HelpGenerator` | Generate formatted help text from the spec |
| `Parser` | Orchestrate all phases: routing, scanning, validation |

## Error Handling

Parse errors are collected (not fail-fast) and raised as a single `ParseErrors` exception containing an array of `ParseError` structs:

```ruby
rescue CodingAdventures::CliBuilder::ParseErrors => e
  e.errors.each do |err|
    warn "#{err.message}"
    warn "  Hint: #{err.suggestion}" if err.suggestion
  end
end
```

Each `ParseError` has:
- `error_type` — machine-readable string (e.g. `"missing_required_flag"`)
- `message` — human-readable description
- `suggestion` — optional corrective hint (fuzzy match for unknown flags/commands)
- `context` — the command path at the point of error
