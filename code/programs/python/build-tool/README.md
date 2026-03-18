# coding-adventures-build-tool

An incremental, parallel monorepo build tool with hash-based caching.

## What it does

This CLI program:
1. Discovers all packages via DIRS/BUILD files
2. Resolves dependencies by parsing pyproject.toml and .gemspec files
3. Builds a directed graph of dependencies
4. Hashes all source files in each package
5. Compares hashes against a committed cache file (.build-cache.json)
6. Only runs BUILD commands for packages whose hash (or dependency hash) changed
7. Runs independent packages in parallel using concurrent.futures

## Usage

```bash
# Auto-detect root, build changed packages
build-tool

# Specify root explicitly
build-tool --root /path/to/repo

# Rebuild everything
build-tool --force

# Show what would build without building
build-tool --dry-run

# Limit parallel workers
build-tool --jobs 4

# Only build Python packages
build-tool --language python
```

## How it fits in the stack

This is a standalone program (not a library) that orchestrates builds across
the entire coding-adventures monorepo. It understands the DIRS/BUILD file
conventions used throughout the repository and can build Python, Ruby, and Go
packages.

## Installation

```bash
cd code/programs/python/build-tool
uv pip install -e ".[dev]"
```

## Development

```bash
uv run pytest tests/ -v
```
