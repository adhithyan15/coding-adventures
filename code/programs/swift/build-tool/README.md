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
