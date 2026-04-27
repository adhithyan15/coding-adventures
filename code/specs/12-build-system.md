# 12 — Build System

## Overview

The build system discovers, resolves, and builds packages across a multi-language monorepo. It is implemented in four languages (Go, Python, Ruby, Rust) with a shared architecture, but the Go implementation is the most up to date and is the primary build tool used in CI.

The build system is not part of the computing stack — it is the infrastructure that builds and tests the stack.

## Design Goals

1. **Incremental**: Only rebuild packages that changed (via git diff or hash comparison).
2. **Parallel**: Independent packages build concurrently.
3. **Multi-language**: The primary tool builds Python, Ruby, Go, Rust, and TypeScript packages.
4. **Zero configuration**: Packages are discovered automatically from the directory tree.
5. **Deterministic**: Same inputs always produce the same build plan.

## Package Discovery

### Algorithm

The build system discovers packages by recursively walking the directory tree looking for BUILD files. A directory containing a BUILD file is a package. The walk skips known non-source directories for performance.

```
function discover(directory):
    if directory.name in SKIP_LIST:
        return                          # ignore junk directories

    if BUILD file exists in directory:
        register as package             # leaf node — don't recurse deeper
        return

    for each subdirectory in directory:
        discover(subdirectory)          # recurse
```

This is the same approach used by Bazel, Buck, and Pants. It requires no configuration files to route the walk — the presence of a BUILD file is sufficient.

### Skip List

The following directory names are skipped during discovery. They are known to contain non-source files that should never be treated as packages:

```
.git            # version control
.hg             # Mercurial
.svn            # Subversion
.venv           # Python virtual environments
.tox            # Python tox environments
.mypy_cache     # mypy type checker cache
.pytest_cache   # pytest cache
.ruff_cache     # ruff linter cache
__pycache__     # Python bytecode cache
node_modules    # Node.js dependencies
vendor          # vendored dependencies (Go, Ruby)
dist            # build output
build           # build output
target          # Rust/Java build output
.claude         # Claude Code worktrees and config
Pods            # CocoaPods (iOS)
```

### BUILD File Format

A BUILD file is a plain text file containing shell commands, one per line. Blank lines and lines starting with `#` are ignored. The commands are executed sequentially in the package's directory.

```
# Example BUILD file for a Python package
uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
```

```
# Example BUILD file for a Go package
go test ./... -v -cover
```

```
# Example BUILD file for a Rust package
cargo test -p logic-gates -- --nocapture
```

### Platform-Specific BUILD Files

On macOS, if `BUILD_mac` exists in a directory, it takes precedence over `BUILD`. On Linux, `BUILD_linux` takes precedence. This allows platform-specific build commands (e.g., different compiler flags).

Priority:
1. `BUILD_mac` (on macOS/Darwin only)
2. `BUILD_linux` (on Linux only)
3. `BUILD` (cross-platform fallback)

### Language Inference

The package's language is inferred from its directory path. The build system scans path components for known language names:

| Path component | Language |
|---------------|----------|
| `python`      | python   |
| `ruby`        | ruby     |
| `go`          | go       |
| `rust`        | rust     |
| `elixir`      | elixir   |

For example, `code/packages/python/logic-gates` yields language `python`. If no known language component is found, the language is `unknown`.

### Package Naming

A package's qualified name is `{language}/{dirname}`. For example:

- `code/packages/python/logic-gates` → `python/logic-gates`
- `code/packages/go/directed-graph` → `go/directed-graph`
- `code/packages/rust/arithmetic` → `rust/arithmetic`

## Dependency Resolution

The build system parses language-specific metadata files to discover inter-package dependencies:

| Language | Metadata file    | Dependency prefix           |
|----------|------------------|-----------------------------|
| Python   | `pyproject.toml` | `coding-adventures-`        |
| Ruby     | `*.gemspec`      | `coding_adventures_`        |
| Go       | `go.mod`         | module path contains repo   |
| TypeScript | `package.json` | `@coding-adventures/`       |
| Rust     | `Cargo.toml`     | workspace member path       |
| Elixir   | `mix.exs`        | `:coding_adventures_`       |

Dependencies on external packages (not in the monorepo) are silently ignored. The resolver builds a directed graph where an edge from A to B means "B depends on A" (A must build before B).

## Build Execution

### Change Detection

The primary change detection mode uses git:

```
git diff --name-only <base>...HEAD
```

This produces the list of files that changed relative to the base branch (typically `origin/main`). Changed files are mapped to packages by path prefix matching. The dependency graph is then used to find all affected packages — both directly changed packages and their transitive dependents.

### Hash-Based Fallback

When git diff is unavailable, the build system falls back to SHA256 hashing:

1. For each package, hash all source files (sorted by path for determinism).
2. Compute a "deps hash" by collecting hashes of all transitive dependencies.
3. Compare against a cache file (`.build-cache.json`).
4. If the package hash or deps hash changed, rebuild.

### Parallel Execution

The dependency graph is partitioned into independent groups — sets of packages with no dependencies between them. Groups are executed sequentially (respecting dependency order), but packages within each group run in parallel.

```
Level 0:  logic-gates  (no dependencies)
Level 1:  arithmetic, clock  (depend on logic-gates)
Level 2:  cpu-simulator  (depends on arithmetic + clock)
```

Parallelism is bounded by a configurable job count (default: number of CPU cores).

### Failure Propagation

If a package fails to build, all packages that transitively depend on it are marked "dep-skipped" and not executed. This avoids wasting time on builds that will definitely fail.

## CLI Interface

The implementations intentionally share the same overall CLI shape, but they are not perfectly feature-identical. The Go tool is the reference behavior used in CI.

```
build-tool [flags]

Flags:
  -root <path>          Repository root (auto-detect from .git if omitted)
  -diff-base <ref>      Git ref to diff against (default: origin/main)
  -force                Rebuild everything regardless of cache
  -dry-run              Show what would build without executing
  -jobs <N>             Max parallel workers (default: CPU count)
  -language <lang>      Filter: implementation-dependent; Go supports python, ruby, go, rust, typescript, elixir, all
  -cache-file <path>    Path to cache file (default: .build-cache.json)
```

## Implementations

| Language | Location                              | Parallelism      | Notes                    |
|----------|---------------------------------------|-------------------|--------------------------|
| Go       | `code/programs/go/build-tool/`        | goroutines        | Primary CI tool, broadest language support |
| Python   | `code/programs/python/build-tool/`    | ThreadPoolExecutor| Reference implementation |
| Ruby     | `code/programs/ruby/build-tool/`      | Threads           | Educational              |
| Rust     | `code/programs/rust/build-tool/`      | rayon             | Native performance       |

## Migration Note

The build system previously used DIRS files to route directory traversal. DIRS files are plain text files listing subdirectories to descend into. That mechanism has been replaced by recursive BUILD file discovery because DIRS files create merge conflicts when multiple contributors add packages in parallel. Any remaining DIRS files are legacy and ignored.
