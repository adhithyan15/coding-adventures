# Starlark Parser (TypeScript)

Parses Starlark source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads `starlark.grammar` and delegates all parsing to the generic engine.

## What Is Starlark?

Starlark is a deterministic, hermetic dialect of Python designed by Google for the Bazel build system. It is the language used in BUILD files, .bzl files, and other build system configuration. Starlark intentionally omits features like `class`, `import`, `while`, and recursion to guarantee termination and reproducibility.

## Usage

```typescript
import { parseStarlark } from "@coding-adventures/starlark-parser";

const ast = parseStarlark("x = 1 + 2");
console.log(ast.ruleName); // "file"
```

## What Can It Parse?

- **Assignments** — `x = 1`, `x += 1`
- **Function definitions** — `def f(a, b=1): ...`
- **If/elif/else** — conditional blocks with indentation
- **For loops** — `for x in items: ...` (no while loops in Starlark)
- **Load statements** — `load("//path", "symbol")`
- **Function calls with named args** — `cc_library(name = "foo", srcs = ["foo.cc"])`
- **List and dict literals** — `[1, 2, 3]`, `{"a": 1}`
- **Comprehensions** — `[x * 2 for x in range(10)]`
- **Full operator precedence** — from lambda to primary expressions

## Dependencies

- `@coding-adventures/starlark-lexer` -- tokenizes Starlark source code
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
