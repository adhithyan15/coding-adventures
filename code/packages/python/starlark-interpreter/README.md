# Starlark Interpreter

Full Starlark interpreter that chains lexer → parser → compiler → VM into a single pipeline with `load()` support for BUILD files.

## Where It Fits

```
Source Code
    ↓
starlark_lexer.tokenize()        -- tokenization
    ↓
starlark_parser.parse()          -- parsing
    ↓
starlark_ast_to_bytecode_compiler.compile_starlark()  -- compilation
    ↓
starlark_vm.create_starlark_vm() -- execution
    ↓
starlark_interpreter.interpret() -- [THIS PACKAGE] full pipeline + load()
```

## Quick Start

```python
from starlark_interpreter import interpret

# Simple execution
result = interpret("x = 1 + 2\nprint(x)\n")
assert result.variables["x"] == 3
assert result.output == ["3"]
```

## load() Support

The interpreter adds `load()` as a built-in function, enabling BUILD-file patterns:

```python
from starlark_interpreter import interpret

# Define rule files
files = {
    "//rules/python.star": '''
def py_library(name, deps):
    return {"name": name, "deps": deps}
''',
}

# Execute a BUILD-style file
result = interpret(
    'load("//rules/python.star", "py_library")\n'
    'target = py_library(name="mylib", deps=["//dep1"])\n',
    file_resolver=files,
)
assert result.variables["target"]["name"] == "mylib"
```

## API

- `interpret(source, file_resolver=None)` — Execute Starlark source code
- `interpret_file(path, file_resolver=None)` — Execute a Starlark file
- `StarlarkInterpreter` — Class with shared cache across multiple `interpret()` calls

## Dependencies

- `coding-adventures-starlark-lexer` — tokenization
- `coding-adventures-starlark-parser` — parsing
- `coding-adventures-starlark-ast-to-bytecode-compiler` — AST to bytecode
- `coding-adventures-starlark-vm` — bytecode execution
- `coding-adventures-virtual-machine` — GenericVM framework

## Install

```bash
pip install coding-adventures-starlark-interpreter
```
