# Build Tool (TypeScript)

An incremental, parallel monorepo build tool implemented in TypeScript. This is a port of the Python build tool, maintaining full feature parity.

## What it does

The build tool discovers packages in the monorepo by walking the directory tree looking for BUILD files, resolves inter-package dependencies by parsing language-specific metadata files, and executes builds in parallel topological order.

Discovery classifies only the exact bucket immediately below `code/packages`
or `code/programs`. It recognizes every established and emerging implementation
lane plus the repository's execution, domain, build-language, and shared .NET
host buckets. Program identities use `language/programs/name`, specification
fixtures are excluded, and residual duplicate qualified names fail closed with
the portable `DUPLICATE_PACKAGE_IDENTITY` diagnostic and CLI exit code 2.
Generated dependency and build trees are excluded by exact, case-sensitive
directory component. This includes Cabal's `dist-newstyle` and Dune's `_build`
output without excluding similarly named source directories such as `_Build`
and `_build-example`. The shared language-registry fixture and direct
Windows-safe discovery regressions enforce these boundaries.

Source hashing independently prunes the language-neutral 26-component
generated-artifact registry before both extension and declared-source
selection. The hasher tests project both neutral source-collection fixtures,
retain exact case variants and near names, and verify that directory symlinks
or Windows junctions are not traversed. This hashing policy is intentionally
separate from discovery: a source directory such as `specs` remains eligible,
while exact `_build`, `node_modules`, `.cargo`, and `cover` components do not
affect a package cache key.

## How it fits in the stack

This is one of several build tool implementations in the monorepo (Python, Ruby, Go, Rust, Elixir, TypeScript). All implementations share the same architecture and produce identical results. The Go implementation is the primary one used in CI; the others serve as educational implementations demonstrating the same concepts in different languages.

## Architecture

