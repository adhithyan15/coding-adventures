# 12 — Build System

## Overview

The build system discovers, resolves, and builds packages across a
multi-language monorepo. The Go implementation is the primary tool used in CI
and is currently the broadest operational reference.

Executable front doors also exist in C#, Elixir, F#, Haskell, Lua, Perl,
Python, Ruby, Rust, Swift, and TypeScript. Dart, Java, and Kotlin are established
package lanes that still need build-tool implementations. F# currently delegates
to the C# engine. C and C++ are emerging lanes, and OCaml is planned as an
emerging lane.

These implementations are not yet feature-identical. Observable portable
behavior is defined by [the build-tool conformance
contract](build-tool-conformance.md), not by directory presence or this
architecture overview.

The build system is not part of the computing stack — it is the infrastructure that builds and tests the stack.

## Design Goals

1. **Incremental**: Only rebuild packages that changed (via git diff or hash comparison).
2. **Parallel**: Independent packages build concurrently.
3. **Multi-language**: The primary tool discovers every established
   implementation lane and the explicitly supported emerging lanes.
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

Platform-specific shell files take precedence over shared Unix, Starlark, and
generic fallbacks. The canonical order is specified and tested in
`build-tool-conformance.md`. In particular, a Windows `BUILD_windows` override
must win over a Starlark plan; current implementations that do otherwise are
non-conforming.

An override is a complete standalone recipe, not a patch applied to the
canonical `BUILD`. It must therefore preserve every canonical repository-local
prerequisite and install the complete closure in dependency order before
building the package itself.
Lua packages whose canonical recipe installs sibling rocks require a
`BUILD_windows` recipe with the same sibling closure, Windows path and redirect
syntax, and equivalent dependency-resolution hardening. Each BUILD line runs
in a separate shell process, so no prerequisite may rely on a previous line's
working-directory change.

### Language Inference

The package's language is inferred from its directory path. The build system scans path components for known language names:

| Classification | Path components |
|---|---|
| Established implementation | `csharp`, `dart`, `elixir`, `fsharp`, `go`, `haskell`, `java`, `kotlin`, `lua`, `perl`, `python`, `ruby`, `rust`, `swift`, `typescript` |
| Emerging implementation | `c`, `cpp`; later `ocaml` |
| Shared execution/build buckets | `dotnet`, `wasm`, `starlark` |

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

## BUILD Validation

`--validate-build-files` must reject source that the discovery graph would
silently omit. In particular, every supplied Cargo-manifest directory must be
covered by a runnable `BUILD`, `BUILD_windows`, `BUILD_mac`, `BUILD_linux`, or
`BUILD_mac_and_linux` in that directory or an ancestor beneath `code/`. A file
containing only blanks or comments is not runnable and cannot be used as a
one-touch bypass. Generated and vendored directories use the exact artifact
skip registry defined by the language-neutral conformance contract.

Intentionally uncovered manifests are policy data in
`code/BUILD-EXEMPTIONS`. Each `EXCLUDED` or `PENDING` entry requires a portable
repository-relative path and a non-empty reason. Duplicate, unsafe,
out-of-scan, artifact, unknown-kind, and reasonless entries are errors. An
entry is also an error after its directory disappears, its Cargo manifest is
removed, or a runnable BUILD begins covering it. This makes PENDING debt
countable and prevents the ledger from silently outliving the gap.

Portable engines consume the closed `orphan_crate_coverage` snapshot in
[`build-tool-conformance.md`](build-tool-conformance.md#13-closed-pure-domain-fixture-model).
The neutral check receives normalized manifest, directory, BUILD-state, and
ledger records as inert data. It performs no filesystem walk, Git query,
process launch, environment lookup, or network access; native enumeration and
file reading stay outside the process-free oracle.

Tracked dependency artifacts are a separate validation boundary. The closed
`tracked_artifact_absence` snapshot supplies bounded path and entry-kind
records as inert data. Portable engines normalize separators, reject unsafe
paths without echoing them, and reject every case or Unicode compatibility
alias of a `node_modules` path component. Regular files, symlinks, and reparse
records are classified identically and never opened or followed. Native Git
index enumeration and host-filesystem metadata collection stay outside the
language-neutral process-free oracle.

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

The implementations intentionally share the same overall CLI shape, but they
are not perfectly feature-identical. The Go tool is the reference behavior used
in CI. The portable parser subset is the bounded long-form grammar in
[`build-tool-conformance.md`](build-tool-conformance.md#12-cli-and-reporting).
That subset is a process-free contract: it normalizes supplied tokens and
deterministic null/default sentinels without consulting the host. Native tools
may retain additional spellings, but portable conformance is measured only
against that closed grammar.

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

The portable form uses `--root`, `--diff-base`, `--force`, `--dry-run`,
`--jobs`, `--language`, `--cache-file`, `--validate-build-files`,
`--no-validate-build-files`, `--detect-languages`, `--emit-plan`,
`--plan-file`, `--shard-count`, `--shard-index`, `--emit-shard-matrix`, and
`--clippy`. Reserved adapter flags and shell, environment, redirection,
command-substitution, and response-file syntax are data errors; parsers must
not expand or execute them.

## Implementations

| Status | Languages | Notes |
|---|---|---|
| Operational front doors | C#, Elixir, F#, Go, Haskell, Lua, Perl, Python, Ruby, Rust, Swift, TypeScript | Behavior and maturity vary; Go is the CI reference. |
| Missing established front doors | Dart, Java, Kotlin | Required for final parity. |
| Emerging/future | C, C++, OCaml | Graduation requires an explicit applicability decision and conformance. |

The exact implementation inventory and remediation order live in
`package-parity-roadmap.md`.

## Migration Note

The build system previously used DIRS files to route directory traversal. DIRS files are plain text files listing subdirectories to descend into. That mechanism has been replaced by recursive BUILD file discovery because DIRS files create merge conflicts when multiple contributors add packages in parallel. Any remaining DIRS files are legacy and ignored.
