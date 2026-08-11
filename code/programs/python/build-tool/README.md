# coding-adventures-build-tool

An incremental, parallel monorepo build tool with hash-based caching.

## What it does

This CLI program:
1. Discovers all packages via recursive `BUILD` file walking
2. Resolves dependencies from each supported ecosystem's package metadata
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
the entire coding-adventures monorepo. It understands the recursive `BUILD`
discovery convention used throughout the repository and orchestrates every
language listed by `build-tool --help`.

Lua rockspec metadata is decoded as strict UTF-8. Invalid text fails closed with
the stable `METADATA_INVALID_UTF8` diagnostic rather than using a host locale or
silently replacing bytes.

Ordinary dependency aliases are resolved only inside the package's ecosystem.
For example, the same `coding-adventures-shared` name in Lua, Perl, Python, and
Haskell maps to four distinct package identities. A legacy BUILD file may name
an intentional cross-language dependency only with its exact qualified identity:

```text
# build-tool: deps=lua/shared
```

Unknown, unqualified, and self-referential comment entries are ignored. Library
packages retain priority over same-named programs within one ecosystem.

## Installation

```bash
cd code/programs/python/build-tool
uv pip install -e ".[dev]"
```

## Development

```bash
uv run pytest tests/ -v
```
