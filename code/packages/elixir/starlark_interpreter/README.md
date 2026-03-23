# Starlark Interpreter

The top-level execution pipeline for Starlark programs. Chains together the
lexer, parser, compiler, and VM into a single `interpret/2` call, and adds
the critical `load()` function that makes BUILD files work.

## Where It Fits in the Stack

```
                    ┌─────────────────────────┐
                    │  starlark_interpreter    │  <-- YOU ARE HERE
                    │  (this package)          │
                    └────────────┬────────────┘
                                 │
              ┌──────────────────┼──────────────────┐
              │                  │                    │
    ┌─────────▼──────┐ ┌────────▼────────┐ ┌────────▼────────┐
    │  starlark_vm   │ │starlark_ast_to_ │ │bytecode_compiler│
    │  (execution)   │ │bytecode_compiler│ │  (generic)      │
    └───────┬────────┘ │  (compilation)  │ └─────────────────┘
            │          └────────┬────────┘
            │                   │
            └───────────────────┘
                      │
            ┌─────────▼─────────┐
            │  virtual_machine  │
            │  (generic VM)     │
            └───────────────────┘
```

## Usage

### Simple Execution

```elixir
alias CodingAdventures.StarlarkInterpreter

result = StarlarkInterpreter.interpret("x = 1 + 2\nprint(x)\n")
result.variables["x"]  #=> 3
result.output           #=> ["3"]
```

### With load()

```elixir
files = %{
  "//rules/math.star" => "def double(n):\n    return n * 2\n"
}

result = StarlarkInterpreter.interpret(
  "load(\"//rules/math.star\", \"double\")\nresult = double(21)\n",
  file_resolver: files
)
result.variables["result"]  #=> 42
```

### From a File

```elixir
result = StarlarkInterpreter.interpret_file("path/to/program.star")
```

### With a Function Resolver

```elixir
resolver = fn label ->
  path = String.replace(label, "//", "/repo/root/")
  File.read!(path)
end

result = StarlarkInterpreter.interpret(source, file_resolver: resolver)
```

## API

- `interpret(source, opts)` — Execute Starlark source code
- `interpret_file(path, opts)` — Execute a Starlark file from disk

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `:file_resolver` | `nil` | Map or function to resolve `load()` paths |
| `:max_recursion_depth` | `200` | Maximum call stack depth |
| `:load_cache` | `%{}` | Pre-populated cache of loaded files |

## How load() Works

When the compiler encounters `load("//rules/python.star", "py_library")`,
it emits:

```
LOAD_MODULE 0      # names[0] = "//rules/python.star"
DUP                # Keep module dict for multiple imports
IMPORT_FROM 1      # names[1] = "py_library"
STORE_NAME 1       # Store in current scope
POP                # Remove module dict
```

The interpreter overrides the default `LOAD_MODULE` stub with a handler that:

1. Resolves the file path using the configured file resolver
2. Recursively interprets the loaded file through the same pipeline
3. Caches the result (each file evaluated at most once)
4. Pushes the loaded file's variables as a dict onto the stack

## Dependencies

- `virtual_machine` — Generic stack-based bytecode interpreter
- `bytecode_compiler` — Generic AST-to-bytecode compiler framework
- `starlark_ast_to_bytecode_compiler` — Starlark-specific compiler
- `starlark_vm` — Starlark-specific VM with builtins
