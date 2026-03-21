# CodingAdventures.Brainfuck

A Brainfuck interpreter built on the GenericVM from the `virtual_machine` package.

## What is this?

This package proves that the GenericVM is truly generic by running a radically
different language on it. Where a language like Starlark uses the stack, variables,
and call frames, Brainfuck ignores all of that and uses only the `extra` map for
its tape and data pointer.

Same execution engine, completely different semantics.

## Quick Start

```elixir
alias CodingAdventures.Brainfuck

# Hello World
result = Brainfuck.execute_brainfuck(
  "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++."
)
result.output  #=> "Hello World!\n"

# Addition: 2 + 5 = 7
result = Brainfuck.execute_brainfuck("++>+++++[<+>-]")
Enum.at(result.tape, 0)  #=> 7

# Input echo
result = Brainfuck.execute_brainfuck(",.", "A")
result.output  #=> "A"
```

## Architecture

```
Source Code ("++[>+<-]")
    |
    v
Translator  ──>  CodeObject (bytecode with bracket matching)
    |
    v
GenericVM   ──>  Fetch-Decode-Execute loop
    |                 |
    |            Handler Registry
    |            (9 opcode handlers)
    |                 |
    v                 v
BrainfuckResult  (output, tape, traces)
```

## Modules

| Module | Purpose |
|--------|---------|
| `Opcodes` | 9 opcode definitions (0x01-0x08 + 0xFF) |
| `Translator` | Source -> bytecode with bracket matching |
| `Handlers` | Opcode handler functions for the GenericVM |
| `VM` | Factory function and convenience executor |

## How it fits in the stack

This package depends on `coding_adventures_virtual_machine`, which provides
the GenericVM execution engine. The brainfuck package "teaches" the GenericVM
to speak Brainfuck by registering 9 opcode handlers and storing language-specific
state (tape, data pointer, input buffer) in the GenericVM's `extra` map.

## Running tests

```bash
mix deps.get
mix test --cover
```
