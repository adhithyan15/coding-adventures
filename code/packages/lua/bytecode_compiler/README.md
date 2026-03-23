# bytecode-compiler

Compiler translating ASTs to stack-based bytecode instructions.

## Layer 4

This package is part of Layer 4 of the coding-adventures computing stack,
sitting between the parser (Layer 3) and the virtual machine (Layer 5).

```
Source code → [Lexer] → Tokens → [Parser] → AST → [Compiler] → Bytecode → [VM]
```

## Three Compilers

This package provides three compilers, each illustrating a different design:

### BytecodeCompiler

A simple, hardcoded compiler that translates our parser's AST (NumberLiteral,
StringLiteral, BinaryOp, Assignment, ExpressionStmt) directly to our VM's
instruction set. Tightly coupled to one language.

```lua
local bc = require("coding_adventures.bytecode_compiler")

local compiler = bc.BytecodeCompiler.new()
local program = bc.Program({
    bc.Assignment(
        bc.Name("x"),
        bc.BinaryOp(bc.NumberLiteral(1), "+", bc.NumberLiteral(2))
    )
})
local code = compiler:compile(program)
-- code.instructions = { LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0, HALT }
-- code.constants = { 1, 2 }
-- code.names = { "x" }
```

### JVMCompiler

A compiler targeting JVM-style bytecode with specialized encodings for small
integers (ICONST_n), byte-range values (BIPUSH), and indexed local variables
(ILOAD_n / ISTORE_n).

```lua
local compiler = bc.JVMCompiler.new()
local program = bc.Program({
    bc.Assignment(bc.Name("x"), bc.NumberLiteral(3))
})
local code = compiler:compile(program)
-- code.bytecode = { ICONST_3, ISTORE_0, RETURN }
```

### GenericCompiler

A pluggable framework where language-specific behavior is provided by
registering handler functions. Includes scope management, jump patching,
and nested code object compilation.

```lua
local compiler = bc.GenericCompiler.new()

compiler:register_rule("number", function(c, node)
    local token = node.children[1]
    local value = tonumber(token.value)
    c:emit(bc.OpLoadConst, c:add_constant(value))
end)

compiler:register_rule("addition", function(c, node)
    c:compile_node(node.children[1])
    c:compile_node(node.children[3])
    c:emit(bc.OpAdd)
end)

local ast = bc.ASTNode("addition", {
    bc.ASTNode("number", { bc.TokenNode("NUMBER", "1") }),
    bc.TokenNode("PLUS", "+"),
    bc.ASTNode("number", { bc.TokenNode("NUMBER", "2") }),
})

local code = compiler:compile(ast)
```

## Dependencies

- parser (Layer 3) — provides AST node types
- virtual-machine (Layer 5) — provides Instruction, CodeObject, OpCode constants

## API Reference

### OpCode Constants

All VM opcodes are exported as module-level constants:

| Constant | Value | Description |
|----------|-------|-------------|
| `OpLoadConst` | 0x01 | Push constant onto stack |
| `OpPop` | 0x02 | Discard top of stack |
| `OpStoreName` | 0x10 | Store value in named variable |
| `OpLoadName` | 0x11 | Load value from named variable |
| `OpAdd` | 0x20 | Add top two values |
| `OpSub` | 0x21 | Subtract |
| `OpMul` | 0x22 | Multiply |
| `OpDiv` | 0x23 | Divide |
| `OpJump` | 0x40 | Unconditional jump |
| `OpJumpIfFalse` | 0x41 | Jump if top is falsy |
| `OpHalt` | 0xFF | Stop execution |

### CompilerScope

Tracks local variable slots within a scope.

- `CompilerScope.new(parent)` — create scope linked to parent
- `scope:add_local(name)` — register local, returns 0-based slot
- `scope:get_local(name)` — returns slot, found_bool
- `scope:num_locals()` — count of registered locals

## Development

```bash
# Run tests
cd tests && busted test_bytecode_compiler.lua

# Or use the build system
bash BUILD
```
