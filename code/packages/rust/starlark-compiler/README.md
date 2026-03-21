# starlark-compiler

Compiles Starlark ASTs into bytecode that the Starlark VM can execute.

## Architecture

The full pipeline from source code to execution:

```text
Starlark source code
    | (starlark_lexer)
Token stream
    | (starlark_parser)
AST (ASTNode tree)
    | (THIS CRATE)
CodeObject (bytecode)
    | (starlark_vm)
Execution result
```

## Opcode Organization

Opcodes are grouped by category using the high nibble:

- `0x0_` = Stack operations (push, pop, dup, load constants)
- `0x1_` = Variable operations (store/load by name or slot)
- `0x2_` = Arithmetic (add, sub, mul, div, bitwise)
- `0x3_` = Comparison and boolean (==, !=, <, >, in, not)
- `0x4_` = Control flow (jump, branch)
- `0x5_` = Functions (make, call, return)
- `0x6_` = Collections (build list, dict, tuple)
- `0x7_` = Subscript and attribute (indexing, slicing, dot access)
- `0x8_` = Iteration (get_iter, for_iter, unpack)
- `0x9_` = Module (load statement)
- `0xA_` = I/O (print)
- `0xF_` = VM control (halt)

## Dependencies

- `starlark-lexer` -- tokenizes Starlark source code
- `starlark-parser` -- parses tokens into an AST
- `bytecode-compiler` -- generic compiler framework
- `virtual-machine` -- CodeObject and Instruction types
