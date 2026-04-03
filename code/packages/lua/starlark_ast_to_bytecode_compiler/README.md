# coding-adventures-starlark-ast-to-bytecode-compiler

A Pure Lua compiler that translates Starlark ASTs (from `starlark_parser`) to stack-based bytecode for the Starlark VM.

## Usage

```lua
local C = require("coding_adventures.starlark_ast_to_bytecode_compiler")

-- Build an AST node (normally produced by starlark_parser)
local tree = C.ast_node("file", {
    C.ast_node("statement", {
        C.ast_node("simple_stmt", {
            C.ast_node("assign_stmt", {
                C.ast_node("identifier", { C.token_node("NAME", "x") }),
                C.token_node("OP", "="),
                C.ast_node("arith", {
                    C.ast_node("atom", { C.token_node("INT", "1") }),
                    C.token_node("OP", "+"),
                    C.ast_node("atom", { C.token_node("INT", "2") }),
                }),
            })
        })
    })
})

-- Compile to bytecode
local co = C.compile_ast(tree)
-- co.instructions = [{opcode=0x01, operand=0}, {opcode=0x01, operand=1}, {opcode=0x20}, {opcode=0x10, operand=0}, {opcode=0xFF}]
-- co.constants    = {1, 2}
-- co.names        = {"x"}
```

## Supported Constructs

| Construct           | AST Rule             | Opcodes emitted          |
|---------------------|----------------------|--------------------------|
| Integer literal     | atom (INT token)     | LOAD_CONST               |
| String literal      | atom (STRING token)  | LOAD_CONST               |
| True / False / None | atom (NAME token)    | LOAD_TRUE / LOAD_FALSE / LOAD_NONE |
| Variable reference  | identifier           | LOAD_NAME                |
| Assignment          | assign_stmt          | LOAD_CONST + STORE_NAME  |
| Addition            | arith (+)            | LOAD... + ADD            |
| Comparison          | comparison (==, <)   | LOAD... + CMP_EQ / CMP_LT |
| Boolean and/or      | and_expr / or_expr   | short-circuit jumps      |
| not                 | not_expr             | LOGICAL_NOT              |
| Unary -/~           | factor               | NEGATE / BIT_NOT         |
| if/elif/else        | if_stmt              | JUMP_IF_FALSE + patching |
| for loop            | for_stmt             | GET_ITER + FOR_ITER      |
| Function def        | def_stmt             | MAKE_FUNCTION            |
| Function call       | call                 | CALL_FUNCTION            |
| List literal        | list_expr            | BUILD_LIST N             |
| Dict literal        | dict_expr            | BUILD_DICT N             |
| Tuple               | tuple_expr           | BUILD_TUPLE N            |
| pass                | pass_stmt            | (nothing)                |
| return              | return_stmt          | LOAD_CONST + RETURN      |
| Augmented assign    | augmented_assign_stmt| LOAD + op + STORE        |

## Installation

```sh
luarocks make --local coding-adventures-starlark-ast-to-bytecode-compiler-0.1.0-1.rockspec
```

## Testing

```sh
cd tests && busted . --verbose --pattern=test_
```
