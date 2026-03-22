# @coding-adventures/starlark-interpreter

Complete Starlark interpreter -- chains lexer, parser, compiler, and VM together with `load()` support for module imports.

## What It Does

This package implements the full Starlark execution pipeline:

```
source code -> tokens -> AST -> bytecode -> execution
     |            |        |        |           |
   lexer       parser  compiler    VM     interpreter
```

The interpreter adds the critical `load()` function that makes BUILD files work. When a Starlark program calls `load("//rules/python.star", "py_library")`, the interpreter:

1. **Resolves** the path to actual file contents (via a configurable file resolver).
2. **Executes** the file through the same pipeline (recursive interpretation).
3. **Extracts** the requested symbols from the result.
4. **Injects** them into the current scope.

## How It Fits in the Stack

```
Layer 9:  starlark-interpreter  <-- THIS PACKAGE
Layer 8:  starlark-vm           (Starlark-specific VM configuration)
Layer 7:  starlark-compiler     (AST -> Starlark bytecode)
Layer 6:  starlark-parser       (source -> AST)
Layer 5:  starlark-lexer        (source -> tokens)
Layer 4:  virtual-machine       (generic bytecode execution)
Layer 3:  bytecode-compiler     (generic AST -> bytecode)
Layer 2:  parser                (generic grammar-driven parsing)
Layer 1:  lexer                 (generic tokenization)
```

## Usage

### Execute Pre-compiled Bytecode

```typescript
import { interpretBytecode, Op } from "@coding-adventures/starlark-interpreter";

const code = {
  instructions: [
    { opcode: Op.LOAD_CONST, operand: 0 },
    { opcode: Op.STORE_NAME, operand: 0 },
    { opcode: Op.HALT },
  ],
  constants: [42],
  names: ["x"],
};

const result = interpretBytecode(code);
console.log(result.variables["x"]); // 42
```

### With a Compiler (when starlark-ast-to-bytecode-compiler is available)

```typescript
import { interpret } from "@coding-adventures/starlark-interpreter";
import { compileStarlark } from "@coding-adventures/starlark-ast-to-bytecode-compiler";

const result = interpret("x = 1 + 2\nprint(x)\n", {
  compileFn: compileStarlark,
});
console.log(result.variables["x"]); // 3
console.log(result.output);         // ["3"]
```

### With load() Support

```typescript
import { StarlarkInterpreter } from "@coding-adventures/starlark-interpreter";
import { compileStarlark } from "@coding-adventures/starlark-ast-to-bytecode-compiler";

const interp = new StarlarkInterpreter({
  compileFn: compileStarlark,
  fileResolver: {
    "//rules/math.star": "def double(n):\n    return n * 2\n",
  },
});

const result = interp.interpret(
  'load("//rules/math.star", "double")\nresult = double(21)\n'
);
console.log(result.variables["result"]); // 42
```

## Key Concepts

### File Resolvers

The interpreter does not know where files live on disk. A **file resolver** maps labels to file contents:

- **Dict resolver** -- a plain object for testing: `{ "//rules/test.star": "x = 42" }`
- **Function resolver** -- a function for production: `(label) => fs.readFileSync(label)`

### Load Caching

Each file is evaluated **at most once**. Subsequent `load()` calls return cached symbols. This matches Bazel semantics where loaded files are frozen after first evaluation. Use `interp.clearCache()` to force re-evaluation.

### Mini VM

This package includes a mini Starlark VM with basic opcode handlers for testing. When the full `starlark-vm` TypeScript package is available, pass its `createStarlarkVM` function instead.

## Dependencies

- `@coding-adventures/virtual-machine` -- GenericVM for bytecode execution
- `@coding-adventures/bytecode-compiler` -- CodeObject types
- `@coding-adventures/starlark-lexer` -- Starlark tokenization
- `@coding-adventures/starlark-parser` -- Starlark parsing
