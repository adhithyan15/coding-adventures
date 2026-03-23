# coding_adventures_starlark_vm

A complete Starlark virtual machine built on top of the GenericVM framework.

## What It Does

This package implements the execution layer of the Starlark language. It takes compiled bytecode (from `starlark_ast_to_bytecode_compiler`) and runs it on a stack-based virtual machine (from `virtual_machine`).

The VM registers:
- **46 opcode handlers** covering stack operations, variable access, arithmetic, comparisons, control flow, functions, collections, iteration, module loading, and I/O
- **23 builtin functions** including `print`, `len`, `type`, `range`, `sorted`, `reversed`, `enumerate`, `zip`, `min`, `max`, `abs`, `all`, `any`, and more

## How It Fits in the Stack

```
Source Code (String)
    |  starlark_lexer
    v
Tokens
    |  starlark_parser
    v
AST
    |  starlark_ast_to_bytecode_compiler
    v
Bytecode (CodeObject)
    |  starlark_vm  <-- THIS PACKAGE
    v
Execution Result (variables, output, traces)
```

## Usage

### Quick Execution

```ruby
require "coding_adventures_starlark_vm"

result = CodingAdventures::StarlarkVM.execute_starlark("x = 1 + 2\n")
result.variables["x"]  # => 3
result.output           # => []
```

### With Print

```ruby
result = CodingAdventures::StarlarkVM.execute_starlark("print(\"hello world\")\n")
result.output  # => ["hello world"]
```

### Manual VM Setup

```ruby
require "coding_adventures_starlark_vm"

# Create a configured VM
vm = CodingAdventures::StarlarkVM.create_starlark_vm

# Compile separately
code = CodingAdventures::StarlarkAstToBytecodeCompiler::Compiler.compile_starlark("x = 42\n")

# Execute
traces = vm.execute(code)
vm.variables["x"]  # => 42
```

## Dependencies

- `coding_adventures_virtual_machine` -- GenericVM execution engine
- `coding_adventures_starlark_ast_to_bytecode_compiler` -- Bytecode compiler
- `coding_adventures_bytecode_compiler` -- Generic compiler framework
- `coding_adventures_starlark_parser` -- Starlark parser
- `coding_adventures_starlark_lexer` -- Starlark lexer
- `coding_adventures_parser` -- Generic parser framework
- `coding_adventures_lexer` -- Generic lexer framework
- `coding_adventures_grammar_tools` -- Grammar utilities
