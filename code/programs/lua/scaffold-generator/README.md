# Scaffold Generator (Lua)

Generates CI-ready Lua package scaffolding for the coding-adventures monorepo.

## Usage

```bash
lua scaffold.lua PACKAGE_NAME [options]
```

### Arguments

- `PACKAGE_NAME` — Kebab-case package name (e.g., `logic-gates`)

### Options

- `--depends-on dep1,dep2` — Comma-separated sibling package names
- `--layer N` — Layer number for README context
- `--description "text"` — One-line package description
- `--type library|program` — Package type (default: library)
- `--dry-run` — Print what would be generated without writing

### Example

```bash
lua scaffold.lua logic-gates --layer 10 --description "The fundamental building blocks of all digital circuits"
lua scaffold.lua arithmetic --depends-on logic-gates --layer 9 --description "Integer arithmetic circuits built from logic gates"
```

## What Gets Generated

```
code/packages/lua/{snake_case_name}/
├── coding-adventures-{name}-0.1.0-1.rockspec
├── src/coding_adventures/{snake_case_name}/
│   └── init.lua
├── tests/
│   └── test_{snake_case_name}.lua
├── BUILD
├── BUILD_windows
├── README.md
└── CHANGELOG.md
```

## Dependency Resolution

When `--depends-on` is specified, the generator:

1. Validates all direct dependencies exist on disk
2. Computes transitive closure via BFS (reads each dep's rockspec)
3. Topologically sorts dependencies (Kahn's algorithm) for correct install order
4. Generates BUILD file with `luarocks make` in dependency order