| Module                          | Purpose                                                                               |
| ------------------------------- | ------------------------------------------------------------------------------------- |
| `discovery.ts`                  | Walks directory tree, finds BUILD files, infers language                              |
| `resolver.ts`                   | Parses dependency metadata, builds directed graph (Kahn's algorithm)                  |
| `gitdiff.ts`                    | Git-based change detection (`git diff --name-only`)                                   |
| `hasher.ts`                     | SHA256 hashing of source files for cache-based change detection                       |
| `cache.ts`                      | JSON cache file for fallback change detection                                         |
| `executor.ts`                   | Parallel build execution respecting dependency order                                  |
| `reporter.ts`                   | Human-readable build report formatting                                                |
| `validator.ts`                  | Build-contract checks plus pure orphan-crate and tracked-artifact snapshot validation |
| `toolchain-detection.ts`        | Pure bounded extra-CI toolchain declaration evaluation                                |
| `tracked-artifact-unicode17.ts` | Generated, source-pinned Unicode 17 normalization and casing substrate                |
| `index.ts`                      | CLI entry point tying everything together                                             |

## Supported languages

The resolver parses package-manager metadata for the established monorepo lanes,
including:

- **Python**: `pyproject.toml` (`coding-adventures-*` prefix)
- **Ruby**: `.gemspec` (`coding_adventures_*` prefix)
- **Go**: `go.mod` (full module paths)
- **TypeScript**: `package.json` (`@coding-adventures/*` scoped names)
- **Rust**: `Cargo.toml` (crate names with path dependencies)
- **Elixir**: `mix.exs` (`coding_adventures_*` atom names)
- **Lua**: `.rockspec` files (`coding-adventures-*` rock names)
- **Perl**: `Makefile.PL` / `cpanfile` package references
- **Haskell**: Cabal package dependencies
- **.NET**: C# and F# project references

Lua rockspecs are decoded as strict UTF-8. Malformed metadata fails closed with
the stable `METADATA_INVALID_UTF8` diagnostic and CLI exit code 2; a valid
literal U+FFFD replacement character remains valid UTF-8.

Emitted build plans use repository-relative forward-slash package paths on
every platform, including Windows, so downstream jobs receive the same logical
plan regardless of the producer host.

Git-diff package matching uses the same repository-relative forward-slash
paths on every platform. Its integration tests create native temporary Git
repositories and invoke Git with direct argument vectors, so the package suite
runs without a POSIX shell on Windows.

## Process-free orphan-crate validation

`validateOrphanCrateSnapshot()` consumes a closed, caller-supplied snapshot of
Cargo-manifest directories, recognized BUILD records, and exemption-ledger
entries. It derives direct and ancestor BUILD coverage, ignores only exact
generated-artifact components, reports empty or unlisted crates, rejects
malformed exemptions with a fixed redacted ledger path, detects stale
exemptions, and returns the active `PENDING` count.

The adapter uses the shared fixed BUILD filename order, Unicode-scalar path
limits and ordering, Python-compatible reason whitespace and diagnostic-detail
ordering, and the source-pinned Unicode 17 NFC plus full-casefold tables for
duplicate exemption identities. It does not enumerate the checkout, inspect
the filesystem, consult Git, launch a process, read the environment, or access
the network. All four language-neutral orphan-crate fixtures enter through this
TypeScript-native API.

## Process-free tracked-artifact validation

`validateTrackedArtifactSnapshot()` consumes bounded path records supplied by a
reviewed caller. It rejects unsafe portable paths with redacted diagnostics and
detects exact, nested, case, and Unicode compatibility aliases of
`node_modules`. Backslashes are normalized lexically; diagnostic ordering uses
Unicode scalar values rather than UTF-16 code units or the host locale.

The adapter is deliberately not a repository scanner. It does not enumerate a
checkout, invoke Git, open paths, follow symlinks or reparse points, inspect the
environment, start a process, or use the network. NFC, NFKC, full default case
folding, and full uppercase come from the generated Unicode 17.0.0 module, not
Node's ambient ICU tables. Regenerate and verify that module from the repository
root with:

```bash
(cd code/programs/typescript/build-tool && npm ci)
python code/scripts/generate_tracked_artifact_unicode17.py
python code/scripts/generate_tracked_artifact_unicode17.py --check
```

Generation executes both the Python and TypeScript outputs against the pinned
official normalization, case-folding, and uppercase vectors before accepting
either artifact.

## Process-free extra-CI toolchain declarations

`evaluateToolchainSnapshot()` consumes only caller-owned package names,
languages, and the five closed platform BUILD fronts. The package-local Vitest
suite dynamically discovers every language-neutral
`toolchain-detection-*.json` fixture and evaluates it through this native
boundary. The API applies explicit darwin/Linux/Windows front precedence and
returns a fresh frozen boolean map over the sorted 16-toolchain registry.

The boundary meters UTF-8 bytes and LF-delimited logical lines before parsing,
rejects aggregate snapshots immediately when their ceiling is crossed, and
recognizes only exact bounded `# needs-toolchain: NAME` comments. Direct
callers receive stable typed shape and resource errors for sparse, duplicate,
oversized, or out-of-grammar snapshots. The evaluator does not enumerate the
checkout, read files, invoke Git, inspect the environment, start a process, or
access the network.

## Platform-specific BUILD files

The discovery system supports platform-specific BUILD files with the following priority:

| Platform        | Priority                                        |
| --------------- | ----------------------------------------------- |
| macOS (darwin)  | `BUILD_mac` > `BUILD_mac_and_linux` > `BUILD`   |
| Linux           | `BUILD_linux` > `BUILD_mac_and_linux` > `BUILD` |
| Windows (win32) | `BUILD_windows` > `BUILD`                       |

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
# Install pinned development dependencies
npm ci

# Typecheck production and adapter source without emitting generated files
npm run typecheck

# Run tests
npx vitest run

# Run tests with coverage
npx vitest run --coverage
```

The generic `BUILD` front runs dependency installation, this strict no-emit
compiler gate, and then the coverage suite in that order. Tests stay outside
the production `tsconfig.json`; every adapter added under `src/` is covered
automatically without introducing a second test-only compiler contract.

## Design decisions

- **Zero runtime dependencies**: Only uses Node.js built-in modules (`node:fs`, `node:path`, `node:crypto`, `node:child_process`, `node:os`, `node:util`).
- **Pinned Unicode policy**: The tracked-artifact validator embeds reviewed
  Unicode 17.0.0 data under Unicode License v3; distributions include
  `UNICODE-LICENSE.txt`, and package metadata declares
  `MIT AND Unicode-3.0`.
- **Portable metadata diagnostics**: Strict-decoding failures use repository-relative paths and never expose checkout-specific host paths.
- **Collision-safe discovery**: Canonical package/program identities are unique;
  duplicate diagnostics contain sorted repository-relative paths and never
  expose the checkout root.
- **Inline directed graph**: Rather than importing an external graph library, the resolver includes a minimal DirectedGraph implementation.
- **ESM-only**: Uses ES modules throughout (`"type": "module"` in package.json).
- **Literate programming**: All source files include extensive comments explaining concepts, algorithms, and design decisions.
