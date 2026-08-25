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
| `-language` | all | Filter to any canonical discovery bucket, or `all` |
| `-diff-base` | origin/main | Git ref to diff against for change detection |
| `-cache-file` | .build-cache.json | Path to the build cache file |
| `-validate-build-files` | true | Validate BUILD dependency metadata, crate coverage, and tracked artifacts |

`-validate-build-files` also fails closed when Git tracks any `node_modules`
path. Dependency directories are machine-local build products; committing one
can hide an absolute symlink that works only in its author's checkout.

## Metadata safety

A package's CI toolchain is normally inferred purely from its path bucket
(`packages/<language>/...`) — but a package whose own tests need a
*different* toolchain (e.g. a Rust crate under `packages/rust/**` that
shells out to a `javac`/`java` process) can declare that with a bare
BUILD-file comment: `# needs-toolchain: java`. This is consulted alongside
the inferred language, not instead of it, and multiple lines are supported
for a package needing more than one extra toolchain.

Build plans carry optional platform-specific dependency graphs and affected
closures. The detect job resolves Linux, Darwin, and Windows BUILD metadata,
unions their required toolchains, and assigns shared shards from the union so
platform-only work cannot disappear from the matrix. Each build runner then
selects only its own graph and affected set. A Windows-only native prerequisite
is therefore installed and ordered on Windows without causing unrelated Unix
packages to build. Older v1 plans without platform overrides continue to use
the top-level graph and affected set.

Text package metadata is decoded according to the language-neutral build-tool
contract. In particular, Lua `.rockspec` files must be strict UTF-8. Invalid
bytes stop resolution with `METADATA_INVALID_UTF8`, identify the package and
repository-relative manifest, and return CLI exit code 2 without exposing the
checkout path or silently replacing input.

Lua standalone-build validation follows the shared language-neutral fixture
contract. When a canonical `BUILD` installs repository-local sibling rocks,
an absent `BUILD_windows` is an empty Windows closure rather than a reason to
skip comparison. The validator reports every missing canonical sibling in
stable sorted order; an explicit Windows recipe may add transitive local rocks.

Cargo dependency resolution reads inline path dependencies from the top-level
`[dependencies]` table. When a dependency uses a local source alias together
with `package = "published-name"`, the published package name drives internal
lookup. Package metadata, features, dev/build/target tables, and registry-only
dependencies remain outside this graph contract.

Ruby dependency resolution reads only `add_dependency` and
`add_runtime_dependency` calls on the gem specification receiver. Development
dependencies, metadata, and comments remain outside the graph. Declared gem
names are registered alongside directory-derived aliases, so valid
hyphen/underscore naming differences resolve deterministically. The shared
field-boundary fixture and complete 301-package lane resolve exactly 454 edges.

Perl dependency resolution reads only top-level runtime `requires`
declarations from each root `cpanfile`. Requirements inside test or other
phase blocks, `Makefile.PL` dependency tables, metadata, and comments remain
outside the graph. Exact `Makefile.PL` `NAME` values and current and legacy
distribution spellings are aliases only. The shared field-boundary fixture and
complete 256-package lane resolve exactly 217 total edges: 216 from
authoritative manifests and one qualified BUILD dependency.

Elixir dependency resolution reads local dependency tuples only from direct
project `deps:` lists and lists returned by block or shorthand `defp deps`
functions. Multiline tuples are accepted when they contain a quoted `path:`
option. Project and application metadata, source prose, comments, `mix.lock`,
and non-path Hex or Git dependencies cannot invent graph edges. The shared
field-boundary fixture covers direct, block, shorthand, multiline, comment,
metadata, and external-dependency cases; the complete 282-package lane resolves
exactly 472 edges.

Dart dependency resolution reads only direct package keys under the root
`dependencies:` and `dev_dependencies:` maps. Scalar constraints and nested
source maps are both valid direct entries, while nested `path:`, `git:`,
`url:`, `ref:`, and `sdk:` options, dependency overrides, comments, and
unrelated YAML fields cannot invent graph edges. Root `name:` values and
directory-derived snake-case aliases identify packages. The shared
field-boundary fixture covers these boundaries, and the complete 82-package
lane resolves exactly 67 edges.

TypeScript dependency resolution parses the root `package.json` and reads only
direct property names from `dependencies` and `devDependencies`. Exact
top-level `name` strings and directory-derived scoped or unscoped names supply
aliases. Dependency values are not followed, and peer or optional dependency
tables, scripts, nested tool configuration, nested names, and malformed JSON
cannot invent partial graph edges. The shared field-boundary fixture covers
single-line objects and adversarial decoys; the complete 470-package lane
resolves exactly 1,076 edges.

Java and Kotlin dependency resolution reads only `includeBuild("...")` calls
from root `settings.gradle.kts` files. Calls may span lines; their quoted
relative paths are normalized lexically and must exactly match a discovered
package root in the same language lane. Comments, unrelated strings,
`include(...)`, build-script coordinates, absolute paths, cross-lane paths,
and unknown targets cannot invent graph edges. Referenced targets are never
opened. Java/Kotlin source and Gradle settings/build files participate in the
package hash. The two shared field-boundary fixtures cover direct and nested
paths plus adversarial decoys; the complete Java and Kotlin lanes resolve
exactly 186 and 166 edges respectively.

C#, F#, and shared .NET dependency resolution reads only literal `Include`
attributes on unqualified `ProjectReference` start elements in root `.csproj`
and `.fsproj` files. Relative paths use portable separators, normalize
lexically, and must exactly match another discovered root project file in the
shared .NET scope. Comments, CDATA, processing instructions, escaped examples,
namespaced elements, package references, MSBuild properties, globs, absolute
paths, nested test projects, and unknown targets cannot invent graph edges;
referenced files are never opened. Three shared fixtures cover both language
lanes and cross-language references. The complete 198-package C# and
197-package F# lanes resolve exactly 238 and 239 edges respectively.

## Canonical discovery identities

Discovery uses only the exact bucket immediately below a `packages` or
`programs` path component. It recognizes every established implementation
lane, the emerging C, C++, and OCaml lanes, the WASM target, the Mosaic and
Twig domain languages, the Starlark build language, and the retained shared
`.NET` host bucket. Programs keep a `programs` segment, such as
`go/programs/build-tool`, so they cannot collide with a library of the same
name. Specification fixture trees are not buildable packages.

Discovery fails closed when two directories normalize to the same identity:

```text
DUPLICATE_PACKAGE_IDENTITY: package=unknown/demo paths=code/packages/alpha/demo,code/packages/beta/demo
```

The diagnostic contains sorted repository-relative paths, never the checkout
root, and the CLI returns exit code `2`.

## Architecture

The tool is organized into seven internal packages, each responsible for one phase of the build pipeline:

1. **discovery** -- Recursively walks for `BUILD` files to find packages
2. **resolver** -- Parses ecosystem metadata including `pyproject.toml`,
   `.gemspec`, `go.mod`, `Cargo.toml`, `package.json`, `mix.exs`, `.rockspec`,
   `pubspec.yaml`, Cabal files, Gradle files, and .NET project references
3. **hasher** -- SHA256 hashing for change detection
4. **cache** -- JSON-based build cache (read/write with atomic saves)
5. **executor** -- Parallel execution with goroutines + semaphore
6. **gitdiff** -- Git-based change detection for incremental builds
7. **reporter** -- Terminal-friendly build report formatting

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
