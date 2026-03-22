# Unix Tools

A collection of classic Unix command-line utilities reimplemented in Elixir, powered by [CLI Builder](../../../packages/elixir/cli_builder/).

## Purpose

This package serves two goals:

1. **Learn Elixir** by building real, useful programs.
2. **Demonstrate CLI Builder** — every tool's interface is defined declaratively in a JSON spec file, with zero hand-written argument parsing.

## Tools Included

### pwd

Print the absolute pathname of the current working directory.

```bash
# Logical path (default) — uses $PWD, which preserves symlink names
mix run -e "UnixTools.Pwd.main([])"

# Physical path — resolves all symlinks
mix run -e "UnixTools.Pwd.main([\"-P\"])"
```

Flags:
- `-L` / `--logical` — display the logical working directory (default)
- `-P` / `--physical` — resolve symlinks and display the physical path
- `--help` — show help text
- `--version` — show version

## How It Works

Each tool has:
- A **JSON spec file** (e.g., `pwd.json`) that defines the CLI interface
- An **Elixir module** (e.g., `UnixTools.Pwd`) that contains only business logic

CLI Builder reads the spec and handles all parsing, validation, help generation, and error reporting. The Elixir module pattern-matches on the parse result and runs the appropriate logic.

## Building and Testing

```bash
mix deps.get
mix test
```

## Dependencies

- [CLI Builder](../../../packages/elixir/cli_builder/) — declarative CLI parsing from JSON specs
