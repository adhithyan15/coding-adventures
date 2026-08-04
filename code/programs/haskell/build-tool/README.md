# build-tool

Haskell implementation of the monorepo build tool.

## What it does

This version discovers packages by walking `code/`, resolves internal
dependencies from package manifests, hashes package inputs for incremental
builds, uses git diff information to narrow the build set, and executes
`BUILD` scripts in dependency order.

Lua rockspec metadata is read as raw bytes and decoded as strict UTF-8 before
dependency resolution. Malformed bytes fail closed with package and
repository-relative manifest identity:

```text
METADATA_INVALID_UTF8: package=lua/pkg manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec encoding=UTF-8
```

The standalone front door writes that diagnostic to stderr and exits `2`.
The package tests consume the shared `resolution/lua-utf8` and
`resolution/lua-invalid-utf8` fixtures, cover representative malformed UTF-8
classes, and distinguish a valid literal U+FFFD from a decoder replacement.

Lua dependency resolution reads quoted package requirements only from the
authoritative `dependencies = { ... }` table. Package declarations,
descriptions, source metadata, comments, and unrelated strings cannot invent
graph edges. Selected `# build-tool: deps=` metadata is merged separately,
program identities retain the `language/programs/name` segment, and a package
alias wins over a same-basename program alias. Shared fixtures cover the
field-aware boundary, genuine cycles, legacy BUILD dependencies, and
package/program alias collisions.

Haskell dependency resolution reads package requirements only from Cabal
`build-depends` fields, including indented comma continuations and repeated
fields across stanzas. Synopsis and description text, source directories,
compiler options, comments, and other fields cannot invent graph edges. The
shared `resolution-haskell-field-aware` fixture covers both inline and
multiline dependencies plus representative non-authoritative collisions. The
complete 205-package Haskell lane matches the Go oracle edge-for-edge.

Python dependency resolution reads only the PEP 621 `[project]` table's
`dependencies` array. Build-system requirements, optional dependency groups,
tool tables, package metadata, and comments cannot invent graph edges. Python
distribution names are lowercased and normalize every run of `-`, `_`, or `.`
to one hyphen before lookup, so repository packages resolve consistently with
PEP 503 naming. Shared fixtures cover the canonical diamond, field boundaries,
version and environment markers, and mixed-separator aliases. The complete
488-package Python lane matches the Go oracle at 1,118 edges.

Rust dependency resolution reads only inline, path-based entries in the
top-level Cargo `[dependencies]` table. Package metadata, features, workspace
fields, dev/build/target dependency tables, comments, and registry-only
dependencies cannot invent graph edges. Renamed dependencies use the inline
`package = "..."` value rather than the local source alias. The shared
field-boundary fixture and complete 948-package lane match the canonical Go
resolver exactly at 2,373 edges.

Ruby dependency resolution reads only `add_dependency` and
`add_runtime_dependency` calls on the `Gem::Specification.new` block receiver.
Development dependencies, package metadata, file lists, comments, and unrelated
text cannot invent graph edges. Both quote forms and optional call parentheses
are accepted, and declared gem names provide authoritative aliases when a
directory-derived name differs. The shared field-boundary fixture and complete
301-package lane match the canonical Go resolver exactly at 454 edges.

Package hashing reads included files as raw bytes. Repository-relative paths
are normalized to `/`, encoded explicitly as UTF-8, and combined with those
bytes using the existing boundary framing before `git hash-object` receives
the payload through binary standard input. Source bytes therefore do not pass
through the host locale: valid Unicode, NUL, and malformed text bytes hash
deterministically on Windows, macOS, and Linux.

## Development

```bash
# Run tests and build the executable
bash BUILD

# Windows: run the command recorded in BUILD_windows
cabal test all
```

## Usage

```bash
# Build whatever changed from origin/main
cabal run build-tool -- --language haskell

# Dry-run the repo-wide plan
cabal run build-tool -- --dry-run --emit-plan
```
