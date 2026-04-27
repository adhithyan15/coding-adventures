# Build Tool (Go)

The **primary build tool** for the coding-adventures monorepo. Compiled to a native binary for fast, dependency-free CI execution.

## What it does

This tool discovers packages in the monorepo by recursively walking for `BUILD` files, resolves inter-package dependencies, hashes source files for change detection, and only rebuilds packages whose source or dependency inputs changed. Independent packages are built in parallel using Go goroutines.

## Building

```bash
cd code/programs/go/build-tool
go build -o build-tool .
```

On Windows, build the executable with the `.exe` suffix so PowerShell runs it
directly instead of asking which application should open an extensionless file:

```powershell
cd code\programs\go\build-tool
go build -o ..\..\..\..\build-tool.exe .
```

This produces a single static binary with no runtime dependencies.

## Usage

```bash
# Auto-detect repo root, build all changed packages
./build-tool

# Specify root explicitly
./build-tool -root /path/to/repo

# Rebuild everything regardless of cache
./build-tool -force

# Show what would build without actually building
./build-tool -dry-run

# Limit parallel workers
./build-tool -jobs 4

# Only build Python packages
./build-tool -language python

# Custom git diff base
./build-tool -diff-base origin/develop

# Custom cache file location
./build-tool -cache-file /tmp/my-cache.json
```

On Windows, use the compiled `.exe`:

```powershell
.\build-tool.exe -root . -diff-base origin/main
.\build-tool.exe -root . -validate-build-files -detect-languages -emit-plan build-plan.json
```

## CLI flags

| Flag | Default | Description |
|------|---------|-------------|
| `-root` | auto-detect | Repo root directory (walks up looking for .git) |
| `-force` | false | Rebuild everything regardless of cache |
| `-dry-run` | false | Show what would build without executing |
| `-jobs` | NumCPU | Maximum parallel build jobs |
| `-language` | all | Filter to: python, ruby, go, rust, typescript, elixir, lua, perl, swift, wasm, csharp, fsharp, dotnet, or all |
| `-diff-base` | origin/main | Git ref to diff against for change detection |
| `-cache-file` | .build-cache.json | Path to the build cache file |

## Architecture

The tool is organized into seven internal packages, each responsible for one phase of the build pipeline:

1. **discovery** -- Recursively walks for `BUILD` files to find packages
2. **resolver** -- Parses `pyproject.toml`, `.gemspec`, `go.mod`, `Cargo.toml`, `package.json`, `mix.exs`, and `pubspec.yaml`
3. **hasher** -- SHA256 hashing for change detection
4. **cache** -- JSON-based build cache (read/write with atomic saves)
5. **executor** -- Parallel execution with goroutines + semaphore
6. **gitdiff** -- Git-based change detection for incremental builds
7. **reporter** -- Terminal-friendly build report formatting

## Shared toolchains

Some package languages share an underlying toolchain:

- `wasm` packages currently resolve dependencies like Rust crates and use the Rust toolchain in CI planning.
- `csharp` and `fsharp` packages are grouped under the `.NET` toolchain for CI planning and dependency resolution.

## Go concurrency advantage

The key advantage of the Go implementation over Python/Ruby is concurrency. Go uses goroutines -- lightweight user-space threads (~2KB each vs ~8MB for OS threads). The executor spawns one goroutine per package at each dependency level, with a semaphore (buffered channel) limiting actual concurrency to `-jobs`.

```go
semaphore := make(chan struct{}, maxJobs)
var wg sync.WaitGroup

for _, pkg := range level {
    wg.Add(1)
    go func(p Package) {
        defer wg.Done()
        semaphore <- struct{}{}        // acquire
        defer func() { <-semaphore }() // release
        result := runPackageBuild(p)
        results <- result
    }(pkg)
}
wg.Wait()
```

## Comparison with Python/Ruby implementations

| Feature | Go (this) | Python | Ruby |
|---------|-----------|--------|------|
| Startup time | ~5ms | ~200ms | ~300ms |
| Concurrency | goroutines | ThreadPoolExecutor | (planned) |
| Dependencies | none (static binary) | Python 3.12+ | Ruby 3.4+ |
| CI-ready | yes (commit binary) | requires interpreter | requires interpreter |

## Running tests

```bash
go test ./... -v
```

## How it fits in the stack

This is a **program** (not a library). It uses the `directed-graph` package from `code/packages/go/directed-graph` for dependency graph operations (topological sort, independent groups, transitive dependents).
