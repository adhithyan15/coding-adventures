# starlark-interpreter

Full Starlark pipeline: source code -> lexer -> parser -> compiler -> VM -> result.

This package chains together every layer of the Starlark compilation and execution stack into a single API, and adds `load()` support with file resolution and module caching.

## Where It Fits

```
Source Code (string)
    |
    v
starlark-lexer: tokenize
    |
    v
starlark-parser: parse into AST
    |
    v
starlark-ast-to-bytecode-compiler: compile to bytecode
    |
    v
starlark-vm: execute bytecode  <-- default LOAD_MODULE is a no-op
    |
    v
THIS PACKAGE: overrides LOAD_MODULE to resolve + execute loaded files
    |
    v
StarlarkResult { Variables, Output, Traces }
```

## Usage

### Simple execution

```go
import starlarkinterpreter "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-interpreter"

result, err := starlarkinterpreter.Interpret("x = 1 + 2\n")
fmt.Println(result.Variables["x"])  // 3
```

### With load() support

```go
files := map[string]string{
    "helpers.star": "def double(n):\n    return n * 2\n",
}
resolver := starlarkinterpreter.DictResolver(files)

result, err := starlarkinterpreter.Interpret(
    `load("helpers.star", "double")` + "\n" +
    "x = double(21)\n",
    resolver,
)
fmt.Println(result.Variables["x"])  // 42
```

### Struct API for more control

```go
interp := starlarkinterpreter.NewInterpreter(
    starlarkinterpreter.WithFileResolver(resolver),
    starlarkinterpreter.WithMaxRecursionDepth(500),
)
result, err := interp.Interpret(source)
```

### Execute a file from disk

```go
result, err := starlarkinterpreter.InterpretFile("build.star", resolver)
```

## Key Features

- **Full pipeline**: lexer -> parser -> AST -> bytecode -> VM execution
- **load() support**: resolve files via `FileResolver`, execute them, extract symbols
- **Module caching**: each loaded file is executed once and cached (like Python's `sys.modules`)
- **Functional options**: configure with `WithFileResolver()`, `WithMaxRecursionDepth()`
- **DictResolver**: test-friendly file resolution from a `map[string]string`

## Dependencies

- `starlark-ast-to-bytecode-compiler` -- compiles Starlark source to bytecode
- `starlark-vm` -- executes bytecode with all 59 opcode handlers and 23 builtins
- `virtual-machine` -- the generic stack-based VM engine
