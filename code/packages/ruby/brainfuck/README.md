# Brainfuck Interpreter

A Brainfuck interpreter built on the **pluggable GenericVM** framework from `coding_adventures_virtual_machine`.

## Why Brainfuck?

This package proves that the GenericVM architecture works for radically different languages. Starlark has 50+ opcodes, variables, functions, and collections. Brainfuck has 8 opcodes and a tape. Both run on the same GenericVM chassis — different engines, same car.

## Package Structure

| File             | Purpose                                            |
|------------------|----------------------------------------------------|
| `opcodes.rb`     | Opcode constants and character-to-opcode map        |
| `lexer.rb`       | Grammar-driven tokenizer (`tokenize`)               |
| `parser.rb`      | Grammar-driven parser (`parse`), returns AST        |
| `translator.rb`  | Source code to bytecode translation                 |
| `handlers.rb`    | Opcode handler lambdas registered with GenericVM    |
| `vm.rb`          | BrainfuckResult, factory, convenience executor      |
| `version.rb`     | Gem version constant                                |

## Usage

### Lexer

```ruby
require "coding_adventures_brainfuck"

tokens = CodingAdventures::Brainfuck.tokenize("++[>+<-].")
tokens.each do |tok|
  puts "#{tok.type} #{tok.value.inspect} at #{tok.line}:#{tok.column}"
end
# COMMAND "+" at 1:1
# COMMAND "+" at 1:2
# LOOP_START "[" at 1:3
# ...
```

### Parser

```ruby
ast = CodingAdventures::Brainfuck.parse("++[>+<-].")
# Returns an ASTNode with type :program and children:
#   ASTNode(type: :instruction, children: [ASTNode(type: :command, value: "+")])
#   ASTNode(type: :loop, children: [...])
#   ASTNode(type: :instruction, children: [ASTNode(type: :command, value: ".")])
puts ast.type  # :program
```

### VM Execution

```ruby
require "coding_adventures_brainfuck"

# Hello World
result = CodingAdventures::Brainfuck.execute_brainfuck(
  "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]" \
  ">>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++."
)
puts result.output  # "Hello World!\n"

# Addition: 2 + 5 = 7
result = CodingAdventures::Brainfuck.execute_brainfuck("++>+++++[<+>-]")
puts result.tape[0]  # 7

# Cat program (echo input)
result = CodingAdventures::Brainfuck.execute_brainfuck(",[.,]", input_data: "Hi!")
puts result.output  # "Hi!"
```

## Architecture

```
Source code ("++[>+<-]")
       │
       ▼
   Lexer       ──→  Token stream (type, value, line, column)
       │
       ▼
   Parser      ──→  AST (program / loop / instruction / command nodes)
       │
       ▼
   Translator  ──→  CodeObject (instructions, no constants/names)
       │
       ▼
   GenericVM   ──→  BrainfuckResult (output, tape, traces)
   (with BF
    handlers)
```

- **Lexer** (`lexer.rb`): Grammar-driven tokenizer. Skips comment characters silently.
- **Parser** (`parser.rb`): Grammar-driven parser. Returns an AST; raises `ParseError` with line/column on bracket mismatch.
- **Translator** (`translator.rb`): Converts BF source to bytecode. Each character maps to one instruction. Bracket matching resolves jump targets.
- **Handlers** (`handlers.rb`): 9 handler lambdas registered with GenericVM via `register_opcode()`.
- **VM Factory** (`vm.rb`): `create_brainfuck_vm()` creates a GenericVM with BF handlers and tape state.

## The 8 Commands

| Command | Opcode | Description |
|---------|--------|-------------|
| `>` | RIGHT | Move data pointer right |
| `<` | LEFT | Move data pointer left |
| `+` | INC | Increment cell (wraps 255→0) |
| `-` | DEC | Decrement cell (wraps 0→255) |
| `.` | OUTPUT | Print cell as ASCII |
| `,` | INPUT | Read byte into cell |
| `[` | LOOP_START | Jump past `]` if cell is 0 |
| `]` | LOOP_END | Jump back to `[` if cell is not 0 |

Everything else is a comment.

## How It Fits in the Stack

This package spans Layers 2–5, covering Lexer, Parser, and VM for the Brainfuck language, sitting alongside `starlark-vm` as a second language plugin for the GenericVM framework.

```
Layer 5: Language VMs    [starlark-vm] [brainfuck]  ← YOU ARE HERE
Layer 5: Generic VM      [virtual-machine (GenericVM)]
Layer 4: Compiler        [bytecode-compiler (GenericCompiler)]
Layer 3: Parser          [parser]        ← also implemented here
Layer 2: Lexer           [lexer]         ← also implemented here
Layer 1: Grammar Tools   [grammar-tools]
```
