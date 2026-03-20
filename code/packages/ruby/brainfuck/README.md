# Brainfuck Interpreter

A Brainfuck interpreter built on the **pluggable GenericVM** framework from `coding_adventures_virtual_machine`.

## Why Brainfuck?

This package proves that the GenericVM architecture works for radically different languages. Starlark has 50+ opcodes, variables, functions, and collections. Brainfuck has 8 opcodes and a tape. Both run on the same GenericVM chassis — different engines, same car.

## Usage

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
   Translator  ──→  CodeObject (instructions, no constants/names)
       │
       ▼
   GenericVM   ──→  BrainfuckResult (output, tape, traces)
   (with BF
    handlers)
```

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

This is a **Layer 5** package (Virtual Machine layer), sitting alongside `starlark-vm` as a second language plugin for the GenericVM framework.

```
Layer 5: Language VMs    [starlark-vm] [brainfuck]  ← YOU ARE HERE
Layer 5: Generic VM      [virtual-machine (GenericVM)]
Layer 4: Compiler        [bytecode-compiler (GenericCompiler)]
Layer 3: Parser          [parser]
Layer 2: Lexer           [lexer]
Layer 1: Grammar Tools   [grammar-tools]
```
