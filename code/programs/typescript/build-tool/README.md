# Build Tool (TypeScript)

An incremental, parallel monorepo build tool implemented in TypeScript. This is a port of the Python build tool, maintaining full feature parity.

## What it does

The build tool discovers packages in the monorepo by walking the directory tree looking for BUILD files, resolves inter-package dependencies by parsing language-specific metadata files, and executes builds in parallel topological order.

## How it fits in the stack

This is one of several build tool implementations in the monorepo (Python, Ruby, Go, Rust, Elixir, TypeScript). All implementations share the same architecture and produce identical results. The Go implementation is the primary one used in CI; the others serve as educational implementations demonstrating the same concepts in different languages.

## Architecture

| Module | Purpose |
|---|---|
| `discovery.ts` | Walks directory tree, finds BUILD files, infers language |
| `resolver.ts` | Parses dependency metadata, builds directed graph (Kahn's algorithm) |
| `gitdiff.ts` | Git-based change detection (`git diff --name-only`) |
| `hasher.ts` | SHA256 hashing of source files for cache-based change detection |
| `cache.ts` | JSON cache file for fallback change detection |
| `executor.ts` | Parallel build execution respecting dependency order |
| `reporter.ts` | Human-readable build report formatting |
| `index.ts` | CLI entry point tying everything together |

## Supported languages

The resolver can parse dependencies for all 6 languages in the monorepo:

- **Python**: `pyproject.toml` (`coding-adventures-*` prefix)
- **Ruby**: `.gemspec` (`coding_adventures_*` prefix)
- **Go**: `go.mod` (full module paths)
- **TypeScript**: `package.json` (`@coding-adventures/*` scoped names)
- **Rust**: `Cargo.toml` (crate names with path dependencies)
- **Elixir**: `mix.exs` (`coding_adventures_*` atom names)

## Platform-specific BUILD files

The discovery system supports platform-specific BUILD files with the following priority:

| Platform | Priority |
|---|---|
| macOS (darwin) | `BUILD_mac` > `BUILD_mac_and_linux` > `BUILD` |
| Linux | `BUILD_linux` > `BUILD_mac_and_linux` > `BUILD` |
| Windows (win32) | `BUILD_windows` > `BUILD` |

## Usage

```bash
# Auto-detect repo root, build changed packages
npx tsx src/index.ts

# Specify root explicitly
npx tsx src/index.ts --root /path/to/repo

# Rebuild everything
npx tsx src/index.ts --force

# Show what would build without building
npx tsx src/index.ts --dry-run

# Limit parallel workers
npx tsx src/index.ts --jobs 4

# Only build Python packages
npx tsx src/index.ts --language python
```

## Development

```bash
# Install dependencies
npm install

# Run tests
npx vitest run

# Run tests with coverage
npx vitest run --coverage
```

## Design decisions

- **Zero runtime dependencies**: Only uses Node.js built-in modules (`node:fs`, `node:path`, `node:crypto`, `node:child_process`, `node:os`, `node:util`).
- **Inline directed graph**: Rather than importing an external graph library, the resolver includes a minimal DirectedGraph implementation.
- **ESM-only**: Uses ES modules throughout (`"type": "module"` in package.json).
- **Literate programming**: All source files include extensive comments explaining concepts, algorithms, and design decisions.
