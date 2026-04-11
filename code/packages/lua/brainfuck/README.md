# coding-adventures-brainfuck (Lua)

A Brainfuck interpreter and optimising compiler for the Lua implementation
of the coding-adventures computing stack.

## What Is Brainfuck?

Brainfuck (Urban Müller, 1993) is a Turing-complete language with exactly
8 commands. Despite its absurd minimalism, it reveals the essence of how a
CPU works: a memory tape, a pointer, and conditional jumps.

| Command | Meaning                                        |
|---------|------------------------------------------------|
| `>`     | Move data pointer right                        |
| `<`     | Move data pointer left                         |
| `+`     | Increment byte at current cell (wraps 255→0)   |
| `-`     | Decrement byte at current cell (wraps 0→255)   |
| `.`     | Output current cell as ASCII character         |
| `,`     | Read one byte of input into current cell       |
| `[`     | If current cell is 0, jump past matching `]`   |
| `]`     | If current cell is nonzero, jump back to `[`   |

Any other character is a comment.

## Where It Fits in the Stack

```
grammar-tools    ← grammar file parsing
lexer            ← generic tokenizer framework
parser           ← generic parser framework
brainfuck        ← this package: Lexer, Parser, and Interpreter
virtual-machine  ← stack-based bytecode VM (opcodes reused here)
```

This package now spans the Lexer (Layer 2), Parser (Layer 3), and VM/Interpreter (Layer 5) layers of the coding-adventures stack.

## Package Structure

| File         | Purpose                                          |
|--------------|--------------------------------------------------|
| `init.lua`   | Interpreter: `validate`, `compile_to_opcodes`, `run_opcodes`, `interpret` |
| `lexer.lua`  | Grammar-driven tokenizer (`tokenize`)             |
| `parser.lua` | Grammar-driven parser (`parse`), returns AST      |

## Usage

### Lexer

```lua
local lexer = require("coding_adventures.brainfuck.lexer")

local tokens, err = lexer.tokenize("++[>+<-].")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.column)
end
-- COMMAND  +  1  1
-- COMMAND  +  1  2
-- LOOP_START  [  1  3
-- ...
```

### Parser

```lua
local parser = require("coding_adventures.brainfuck.parser")

local ast, err = parser.parse("++[>+<-].")
-- ast.type == "program"
-- ast.children == {
--   { type = "instruction", children = {{ type = "command", value = "+" }} },
--   { type = "instruction", children = {{ type = "command", value = "+" }} },
--   { type = "loop",        children = { ... } },
--   { type = "instruction", children = {{ type = "command", value = "." }} },
-- }
print(ast.type)  -- "program"
```

### Interpreter

```lua
local bf = require("coding_adventures.brainfuck")

-- High-level: interpret in one call
local out, err = bf.interpret("+++++++++[>++++++++<-]>.", "")
-- out == "H"  (9 * 8 = 72 = ASCII 'H')

-- Two-phase: compile once, run multiple times
local opcodes, err = bf.compile_to_opcodes(",[.,]")
if err then error(err) end
local result = bf.run_opcodes(opcodes, "hello")
-- result == "hello"

-- Validation
local ok, msg = bf.validate("[[]")
-- ok == false, msg == "1 unclosed '[' bracket(s)"
```

## API

### `lexer.tokenize(source)` → tokens, err

Tokenize Brainfuck source. Returns a list of token tables (`{type, value, line, column}`). Comment characters are skipped. Returns `nil, message` on error.

### `parser.parse(source)` → ast, err

Parse Brainfuck source into an AST. Returns a root node table (`{type = "program", children = {...}}`). Returns `nil, message` with line/column info on unmatched bracket.

### `bf.validate(program)` → ok, err

Check bracket balance. Returns `true, nil` if valid, or `false, message` if not.

### `bf.compile_to_opcodes(program)` → opcodes, err

Compile source to an opcode list with pre-computed jump targets. Runs
validate() internally. Returns `nil, message` on error.

### `bf.run_opcodes(opcodes, input_str)` → output_string

Execute compiled opcodes with the given input. Returns accumulated output.

### `bf.interpret(program, input_str)` → output_string, err

Validate + compile + run in one call. Returns `nil, message` on error.

## License

MIT
