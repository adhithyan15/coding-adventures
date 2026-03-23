# starlark-ast-to-bytecode-compiler

Compiles Starlark ASTs into bytecode using the `GenericCompiler` framework.

## Architecture

This crate sits in the middle of the Starlark compilation pipeline:

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

## How It Works

The compiler registers **handler functions** for each Starlark grammar rule
with the `GenericCompiler` from the `bytecode-compiler` crate. When the
compiler walks an AST, it dispatches each node to the appropriate handler.

Each handler inspects the node's children, recursively compiles sub-expressions,
and emits bytecode instructions using the opcodes defined in `starlark-compiler`.

## Grammar Rules Handled

- **Top-level**: `file`, `simple_stmt`, `suite`
- **Statements**: `assign_stmt`, `return_stmt`, `break_stmt`, `continue_stmt`,
  `pass_stmt`, `load_stmt`, `if_stmt`, `for_stmt`, `def_stmt`
- **Expressions**: `expression`, `expression_list`, `or_expr`, `and_expr`,
  `not_expr`, `comparison`
- **Binary**: `arith`, `term`, `shift`, `bitwise_or`, `bitwise_xor`, `bitwise_and`
- **Unary/power**: `factor`, `power`
- **Primary**: `primary`, `atom` (with suffix handling for calls, subscripts, attributes)
- **Collections**: `list_expr`, `dict_expr`, `paren_expr` (including comprehensions)
- **Lambda**: `lambda_expr`

## Usage

```rust
use starlark_ast_to_bytecode_compiler::create_starlark_compiler;
use starlark_compiler::Op;

let mut compiler = create_starlark_compiler();
let code = compiler.compile(&ast, Some(Op::Halt as u8));
```

## Dependencies

- `bytecode-compiler` -- GenericCompiler framework (ASTNode, CompilerScope, etc.)
- `starlark-compiler` -- Op enum and operator-to-opcode mapping tables
- `virtual-machine` -- Value, CodeObject, Instruction types
