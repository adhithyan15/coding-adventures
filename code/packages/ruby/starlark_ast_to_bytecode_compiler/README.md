# Starlark AST-to-Bytecode Compiler (Ruby)

Compiles Starlark Abstract Syntax Trees into bytecode instructions that the virtual machine can execute.

## Where It Fits

This package sits at the end of the Starlark compilation pipeline:

```
Source Code (String)
    |
    v
starlark_lexer: tokenize(source)
    |
    v
starlark_parser: parse(source)
    |
    v
AST (Parser::ASTNode tree)
    |
    v
THIS PACKAGE: Compiler.compile_starlark(source)
    |
    v
CodeObject (instructions + constants + names)
    |
    v
virtual_machine: execute(code_object)
```

## Usage

```ruby
require "coding_adventures_starlark_ast_to_bytecode_compiler"

# One-shot compilation from source
code = CodingAdventures::StarlarkAstToBytecodeCompiler::Compiler.compile_starlark("x = 1 + 2\n")
# => CodeObject with:
#    instructions: [LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0, HALT]
#    constants: [1, 2]
#    names: ["x"]

# Compile a pre-parsed AST
ast = CodingAdventures::StarlarkParser.parse("x = 42\n")
code = CodingAdventures::StarlarkAstToBytecodeCompiler::Compiler.compile_ast(ast)

# Disassemble for debugging
puts CodingAdventures::StarlarkAstToBytecodeCompiler::Compiler.disassemble(code)
```

## Opcodes

The compiler defines 46 opcodes covering all Starlark language features:

| Category | Opcodes |
|----------|---------|
| Stack | LOAD_CONST, POP, DUP, LOAD_NONE, LOAD_TRUE, LOAD_FALSE |
| Variables | STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL, STORE_CLOSURE, LOAD_CLOSURE |
| Arithmetic | ADD, SUB, MUL, DIV, FLOOR_DIV, MOD, POWER, NEGATE, BIT_AND, BIT_OR, BIT_XOR, BIT_NOT, LSHIFT, RSHIFT |
| Comparison | CMP_EQ, CMP_LT, CMP_GT, CMP_NE, CMP_LE, CMP_GE, CMP_IN, CMP_NOT_IN, NOT |
| Control Flow | JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE, JUMP_IF_FALSE_OR_POP, JUMP_IF_TRUE_OR_POP, BREAK, CONTINUE |
| Functions | MAKE_FUNCTION, CALL_FUNCTION, CALL_FUNCTION_KW, RETURN_VALUE |
| Collections | BUILD_LIST, BUILD_DICT, BUILD_TUPLE, LIST_APPEND, DICT_SET |
| Subscript | LOAD_SUBSCRIPT, STORE_SUBSCRIPT, LOAD_ATTR, STORE_ATTR, LOAD_SLICE |
| Iteration | GET_ITER, FOR_ITER, UNPACK_SEQUENCE |
| Module | LOAD_MODULE, IMPORT_FROM |
| I/O | PRINT_VALUE |
| VM Control | HALT |

## Dependencies

- `coding_adventures_bytecode_compiler` -- GenericCompiler framework
- `coding_adventures_virtual_machine` -- CodeObject, Instruction types
- `coding_adventures_starlark_parser` -- Parses Starlark source to AST
- `coding_adventures_starlark_lexer` -- Tokenizes Starlark source
- `coding_adventures_parser` -- ASTNode type
- `coding_adventures_lexer` -- Token type
- `coding_adventures_grammar_tools` -- Grammar utilities

## Running Tests

```bash
bundle install
bundle exec rake test
```
