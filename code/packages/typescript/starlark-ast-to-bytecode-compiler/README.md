# @coding-adventures/starlark-ast-to-bytecode-compiler

Compiles Starlark ASTs into bytecode using the GenericCompiler framework. This is a TypeScript port of the Python reference implementation, faithfully reproducing all 46 opcodes and ~55 grammar rule handlers.

## Where It Fits in the Stack

```
Starlark source code
    | (starlark-lexer)
Token stream
    | (starlark-parser)
AST (ASTNode tree)
    | (THIS PACKAGE)
CodeObject (bytecode)
    | (virtual-machine)
Execution result
```

## Usage

### One-step compilation

```typescript
import { compileStarlark } from "@coding-adventures/starlark-ast-to-bytecode-compiler";

const code = compileStarlark("x = 1 + 2\n");
// code.instructions, code.constants, code.names are ready for the VM
```

### Step-by-step compilation

```typescript
import { createStarlarkCompiler, Op } from "@coding-adventures/starlark-ast-to-bytecode-compiler";
import { parseStarlark } from "@coding-adventures/starlark-parser";

const ast = parseStarlark("x = 1 + 2\n");
const compiler = createStarlarkCompiler();
const code = compiler.compile(ast, Op.HALT);
```

## Opcodes

All 46 Starlark opcodes are defined, organized by category:

| Category | Opcodes |
|----------|---------|
| Stack (0x0_) | LOAD_CONST, POP, DUP, LOAD_NONE, LOAD_TRUE, LOAD_FALSE |
| Variables (0x1_) | STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL, STORE_CLOSURE, LOAD_CLOSURE |
| Arithmetic (0x2_) | ADD, SUB, MUL, DIV, FLOOR_DIV, MOD, POWER, NEGATE, BIT_AND, BIT_OR, BIT_XOR, BIT_NOT, LSHIFT, RSHIFT |
| Comparison (0x3_) | CMP_EQ, CMP_NE, CMP_LT, CMP_GT, CMP_LE, CMP_GE, CMP_IN, CMP_NOT_IN, NOT |
| Control (0x4_) | JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE, JUMP_IF_FALSE_OR_POP, JUMP_IF_TRUE_OR_POP |
| Functions (0x5_) | MAKE_FUNCTION, CALL_FUNCTION, CALL_FUNCTION_KW, RETURN |
| Collections (0x6_) | BUILD_LIST, BUILD_DICT, BUILD_TUPLE, LIST_APPEND, DICT_SET |
| Subscript (0x7_) | LOAD_SUBSCRIPT, STORE_SUBSCRIPT, LOAD_ATTR, STORE_ATTR, LOAD_SLICE |
| Iteration (0x8_) | GET_ITER, FOR_ITER, UNPACK_SEQUENCE |
| Module (0x9_) | LOAD_MODULE, IMPORT_FROM |
| I/O (0xA_) | PRINT |
| VM (0xF_) | HALT |

## Grammar Rule Handlers

The compiler registers handlers for all Starlark grammar rules that require compilation logic:

- **Top-level**: file, simple_stmt
- **Statements**: assign_stmt, return_stmt, break_stmt, continue_stmt, pass_stmt, load_stmt
- **Compound**: if_stmt, for_stmt, def_stmt, suite
- **Expressions**: expression, expression_list, or_expr, and_expr, not_expr, comparison
- **Binary ops**: arith, term, shift, bitwise_or, bitwise_xor, bitwise_and
- **Unary/power**: factor, power
- **Primary**: primary, atom
- **Collections**: list_expr, dict_expr, paren_expr
- **Lambda**: lambda_expr

Pass-through rules (statement, compound_stmt, small_stmt) are handled automatically by the GenericCompiler framework.

## License

MIT
