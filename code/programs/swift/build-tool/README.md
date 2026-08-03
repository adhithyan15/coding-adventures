# coding-adventures-build-tool (Swift)

An incremental, parallel monorepo build tool implemented in Swift.

## What it does

This port mirrors the other build-tool implementations in the repo:

1. Discovers packages by recursively walking `BUILD` files under `code/`
2. Evaluates simple Starlark-style BUILD targets used in this monorepo
3. Resolves internal dependencies across Python, Ruby, Go, TypeScript, Rust, Elixir, Lua, Perl, and Swift
4. Detects changed packages from `git diff`
5. Hashes package sources and dependency state for cache fallback
6. Builds independent packages in parallel by dependency level
7. Emits and consumes JSON build plans for CI
8. Validates the CI full-build toolchain contract

## Metadata safety

Lua `.rockspec` files are decoded as strict UTF-8 before dependency parsing.
Invalid bytes stop resolution, return CLI exit code `2`, and emit a stable
diagnostic with package and repository-relative manifest identity:

```text
METADATA_INVALID_UTF8: package=lua/pkg manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec encoding=UTF-8
```

The resolver tests consume the language-neutral `resolution/lua-utf8` and
`resolution/lua-invalid-utf8` fixtures, require the exact success edge set,
and verify that diagnostics never expose the checkout root.

## Usage

```bash
# Auto-detect the repo root
swift run build-tool

# Dry-run only the affected packages
swift run build-tool --dry-run

# Rebuild everything
swift run build-tool --force

# Limit parallel jobs
swift run build-tool --jobs 4

# Only consider Swift packages
swift run build-tool --language swift

# Emit a CI build plan
swift run build-tool --emit-plan build-plan.json
```

## Development

```bash
cd code/programs/swift/build-tool
swift test
swift run build-tool --help
```
