# pwd — Print Working Directory

A reimplementation of the POSIX `pwd` utility in Elixir, powered by [CLI Builder](../../../packages/elixir/cli_builder/).

## What This Demonstrates

This is the simplest possible Unix tool built on CLI Builder. The entire command-line interface — flags, help text, version output, error messages — is defined in [`pwd.json`](pwd.json). The program itself contains only business logic: reading the current directory and printing it.

## How It Works

```
pwd.json (declarative spec)     lib/pwd/cli.ex (business logic only)
+-------------------------+     +-----------------------------+
| flags: -L, -P           |     | if physical:                |
| mutual exclusivity      |---->|     print(resolve_symlinks) |
| help text, version      |     | else:                       |
| error messages           |     |     print($PWD)             |
+-------------------------+     +-----------------------------+
        CLI Builder                    Your code
     handles all of this           handles only this
```

## Usage

```bash
# Print logical working directory (default)
mix run -e "Pwd.CLI.main([])"

# Print physical working directory (resolve symlinks)
mix run -e "Pwd.CLI.main([\"-P\"])"

# Explicitly request logical path
mix run -e "Pwd.CLI.main([\"-L\"])"

# Show help
mix run -e "Pwd.CLI.main([\"--help\"])"

# Show version
mix run -e "Pwd.CLI.main([\"--version\"])"

# Or build and run as an escript
mix escript.build
./pwd
./pwd -P
./pwd --help
```

## Flags

| Flag | Long Form | Description |
|------|-----------|-------------|
| `-L` | `--logical` | Display the logical current working directory (default) |
| `-P` | `--physical` | Display the physical current working directory (resolve all symlinks) |

## Where It Fits in the Stack

```
Layer 8: CLI Builder (argument parsing, help, validation)
    +-- This program: pwd (business logic only)

Layer 4: State Machine (drives CLI Builder's parsing modes)
Layer 3: Directed Graph (drives CLI Builder's command routing)
```

## Running Tests

```bash
mix deps.get
mix test
```
