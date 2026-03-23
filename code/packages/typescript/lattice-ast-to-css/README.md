# @coding-adventures/lattice-ast-to-css

Three-pass compiler that transforms a Lattice AST (a CSS superset with variables, mixins, control flow, functions, and more) into clean CSS.

## Architecture

The package is organized into several modules:

| Module           | Purpose                                           |
|------------------|---------------------------------------------------|
| `errors.ts`      | 14 error classes (LatticeError and subclasses)     |
| `scope.ts`       | ScopeChain for lexical variable scoping            |
| `values.ts`      | LatticeValue discriminated union, arithmetic, and 37 built-in functions |
| `evaluator.ts`   | ExpressionEvaluator (compile-time expression evaluation) |
| `transformer.ts` | LatticeTransformer (the 3-pass pipeline)           |
| `emitter.ts`     | CSSEmitter (AST to CSS text)                       |

## Three-Pass Pipeline

1. **Pass 1: Symbol Collection** -- Walk the AST, collect variable/mixin/function definitions, remove them from output.
2. **Pass 2: Expansion** -- Resolve variables, expand mixins, evaluate control flow (@if/@for/@each/@while), evaluate functions, handle @content/@at-root/@extend.
3. **Pass 3: Cleanup** -- Remove empty nodes, apply @extend selector merging, splice @at-root hoisted rules.

## Lattice v2 Features

Beyond v1 (variables, mixins, @if/@for/@each, functions, modules), v2 adds:

- **@while loops** with max-iteration guard (default 1000)
- **Variables in selectors** (`.col-$i`, `$tag-name`)
- **@content blocks** for wrapping CSS in a mixin
- **!default and !global flags** for library-style variable management
- **Property nesting** (`font: { size: 14px; weight: bold; }`)
- **@at-root** for escaping nesting context
- **@extend and %placeholder selectors** for selector inheritance
- **Maps** as a first-class value type
- **37 built-in functions** across 5 categories (map, color, list, type, math)

## Usage

```typescript
import { LatticeTransformer, CSSEmitter } from "@coding-adventures/lattice-ast-to-css";
import { parseLattice } from "@coding-adventures/lattice-parser";

const ast = parseLattice("$color: red; h1 { color: $color; }");
const transformer = new LatticeTransformer();
const cssAst = transformer.transform(ast);
const emitter = new CSSEmitter();
const css = emitter.emit(cssAst);
// "h1 {\n  color: red;\n}\n"
```

## Dependencies

- `@coding-adventures/lattice-parser`
- `@coding-adventures/lattice-lexer`
- `@coding-adventures/grammar-tools`
- `@coding-adventures/parser`
- `@coding-adventures/lexer`

## Development

```bash
# Run tests
npx vitest run

# Run tests with coverage
npx vitest run --coverage
```
