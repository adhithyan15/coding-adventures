# cli-builder

A declarative runtime library for CLI argument parsing, driven by directed graphs
and state machines. Write a JSON spec file describing your CLI's structure; CLI
Builder handles all parsing, validation, help generation, and error reporting.

## Where it fits in the stack

```
code/specs/cli-builder-spec.md     <- the formal specification
    │
    └── code/packages/rust/cli-builder/   <- this crate
            │
            ├── depends on: directed-graph  (command routing graph + flag dependency graph)
            └── depends on: state-machine   (modal state machine for parse mode tracking)
```

## The Core Insight

A CLI tool's valid syntax forms a directed graph. Consider `git remote add origin <url>`:
the user navigates from `git` → `remote` → `add`, then provides positional arguments.
Invalid input = a token that has no matching edge from the current node.

Flag constraints (one flag requires another; two flags conflict) form a second directed
graph layered on top. Cycle detection on that graph catches spec errors like
"A requires B requires A" before any parsing happens.

Parsing is executed by a modal state machine with four modes:

```
SCANNING ──────────────────────────► SCANNING
    │  (boolean flag seen)               ▲
    │  (flag with inline value seen)     │
    ▼                                    │
FLAG_VALUE ─── (value token consumed) ──┘
    │
    │  "--" token seen
    ▼
END_OF_FLAGS  (all remaining tokens are positional)
```

## Quick Start

```rust
use cli_builder::{load_spec_from_str, Parser};
use cli_builder::types::ParserOutput;

let spec = load_spec_from_str(r#"{
    "cli_builder_spec_version": "1.0",
    "name": "greet",
    "description": "Print a greeting",
    "flags": [
        {"id": "shout", "short": "s", "long": "shout",
         "description": "Print in uppercase", "type": "boolean"}
    ],
    "arguments": [
        {"id": "name", "name": "NAME",
         "description": "Who to greet", "type": "string",
         "required": true}
    ]
}"#).expect("invalid spec");

let parser = Parser::new(spec);
let args: Vec<String> = std::env::args().collect();

match parser.parse(&args).unwrap_or_else(|e| { eprintln!("{}", e); std::process::exit(1); }) {
    ParserOutput::Parse(result) => {
        let name = result.arguments["name"].as_str().unwrap();
        let shout = result.flags["shout"].as_bool().unwrap();
        let greeting = if shout { name.to_uppercase() } else { name.to_string() };
        println!("Hello, {}!", greeting);
    }
    ParserOutput::Help(h)    => { print!("{}", h.text); }
    ParserOutput::Version(v) => { println!("{}", v.version); }
}
```

## Spec Format

The spec is a JSON file. Full reference in `code/specs/cli-builder-spec.md`.

### Minimal spec

```json
{
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "What my app does"
}
```

### Echo (variadic args, flag conflicts)

```json
{
    "cli_builder_spec_version": "1.0",
    "name": "echo",
    "description": "Display a line of text",
    "version": "1.0.0",
    "flags": [
        {"id": "no-newline", "short": "n", "description": "No trailing newline", "type": "boolean"},
        {"id": "enable-escapes", "short": "e", "description": "Enable backslash escapes",
         "type": "boolean", "conflicts_with": ["disable-escapes"]},
        {"id": "disable-escapes", "short": "E", "description": "Disable backslash escapes",
         "type": "boolean", "conflicts_with": ["enable-escapes"]}
    ],
    "arguments": [
        {"id": "string", "name": "STRING", "description": "Text to print",
         "type": "string", "required": false, "variadic": true, "variadic_min": 0}
    ]
}
```

### Git (subcommands, global flags, aliases)

```json
{
    "cli_builder_spec_version": "1.0",
    "name": "git",
    "description": "The stupid content tracker",
    "parsing_mode": "subcommand_first",
    "global_flags": [
        {"id": "no-pager", "long": "no-pager", "description": "Do not pipe output", "type": "boolean"}
    ],
    "commands": [
        {
            "id": "cmd-commit",
            "name": "commit",
            "aliases": ["ci"],
            "description": "Record changes to the repository",
            "flags": [
                {"id": "message", "short": "m", "long": "message",
                 "description": "Commit message", "type": "string", "required": true}
            ]
        }
    ]
}
```

## Parsing Modes

| Mode | Behavior |
|---|---|
| `gnu` (default) | Flags may appear anywhere in argv. `--` ends flag scanning. |
| `posix` | First non-flag token ends flag scanning. |
| `subcommand_first` | First non-flag token is always a subcommand name. |
| `traditional` | First token may be dash-less stacked flags (`tar xvf`). |

## Type System

| Type | Validation |
|---|---|
| `boolean` | Flag presence = true, absence = false. No value token consumed. |
| `string` | Must be non-empty. |
| `integer` | Must parse as signed integer. |
| `float` | Must parse as IEEE 754 double. |
| `path` | Syntactically valid, non-empty. Existence not checked. |
| `file` | Must be an existing, readable file at parse time. |
| `directory` | Must be an existing directory at parse time. |
| `enum` | Must exactly match one of `enum_values`. Case-sensitive. |

## Error Handling

All errors are collected in a single pass:

```rust
match parser.parse(&args) {
    Ok(output) => { /* handle success */ }
    Err(cli_builder::CliBuilderError::ParseErrors(errs)) => {
        for e in &errs.errors {
            eprintln!("error: {} ({})", e.message, e.error_type);
            if let Some(ref s) = e.suggestion {
                eprintln!("  hint: {}", s);
            }
        }
        std::process::exit(1);
    }
    Err(e) => { eprintln!("{}", e); std::process::exit(1); }
}
```

Error types: `unknown_flag`, `unknown_command`, `missing_required_flag`,
`missing_required_argument`, `conflicting_flags`, `missing_dependency_flag`,
`too_few_arguments`, `too_many_arguments`, `invalid_value`, `invalid_enum_value`,
`exclusive_group_violation`, `missing_exclusive_group`, `duplicate_flag`,
`invalid_stack`, `spec_error`.

Unknown flags include fuzzy suggestions (Levenshtein distance ≤ 2):
`Unknown flag '--mesage'. Did you mean '--message'?`

## Design Principles

1. **Separation of concerns** — the spec is the interface; the implementation is the behavior.
2. **Language agnostic** — one JSON spec works with any CLI Builder implementation.
3. **Composable** — built directly on `directed-graph` and `state-machine` packages.
4. **Fail loudly at load time** — spec errors (cycles, duplicate IDs) are caught before any argv is parsed.
5. **Collect all errors** — users see everything wrong in a single invocation.
