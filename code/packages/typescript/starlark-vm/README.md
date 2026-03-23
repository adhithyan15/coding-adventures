# @coding-adventures/starlark-vm

A Starlark bytecode virtual machine built on the GenericVM framework. This package transforms the generic, language-agnostic VM into a complete Starlark interpreter by registering 46 opcode handlers and 23 built-in functions.

## Where It Fits in the Stack

```
  Source Code       "x = 1 + 2\nprint(x)\n"
       |
  Lexer             [@coding-adventures/starlark-lexer]
       |
  Parser            [@coding-adventures/starlark-parser]
       |
  Compiler          [starlark-ast-to-bytecode-compiler]
       |
  Bytecode          CodeObject { instructions, constants, names }
       |
  >>> VM <<<        [@coding-adventures/starlark-vm]  <-- THIS PACKAGE
       |
  Result            { variables: { x: 3 }, output: ["3"] }
```

## Quick Start

```typescript
import { createStarlarkVM, executeStarlark, Op } from "@coding-adventures/starlark-vm";

// Option 1: One-call execution with a CodeObject
const code = {
  instructions: [
    { opcode: Op.LOAD_CONST, operand: 0 },
    { opcode: Op.LOAD_CONST, operand: 1 },
    { opcode: Op.ADD },
    { opcode: Op.STORE_NAME, operand: 0 },
    { opcode: Op.HALT },
  ],
  constants: [1, 2],
  names: ["x"],
};

const result = executeStarlark(code);
console.log(result.variables["x"]);  // 3

// Option 2: Manual VM creation for more control
const vm = createStarlarkVM();
vm.execute(code);
console.log(vm.variables["x"]);  // 3
```

## Features

### 46 Opcodes (organized by category)

| Category        | Opcodes |
|-----------------|---------|
| Stack           | LOAD_CONST, POP, DUP, LOAD_NONE, LOAD_TRUE, LOAD_FALSE |
| Variables       | STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL, STORE_CLOSURE, LOAD_CLOSURE |
| Arithmetic      | ADD, SUB, MUL, DIV, FLOOR_DIV, MOD, POWER, NEGATE, BIT_AND, BIT_OR, BIT_XOR, BIT_NOT, LSHIFT, RSHIFT |
| Comparisons     | CMP_EQ, CMP_NE, CMP_LT, CMP_GT, CMP_LE, CMP_GE, CMP_IN, CMP_NOT_IN |
| Boolean         | NOT |
| Control Flow    | JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE, JUMP_IF_FALSE_OR_POP, JUMP_IF_TRUE_OR_POP |
| Functions       | MAKE_FUNCTION, CALL_FUNCTION, CALL_FUNCTION_KW, RETURN |
| Collections     | BUILD_LIST, BUILD_DICT, BUILD_TUPLE, LIST_APPEND, DICT_SET |
| Subscript/Attr  | LOAD_SUBSCRIPT, STORE_SUBSCRIPT, LOAD_ATTR, STORE_ATTR, LOAD_SLICE |
| Iteration       | GET_ITER, FOR_ITER, UNPACK_SEQUENCE |
| Module          | LOAD_MODULE, IMPORT_FROM |
| I/O             | PRINT |
| VM Control      | HALT |

### 23 Built-in Functions

type, bool, int, float, str, len, list, dict, tuple, range, sorted, reversed, enumerate, zip, min, max, abs, all, any, repr, hasattr, getattr, print

## Dependencies

- `@coding-adventures/virtual-machine` -- The GenericVM framework

## Development

```bash
npm install
npx vitest run --coverage
```
