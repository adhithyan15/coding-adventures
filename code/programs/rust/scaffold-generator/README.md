# scaffold-generator (Rust)

Generate CI-ready package scaffolding for the coding-adventures monorepo.

## Overview

This is the Rust implementation of the scaffold-generator program. It generates
correctly-structured, CI-ready package directories for all six languages:
Python, Go, Ruby, TypeScript, Rust, and Elixir.

Unlike the Go implementation which uses the cli-builder package, this Rust
version uses simple manual argument parsing (no external dependencies).

## Usage

```bash
# Generate a library package for all 6 languages
scaffold-generator my-package

# Generate for specific languages
scaffold-generator -l rust,go my-package

# Generate with dependencies
scaffold-generator -d logic-gates,arithmetic --layer 2 my-alu

# Generate a program instead of a library
scaffold-generator -t program my-tool

# Dry run (preview without writing files)
scaffold-generator --dry-run my-package
```

## Flags

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--type` | `-t` | Package type: `library` or `program` | `library` |
| `--language` | `-l` | Comma-separated languages or `all` | `all` |
| `--depends-on` | `-d` | Comma-separated sibling dependencies | (none) |
| `--layer` | | Layer number for README context | 0 |
| `--description` | | One-line package description | (empty) |
| `--dry-run` | | Preview without writing | false |
| `--help` | `-h` | Show help | |
| `--version` | `-V` | Show version | |

## Features

- **Name normalization**: Converts kebab-case to snake_case, CamelCase, and joined-lower as needed
- **Dependency resolution**: Reads sibling packages' metadata to discover transitive dependencies
- **Topological sort**: Orders dependencies leaf-first for correct BUILD file install order
- **All 6 languages**: Python, Go, Ruby, TypeScript, Rust, Elixir templates
- **Rust workspace integration**: Automatically updates code/packages/rust/Cargo.toml members list
- **Zero dependencies**: Pure Rust std library, no external crates

## Development

```bash
# Run tests
bash BUILD
```
