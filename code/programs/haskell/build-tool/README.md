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
