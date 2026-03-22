# coding_adventures_starlark_interpreter

A full Starlark interpreter that chains the entire pipeline (lexer, parser, compiler, VM) with `load()` statement support.

## What It Does

This package provides the top-level entry point for running Starlark programs. It adds `load()` support on top of the bare VM, enabling multi-file Starlark projects.

Key features:
- **Full pipeline**: Source code goes in, execution results come out
- **load() support**: Import symbols from other Starlark files
- **Caching**: Loaded modules are cached to avoid re-execution
- **File resolver**: Configurable resolution of module labels to file contents
- **File interpretation**: Read and execute `.star` files from disk

## How It Fits in the Stack

```
Source Code (String)
    |
    v
StarlarkInterpreter.interpret(source)
    |
    |-- StarlarkLexer.tokenize()
    |-- StarlarkParser.parse()
    |-- StarlarkAstToBytecodeCompiler.compile()
    |-- StarlarkVM.create_starlark_vm()
    |-- register_load_handler()  <-- THIS PACKAGE's addition
    |-- vm.execute()
    |
    v
StarlarkResult (variables, output, traces)
```

## Usage

### Simple Interpretation

```ruby
require "coding_adventures_starlark_interpreter"

result = CodingAdventures::StarlarkInterpreter.interpret("x = 1 + 2\n")
result.variables["x"]  # => 3
```

### With load() Support

```ruby
files = {
  "//math.star" => "def double(n):\n    return n * 2\n"
}
resolver = ->(label) { files[label] }

source = <<~STARLARK
  load("//math.star", "double")
  result = double(21)
STARLARK

result = CodingAdventures::StarlarkInterpreter.interpret(source, file_resolver: resolver)
result.variables["result"]  # => 42
```

### Interpreting Files

```ruby
result = CodingAdventures::StarlarkInterpreter.interpret_file("path/to/build.star")
```

### Reusable Interpreter Instance

```ruby
interp = CodingAdventures::StarlarkInterpreter::Interpreter.new(
  file_resolver: resolver,
  max_recursion_depth: 100
)
result1 = interp.interpret(source1)
result2 = interp.interpret(source2)  # shares load cache with result1
```

## Dependencies

- `coding_adventures_starlark_vm` -- VM with opcode handlers and builtins
- `coding_adventures_starlark_ast_to_bytecode_compiler` -- Bytecode compiler
- `coding_adventures_bytecode_compiler` -- Generic compiler framework
- `coding_adventures_starlark_parser` -- Starlark parser
- `coding_adventures_starlark_lexer` -- Starlark lexer
- `coding_adventures_parser` -- Generic parser framework
- `coding_adventures_lexer` -- Generic lexer framework
- `coding_adventures_grammar_tools` -- Grammar utilities
- `coding_adventures_virtual_machine` -- GenericVM execution engine
