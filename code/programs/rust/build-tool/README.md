# Build Tool (Rust)

A **Rust port** of the Go build tool for the coding-adventures monorepo. It discovers packages, resolves dependencies, hashes source files for change detection, and rebuilds only what changed. Independent packages are built in parallel using Rayon's work-stealing thread pool.

## What it does

This tool discovers packages in the monorepo via recursive `BUILD` file walking, resolves inter-package dependencies, hashes source files for change detection, and only rebuilds packages whose source or dependency inputs changed. Independent packages are built in parallel.

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
| `--language` | all | Filter to: python, ruby, go, rust, or all |
| `--cache-file` | .build-cache.json | Path to the build cache file |

## Architecture

The tool is organized into eight modules, each responsible for one aspect of the build pipeline:

1. **graph** — Directed graph data structure with topological sort and affected-node queries
2. **discovery** — Recursively walks for `BUILD` files to find packages
3. **resolver** — Parses pyproject.toml, .gemspec, go.mod, Cargo.toml for dependencies
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

## How it fits in the stack

This is a **program** (not a library). It embeds a directed graph implementation directly rather than importing one as a separate crate, keeping the tool self-contained. The graph module implements the same algorithms as the Go `directed-graph` package.
