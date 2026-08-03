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
