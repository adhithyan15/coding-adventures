# build-tool

Haskell implementation of the monorepo build tool.

## What it does

This version discovers packages by walking `code/`, resolves internal
dependencies from package manifests, hashes package inputs for incremental
builds, uses git diff information to narrow the build set, and executes
`BUILD` scripts in dependency order.

## Development

```bash
# Run tests and build the executable
bash BUILD
```

## Usage

```bash
# Build whatever changed from origin/main
cabal run build-tool -- --language haskell

# Dry-run the repo-wide plan
cabal run build-tool -- --dry-run --emit-plan
```
