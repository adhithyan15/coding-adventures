# unix-tools — Unix Utilities in Rust

A collection of reimplemented POSIX utilities in Rust, powered by [CLI Builder](../../../packages/rust/cli-builder/).

## Included Tools

### pwd — Print Working Directory

The simplest possible Unix tool built on CLI Builder. The entire command-line interface — flags, help text, version output, error messages — is defined in [`pwd.json`](pwd.json). The program itself contains only business logic: reading the current directory and printing it.

#### How It Works

```
pwd.json (declarative spec)     src/main.rs (business logic only)
+-------------------------+     +-----------------------------+
| flags: -L, -P           |     | if physical:                |
| mutual exclusivity      |---->|     print(resolve_symlinks) |
| help text, version      |     | else:                       |
| error messages           |     |     print($PWD)             |
+-------------------------+     +-----------------------------+
        CLI Builder                    Your code
     handles all of this           handles only this
```

#### Usage

```bash
# Print logical working directory (default)
cargo run

# Print physical working directory (resolve symlinks)
cargo run -- -P

# Explicitly request logical path
cargo run -- -L

# Show help
cargo run -- --help

# Show version
cargo run -- --version
```

#### Flags

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

## Building and Testing

```bash
# Build
mise exec -- cargo build

# Run tests
mise exec -- cargo test --all-targets -- --nocapture

# Run the program
mise exec -- cargo run
mise exec -- cargo run -- -P
mise exec -- cargo run -- --help
```
