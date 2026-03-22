# unix-tools -- A Collection of Unix Utilities (Go)

A growing collection of reimplemented POSIX/Unix utilities in Go, powered by [CLI Builder](../../../packages/go/cli-builder/).

## What This Is

This package contains multiple Unix command-line tools, each implemented as a flat file in the same directory. Every tool uses CLI Builder for argument parsing, help text, and error handling -- the Go code contains only business logic.

## Current Tools

| Tool | Description | Spec File |
|------|-------------|-----------|
| `pwd` | Print the absolute pathname of the current working directory | `pwd.json` |

## Architecture

Each tool follows the same pattern:

```
tool.json (declarative spec)       main.go (business logic)
+-------------------------+        +-----------------------------+
| flags, arguments        |        | Tool-specific logic only    |
| mutual exclusivity      |------> | All CLI concerns handled    |
| help text, version      |        | by cli-builder              |
| error messages          |        |                             |
+-------------------------+        +-----------------------------+
        CLI Builder                        Your code
     handles all of this              handles only this
```

## Usage

```bash
# Print logical working directory (default)
unix-tools  # currently runs pwd

# Print physical working directory (resolve symlinks)
unix-tools -P

# Explicitly request logical path
unix-tools -L

# Show help
unix-tools --help

# Show version
unix-tools --version
```

## Flags (pwd)

| Flag | Long Form | Description |
|------|-----------|-------------|
| `-L` | `--logical` | Display the logical current working directory (default) |
| `-P` | `--physical` | Display the physical current working directory (resolve all symlinks) |

## Where It Fits in the Stack

```
Layer 8: CLI Builder (argument parsing, help, validation)
    +-- This program: unix-tools (business logic only)

Layer 4: State Machine (drives CLI Builder's parsing modes)
Layer 3: Directed Graph (drives CLI Builder's command routing)
```

## Building

```bash
# Via the build system
./build-tool

# Manually
go build -o unix-tools .
```

## Testing

```bash
go test ./... -v
```
