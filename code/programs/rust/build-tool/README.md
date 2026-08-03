# Build Tool (Rust)

A **Rust port** of the Go build tool for the coding-adventures monorepo. It discovers packages, resolves dependencies, hashes source files for change detection, and rebuilds only what changed. Independent packages are built in parallel using Rayon's work-stealing thread pool.

## What it does

This tool discovers packages in the monorepo via recursive `BUILD` file walking, resolves inter-package dependencies, hashes source files for change detection, and only rebuilds packages whose source or dependency inputs changed. Independent packages are built in parallel. Discovery uses the repository's canonical language registry, and programs retain a `programs` identity segment so a library and program with the same basename stay distinct. Text metadata is decoded deterministically: Lua `.rockspec` files must be strict UTF-8, and invalid bytes fail closed instead of silently deleting dependency edges.

## Building

```bash
cd code/programs/rust/build-tool
cargo build --release
```

The release binary is at `target/release/build-tool` (or `build-tool.exe` on Windows).

## Usage

```bash
# Auto-detect repo root, build all changed packages
./build-tool

# Specify root explicitly
./build-tool --root /path/to/repo

# Rebuild everything regardless of cache
./build-tool --force

# Show what would build without actually building
./build-tool --dry-run

# Limit parallel workers
./build-tool --jobs 4

# Only build Python packages
./build-tool --language python

# Custom cache file location
./build-tool --cache-file /tmp/my-cache.json

# Custom git diff base
./build-tool --diff-base origin/develop
```

## CLI flags

| Flag | Default | Description |
|------|---------|-------------|
| `--root` | auto-detect | Repo root directory (walks up looking for .git) |
| `--diff-base` | origin/main | Git ref to diff against for change detection |
| `--force` | false | Rebuild everything regardless of cache |
| `--dry-run` | false | Show what would build without executing |
| `--jobs` | CPU count | Maximum parallel build jobs |
| `--language` | all | Filter to one canonical discovery language or use `all` |
| `--cache-file` | .build-cache.json | Path to the build cache file |

## Architecture

The tool is organized into eight modules, each responsible for one aspect of the build pipeline:

1. **graph** — Directed graph data structure with topological sort and affected-node queries
2. **discovery** — Recursively walks for `BUILD` files to find packages
3. **resolver** — Parses supported package metadata, including strict UTF-8 Lua rockspecs, into dependency edges
4. **hasher** — SHA256 hashing for change detection
5. **cache** — JSON-based build cache (read/write with atomic saves)
6. **executor** — Parallel execution with Rayon thread pool
7. **gitdiff** — Git-based change detection (default mode)
8. **reporter** — Terminal-friendly build report formatting

## Rayon parallelism

The Rust implementation uses Rayon's work-stealing thread pool instead of Go's goroutine model. Rayon automatically distributes work across OS threads. For each dependency level, we use `par_iter()` to execute builds in parallel:

```rust
pool.install(|| {
    to_build.par_iter().for_each(|pkg| {
        let result = run_package_build(pkg);
        // Update cache and results...
    });
});
```

This is equivalent to the Go implementation's goroutine + semaphore pattern, but more idiomatic for Rust.

## Comparison with Go implementation

| Feature | Go (primary) | Rust (this) |
|---------|-------------|-------------|
| Startup time | ~5ms | ~3ms |
| Concurrency | goroutines | Rayon thread pool |
| Dependencies | none (static binary) | none (static binary) |
| Safety | runtime checks | compile-time guarantees |
| Memory | GC-managed | ownership-based, no GC |

## Running tests

```bash
cargo test -- --nocapture
```

## Discovery and metadata diagnostics

Discovery rejects any residual qualified-name collision before dependency
resolution. The diagnostic contains the shared identity and sorted
repository-relative paths without exposing the checkout root:

```text
DUPLICATE_PACKAGE_IDENTITY: package=unknown/demo paths=code/packages/alpha/demo,code/packages/beta/demo
```

The shared `discovery/language-registry` fixture covers the canonical buckets
that were previously omitted, while `discovery/duplicate-identity` verifies
the stable CLI exit-code-2 failure path.

Lua `.rockspec` metadata is decoded as strict UTF-8 before dependency parsing.
Invalid bytes stop resolution, return exit code `2`, and emit a stable diagnostic
without the checkout root:

```text
METADATA_INVALID_UTF8: package=lua/pkg manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec encoding=UTF-8
```

The resolver and real CLI tests materialize the language-neutral
`resolution/lua-utf8` and `resolution/lua-invalid-utf8` fixtures.

A resolved dependency self-edge also fails closed with exit code `2` instead
of reaching the embedded graph assertion:

```text
DEPENDENCY_SELF_EDGE: package=elixir/pkg manifest=code/packages/elixir/pkg/mix.exs dependency=elixir/pkg
```

The shared `resolution/elixir-program-package` fixture verifies that the
`grammar_tools` package and program remain distinct, while
`resolution/elixir-self-edge` verifies the stable error path.

## How it fits in the stack

This is a **program** (not a library). It embeds a directed graph implementation directly rather than importing one as a separate crate, keeping the tool self-contained. The graph module implements the same algorithms as the Go `directed-graph` package.
