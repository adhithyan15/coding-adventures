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
| `BuildTool.Validator` | Pure build-contract, orphan-crate, and tracked-artifact validation |
| `BuildTool.TrackedArtifactUnicode17` | Generated, source-embedded Unicode 17 policy tables |

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

## Orphan-crate validation

`BuildTool.Validator.validate_orphan_crate_snapshot/1` accepts one closed,
caller-supplied snapshot of Cargo manifest directories, relevant BUILD states,
and `code/BUILD-EXEMPTIONS` records. It returns the shared validation result and
deterministically ordered diagnostics without enumerating a checkout, opening a
path, invoking Git, reading environment state, launching a process, or using
the network.

Coverage is component-wise: a runnable BUILD in the manifest directory or an
ancestor through `code/` covers the manifest, and a nearer empty BUILD cannot
mask a runnable ancestor. Exact artifact components such as `target`,
`node_modules`, `_build`, `deps`, `.build`, `dist-newstyle`, and `.cargo` stay
outside the bounded scan. Uncovered manifests produce unlisted or empty-BUILD
diagnostics unless a valid active `EXCLUDED` or `PENDING` record applies.

Exemption paths use the same portable, host-independent path policy on every
platform. Unsafe values are redacted to `code/BUILD-EXEMPTIONS`; aliases are
detected with the generated Unicode 17 NFC and full-fold tables; Windows
reserved names use the generated full-uppercase table; and blank reasons match
Python's exact whitespace set. Stale exemptions remain visible, while the
result separately reports the count of valid non-stale `PENDING` entries.

## Tracked-artifact validation

`BuildTool.Validator.validate_tracked_artifact_snapshot/1` accepts bounded,
caller-supplied index records and returns deterministic diagnostics without
opening paths, invoking Git, reading environment state, launching a process, or
using the network. The `/2` form also accepts the required Unicode version and
rejects anything other than `17.0.0` before inspecting the entries.

The validator lexically normalizes separators, rejects unsafe portable paths,
redacts invalid paths to `repository`, and rejects every exact, nested, case,
or Unicode compatibility alias of a `node_modules` component. Safe forbidden
paths remain visible, entry kinds are inert metadata, and diagnostics sort by
Unicode scalar values plus canonical detail text.

`BuildTool.TrackedArtifactUnicode17` is generated from exact size- and
SHA-256-pinned Unicode Consortium inputs. It provides NFC, NFKC, full default
case folding, NFKC-fold, and full uppercase without inheriting the installed
Elixir or Erlang runtime's Unicode tables. From the repository root, regenerate
and byte-check every language target with:

```bash
(cd code/programs/typescript/build-tool && npm ci)
python code/scripts/generate_tracked_artifact_unicode17.py
python code/scripts/generate_tracked_artifact_unicode17.py --check
```

The generator runs emitted Python, TypeScript, Ruby, and Elixir code over every
official normalization, case-folding, and unconditional uppercase vector plus
the Unicode 17 version sentinels. The generated data is redistributed under the
Unicode License v3; the complete notice is shipped as `UNICODE-LICENSE.txt`.
CI repeats the real emitted-Elixir check on the pinned Elixir 1.18.4 / OTP
27.3.4.11 toolchain with
`--check --self-check-runtime elixir`; omitting `--self-check-runtime` retains
the default local gate across every emitted runtime.

## Testing

```bash
mix deps.get
mix test
mix test --cover
```

`test/validator_test.exs` consumes all four language-neutral orphan-crate
fixtures and all five tracked-artifact fixtures. Focused tests cover exemption
precedence, hostile-path redaction, Python blank reasons and ASCII-JSON detail
ordering, rooted BUILD selection, Unicode 17 full-fold aliases, tracked version
drift, scalar path boundaries, pinned Unicode sentinels, and inert entry kinds.

## Dependencies

- **Jason** — JSON encoding/decoding for the build cache
- **CodingAdventures.ProgressBar** — Live terminal progress bar (from this monorepo)
