# Build Tool (Elixir)

An incremental, parallel monorepo build system written in Elixir. This is a full port of
the Go build tool, preserving the same architecture, flags, and behavior while leveraging
Elixir's OTP patterns (Agent, Task) and the BEAM's lightweight concurrency model.

## How it fits in the stack

The `coding-adventures` monorepo contains build tool implementations in multiple languages:

| Language | Purpose |
|----------|---------|
| **Go** | Primary build tool — compiles to a single static binary |
| **Python** | Educational implementation — demonstrates the algorithm in a scripting language |
| **Ruby** | Educational implementation — idiomatic Ruby with gems |
| **Elixir** | This package — OTP-native implementation using Agent and Task |

All implementations share the same algorithm (discover packages, resolve dependencies,
hash source files, execute builds in parallel by dependency level) and produce identical
results given the same inputs.

## Architecture

The build tool follows an 11-step pipeline:

1. **Find repo root** — walk up looking for `.git`
2. **Discover packages** — recursive walk looking for `BUILD` files
3. **Filter by language** — optional `--language` flag
4. **Resolve dependencies** — parse `pyproject.toml`, `.gemspec`, `go.mod`, `package.json`, `Cargo.toml`, `mix.exs`
5. **Git-diff change detection** — `git diff --name-only` against a base ref
6. **Hash packages** — SHA256 of source files and transitive dependency hashes
7. **Load cache** — JSON-based build cache (fallback when git diff unavailable)
8. **Dry-run check** — report what would build and exit
9. **Execute builds** — parallel execution by dependency level using `Task.async_stream`
10. **Save cache** — atomic write to `.build-cache.json`
11. **Print report** — fixed-width table of results

## Modules

| Module | Responsibility |
|--------|---------------|
| `BuildTool.CLI` | Escript entry point, argument parsing, orchestration |
| `BuildTool.Discovery` | Recursive directory walk to find `BUILD` files |
| `BuildTool.DirectedGraph` | Inline DAG implementation (Kahn's algorithm, affected nodes) |
| `BuildTool.Resolver` | Parse dependency metadata, build the dependency graph |
| `BuildTool.GitDiff` | Git-based change detection |
| `BuildTool.Hasher` | SHA256 hashing of source files and dependencies |
| `BuildTool.Cache` | Agent-based JSON build cache with atomic writes |
| `BuildTool.Executor` | Parallel build execution with progress tracking |
| `BuildTool.Reporter` | Fixed-width report table formatting |

## Usage

### Build and run as escript

```bash
mix deps.get
mix escript.build
./build_tool --root /path/to/repo
```

### Common flags

```bash
# Rebuild everything
./build_tool --force

# Show what would build without executing
./build_tool --dry-run

# Build only Python packages
./build_tool --language python

# Limit parallel jobs
./build_tool --jobs 4

# Custom diff base
./build_tool --diff-base origin/develop
```

### All flags

| Flag | Default | Description |
|------|---------|-------------|
| `--root` | auto-detect | Repo root directory |
| `--force` | `false` | Rebuild everything regardless of cache |
| `--dry-run` | `false` | Show what would build without executing |
| `--jobs` | CPU count | Max parallel jobs |
| `--language` | `all` | Filter to a specific language |
| `--diff-base` | `origin/main` | Git ref to diff against |
| `--cache-file` | `.build-cache.json` | Path to cache file |

## Dependencies

- **Jason** — JSON encoding/decoding for the build cache
- **CodingAdventures.ProgressBar** — Live terminal progress bar (from this monorepo)
