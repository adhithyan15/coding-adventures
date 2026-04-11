# CodingAdventures.Brainfuck

A Brainfuck interpreter built on the GenericVM from the `virtual_machine` package.

## What is this?

This package proves that the GenericVM is truly generic by running a radically
different language on it. Where a language like Starlark uses the stack, variables,
and call frames, Brainfuck ignores all of that and uses only the `extra` map for
its tape and data pointer.

Same execution engine, completely different semantics.

## Modules

| Module | Purpose |
|--------|---------|
| `Opcodes` | 9 opcode definitions (0x01-0x08 + 0xFF) |
| `Lexer` | Grammar-driven tokenizer (`tokenize/1`) |
| `Parser` | Grammar-driven parser (`parse/1`), returns AST |
| `Translator` | Source → bytecode with bracket matching |
| `Handlers` | Opcode handler functions for the GenericVM |
| `VM` | Factory function and convenience executor |

## Quick Start

### Lexer

```elixir
alias CodingAdventures.Brainfuck

{:ok, tokens} = Brainfuck.Lexer.tokenize("++[>+<-].")
Enum.each(tokens, fn tok ->
  IO.puts("#{tok.type} #{tok.value} at #{tok.line}:#{tok.column}")
end)
# COMMAND + at 1:1
# COMMAND + at 1:2
# LOOP_START [ at 1:3
# ...
```

### Parser

```elixir
{:ok, ast} = Brainfuck.Parser.parse("++[>+<-].")
# Returns %{type: :program, children: [
#   %{type: :instruction, children: [%{type: :command, value: "+"}]},
#   %{type: :instruction, children: [%{type: :command, value: "+"}]},
#   %{type: :loop, children: [...]},
#   %{type: :instruction, children: [%{type: :command, value: "."}]}
# ]}
IO.inspect(ast.type)  #=> :program
```

### VM Execution

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
Lexer       ──>  Token stream (type, value, line, column)
    |
    v
Parser      ──>  AST (program / loop / instruction / command nodes)
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

## How it fits in the stack

This package spans Layers 2–5, covering Lexer, Parser, and VM for Brainfuck. It depends on `coding_adventures_virtual_machine` for the GenericVM execution engine, and on `coding_adventures_grammar_tools`, `coding_adventures_lexer`, and `coding_adventures_parser` for the tokenization and parsing pipeline. The brainfuck package "teaches" the GenericVM to speak Brainfuck by registering 9 opcode handlers and storing language-specific state (tape, data pointer, input buffer) in the GenericVM's `extra` map.

## Running tests

```bash
mix deps.get
mix test --cover
```
