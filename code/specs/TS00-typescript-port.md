# TS00 — TypeScript Port Specification

## Overview

This specification covers the porting of all existing packages in the computing
stack to TypeScript, adding JavaScript grammar files for the grammar-driven
lexer/parser, and creating cross-language lexer/parser packages.

## Package Conventions

### Naming

- **npm scope**: `@coding-adventures/<package-name>` (e.g., `@coding-adventures/logic-gates`)
- **Directory names**: kebab-case under `code/packages/typescript/` (e.g., `logic-gates/`)
- **Build system name**: `typescript/<package-name>` (e.g., `typescript/logic-gates`)

### Structure

Every TypeScript package follows this layout:

```
<package-name>/
├── BUILD                 # Build/test commands for CI
├── CHANGELOG.md          # Version history
├── README.md             # Package documentation
├── package.json          # npm manifest
├── tsconfig.json         # TypeScript config (extends base)
├── vitest.config.ts      # Test configuration
├── src/
│   ├── index.ts          # Public API re-exports
│   └── <modules>.ts      # Implementation files
└── tests/
    └── <modules>.test.ts # Test files
```

### Dependencies

Internal dependencies use `file:` references in `package.json`:

```json
{
  "dependencies": {
    "@coding-adventures/logic-gates": "file:../logic-gates"
  }
}
```

### Build System Integration

- **BUILD file**: `npm ci --quiet && npx vitest run --coverage`
- **DIRS entry**: Each package listed in `code/packages/typescript/DIRS`
- **Language detection**: The Go build tool infers `"typescript"` from the path
- **Dependency resolution**: Reads `package.json` `dependencies`, maps
  `@coding-adventures/<name>` to `typescript/<name>`

### Testing

- **Framework**: Vitest with V8 coverage provider
- **Coverage threshold**: 80% minimum (target 95%+ for libraries)
- **Config**: Each package has `vitest.config.ts` referencing `@vitest/coverage-v8`

### TypeScript Configuration

Shared base config at `code/packages/typescript/tsconfig.base.json`:

- **Target**: ES2022
- **Module**: Node16
- **Strict mode**: enabled
- **Declaration files**: generated for downstream consumers

### Literate Programming

All source files follow Knuth-style literate programming:

- Explanatory comments describe the "why" and teach concepts
- Truth tables, diagrams, and examples appear inline
- Someone new to programming should be able to learn from reading the source

## Packages to Port

### Layer 1 — No Dependencies

| Package | Description |
|---------|-------------|
| `logic-gates` | Boolean logic gates + sequential circuits |
| `directed-graph` | Graph library: topological sort, cycle detection |
| `grammar-tools` | Parses `.tokens` and `.grammar` files |
| `fp-arithmetic` | IEEE 754 floating-point simulation |

### Layer 2 — Depends on Layer 1

| Package | Dependencies |
|---------|--------------|
| `arithmetic` | `logic-gates` |
| `lexer` | `grammar-tools` |

### Layer 3

| Package | Dependencies |
|---------|--------------|
| `parser` | `lexer` |
| `cache` | `arithmetic` |
| `branch-predictor` | (none) |
| `hazard-detection` | (none) |

### Layer 4–5

| Package | Dependencies |
|---------|--------------|
| `bytecode-compiler` | `parser` |
| `cpu-simulator` | `arithmetic` |
| `virtual-machine` | `bytecode-compiler` |
| `pipeline` | `cache`, `branch-predictor`, `hazard-detection`, `cpu-simulator` |
| `assembler` | (minimal) |

### Layer 6 — ISA Simulators + Utilities

| Package | Description |
|---------|-------------|
| `riscv-simulator` | RISC-V RV32I instruction set |
| `arm-simulator` | ARMv7 subset |
| `wasm-simulator` | WebAssembly bytecode |
| `intel4004-simulator` | Intel 4004 (4-bit, 1971) |
| `jvm-simulator` | Java Virtual Machine bytecode |
| `clr-simulator` | .NET CLR bytecode |
| `html-renderer` | HTML visualization reports |
| `clock` | Clock cycle simulation |

### Cross-Language Packages

| Package | Target Language |
|---------|----------------|
| `python-lexer` | Tokenizes Python via `python.tokens` |
| `python-parser` | Parses Python via `python.grammar` |
| `ruby-lexer` | Tokenizes Ruby via `ruby.tokens` |
| `ruby-parser` | Parses Ruby via `ruby.grammar` |
| `javascript-lexer` | Tokenizes JavaScript via `javascript.tokens` |
| `javascript-parser` | Parses JavaScript via `javascript.grammar` |

## JavaScript Grammar Files

New grammar files at `code/grammars/`:

- `javascript.tokens` — Token definitions for a subset of JavaScript
- `javascript.grammar` — EBNF grammar rules for simple JS expressions

The grammar supports the same simple subset as Python and Ruby: variable
assignment, arithmetic expressions (`x = 1 + 2`), and basic control flow
keywords. JavaScript-specific additions include `let`/`const`/`var` declarations,
semicolons, `===`/`!==` strict equality, and curly braces.

## Porting Guidelines

1. **Reference implementation**: Use the Python version as the primary reference.
   The Go version shows how Python idioms were adapted to a statically-typed language.

2. **Type mapping**:
   - Python `@dataclass(frozen=True)` → TypeScript `readonly` interfaces
   - Python `Enum` → TypeScript `enum` or string literal unions
   - Python `dict[K, V]` → TypeScript `Map<K, V>` or `Record<K, V>`
   - Python `set[T]` → TypeScript `Set<T>`
   - Python `match/case` → TypeScript discriminated unions with `switch`

3. **Bit manipulation**: Use JavaScript bitwise operators for logic gates and
   arithmetic. Use `BigInt` where Python uses arbitrary-precision integers
   (fp-arithmetic).

4. **Path resolution**: Use `import.meta.url` with `path.join()` to locate
   grammar files relative to the package source.

5. **Module system**: ES modules (`"type": "module"` in `package.json`).
