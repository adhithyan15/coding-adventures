# BF01 — Brainfuck Parser

## Overview

Brainfuck has one syntactic construct beyond simple commands: the loop, written `[body]`. Everything else is a flat sequence. This makes the grammar exactly four rules — the smallest meaningful grammar in the toolchain — and a perfect first test of the grammar-driven parser infrastructure.

The parser consumes the token stream produced by `Brainfuck.Lexer` (spec `BF00`) and produces a standard `ASTNode.t()` tree. The key benefit over the current `Translator` approach is **structured error reporting**: an unmatched `[` or `]` becomes a parse error with a precise source location, not a silent runtime failure or a confusing crash.

## The Grammar

Brainfuck's structure can be described in four sentences:

> A **program** is a sequence of zero or more instructions.
> An **instruction** is either a simple command or a loop.
> A **loop** starts with `[`, contains zero or more instructions, and ends with `]`.
> A **command** is any of the six non-bracket operators: `>`, `<`, `+`, `-`, `.`, `,`

Formally, in the same EBNF notation used by the grammar infrastructure:

```
program     = { instruction } ;
instruction = command | loop ;
loop        = LOOP_START { instruction } LOOP_END ;
command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
```

That is the entire grammar. There are no expressions, no variables, no types, no declarations.

## The Grammar File: `brainfuck.grammar`

```
# Parser grammar for Brainfuck
# @version 1
#
# Brainfuck is a minimal language with one compound construct: the loop.
# Everything else is a flat sequence of single-character commands.
#
# Grammar entry point: program (the first rule).
#
# Token names (UPPERCASE) match the types defined in brainfuck.tokens.
# Comment tokens do not appear here — they are consumed by the lexer's
# skip: section and never reach the parser.

# A program is a sequence of zero or more instructions. The outer { }
# means "zero or more repetitions". An empty file is a valid program.
program = { instruction } ;

# An instruction is either a loop (which nests) or a command (which doesn't).
# Loop comes first because its leading token LOOP_START is unambiguous.
instruction = loop | command ;

# A loop: open bracket, any number of nested instructions, close bracket.
# The inner { instruction } means the loop body can be empty ([]), which
# is legal Brainfuck (an infinite loop if the cell is nonzero, or a no-op
# if it is zero — commonly used as a "clear cell" idiom: [-]).
loop = LOOP_START { instruction } LOOP_END ;

# A command is any of the six non-bracket operators. The | means "or".
command = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
```

### Why does `instruction = loop | command` put `loop` first?

The PEG parser tries alternatives left-to-right and commits to the first that succeeds. If `command` came first, the parser would try to match `[` as a command, fail (because `LOOP_START` is not listed in `command`), and then try `loop`. This would work but is slower. Putting `loop` first means the parser checks the leading `LOOP_START` token once and commits immediately.

## AST Shape

The parser produces a tree of `ASTNode.t()` structs. Here is the shape for the program `++[>+<-]`:

```
program (line 1:1 – 1:8)
├── instruction (line 1:1 – 1:1)
│   └── command (line 1:1 – 1:1)
│       └── Token{type: "INC", value: "+", line: 1, col: 1}
├── instruction (line 1:2 – 1:2)
│   └── command (line 1:2 – 1:2)
│       └── Token{type: "INC", value: "+", line: 1, col: 2}
└── instruction (line 1:3 – 1:8)
    └── loop (line 1:3 – 1:8)
        ├── Token{type: "LOOP_START", value: "[", line: 1, col: 3}
        ├── instruction → command → Token{type: "RIGHT",  line: 1, col: 4}
        ├── instruction → command → Token{type: "INC",    line: 1, col: 5}
        ├── instruction → command → Token{type: "LEFT",   line: 1, col: 6}
        ├── instruction → command → Token{type: "DEC",    line: 1, col: 7}
        └── Token{type: "LOOP_END", value: "]", line: 1, col: 8}
```

Each `ASTNode` carries `start_line`, `start_column`, `end_line`, `end_column` — the span of the node in the source file. These are inherited from the underlying tokens and are essential for the debug sidecar (spec `05d`) and LSP diagnostics.

### Using `ASTNode` utilities

The existing `ASTNode` module provides traversal helpers that the compiler and LSP bridge will use:

```elixir
# Find all loop nodes in a program
loops = ASTNode.find_nodes(ast, "loop")

# Find all command nodes
commands = ASTNode.find_nodes(ast, "command")

# Walk the tree depth-first
ASTNode.walk_ast(ast, fn node ->
  case node.rule_name do
    "loop"    -> IO.puts("loop at line #{node.start_line}")
    "command" -> IO.puts("#{ASTNode.token(node).type} at line #{node.start_line}")
    _         -> :ok
  end
end)
```

## Error Reporting

The most important benefit of having a parser is **unmatched bracket errors**. The current `Translator` detects unmatched brackets but does so during compilation, which means the error has no direct source location. The parser error has a precise location.

### Unmatched `[`

```brainfuck
++[>+<-
       ^ EOF reached here
```

The parser starts matching a `loop` rule when it sees `[` at column 3. It then tries `{ instruction }` but reaches EOF without seeing `LOOP_END`. The error:

```
Parse error at line 1, column 8: expected ']' to close loop opened at line 1, column 3
```

### Unmatched `]`

```brainfuck
++>+<-]
      ^ unexpected here
```

The parser successfully matches `++>+<-` as a sequence of commands, then encounters `]` which is a `LOOP_END` token. At the `program` level, `program = { instruction }` tries to match `instruction` next — but `LOOP_END` is not valid as the start of an `instruction` (neither `loop` nor `command` begins with `]`). The error:

```
Parse error at line 1, column 7: unexpected ']' — no matching '[' is open
```

### Nested unmatched brackets

```brainfuck
[>[<]
```

The outer `[` opens a loop. The inner `[<]` is a valid nested loop. But the outer loop's `]` is never found. Error:

```
Parse error at line 1, column 6: expected ']' to close loop opened at line 1, column 1
```

These errors surface as `Diagnostic` structs in the LSP bridge, appearing as red squiggles in VS Code directly on the mismatched bracket — not at runtime, not as a compiler error, but as you type.

## Public API

```elixir
defmodule Brainfuck.Parser do
  @moduledoc """
  Parses a Brainfuck token stream into an AST.

  Accepts either a source string (which it tokenises internally) or a
  pre-tokenised list (for use with the LSP bridge, which already has tokens
  from the lexer step).
  """

  alias CodingAdventures.Parser.ASTNode

  @spec parse(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source)

  @spec parse_tokens([Token.t()]) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse_tokens(tokens)
end
```

### Example

```elixir
{:ok, ast} = Brainfuck.Parser.parse("++[>+<-]")

ast
# %ASTNode{
#   rule_name: "program",
#   start_line: 1, start_column: 1,
#   end_line: 1,   end_column: 8,
#   children: [
#     %ASTNode{rule_name: "instruction", children: [
#       %ASTNode{rule_name: "command", children: [
#         %Token{type: "INC", value: "+", line: 1, column: 1}
#       ]}
#     ]},
#     ...
#     %ASTNode{rule_name: "instruction", children: [
#       %ASTNode{rule_name: "loop", children: [
#         %Token{type: "LOOP_START", value: "[", line: 1, column: 3},
#         ...
#         %Token{type: "LOOP_END",   value: "]", line: 1, column: 8}
#       ]}
#     ]}
#   ]
# }
```

Error case:

```elixir
{:error, msg} = Brainfuck.Parser.parse("++[>+<-")
# {:error, "line 1, column 8: expected ']' to close loop opened at line 1, column 3"}
```

## Implementation

```elixir
defmodule Brainfuck.Parser do
  @grammar_path Path.join([
    :code.priv_dir(:brainfuck),
    "../../grammars/brainfuck.grammar"
  ])

  def parse(source) do
    with {:ok, tokens} <- Brainfuck.Lexer.tokenize(source) do
      parse_tokens(tokens)
    end
  end

  def parse_tokens(tokens) do
    with {:ok, grammar} <- CodingAdventures.Parser.load_grammar(@grammar_path) do
      CodingAdventures.Parser.GrammarParser.parse(tokens, grammar)
    end
  end
end
```

Again: no hand-written recursive descent. The grammar file is the specification; the generic parser infrastructure handles the rest.

## Relationship to Existing Code

The parser sits between the new `Brainfuck.Lexer` (spec `BF00`) and the new `Brainfuck.Compiler` (spec `BF02`). The compilation pipeline becomes:

```
Source string
    ↓ Brainfuck.Lexer.tokenize/1
[Token.t(), ...]
    ↓ Brainfuck.Parser.parse_tokens/1
ASTNode.t()   (program root)
    ↓ Brainfuck.Compiler.compile/1    (spec BF02)
{CodeObject.t(), Sidecar.t()}
    ↓ Brainfuck.VM.execute/2
BrainfuckResult.t()
```

The `Brainfuck.Translator` module is deleted. Its bracket-matching logic is superseded by the parser's grammar-driven loop rule; its character-to-opcode mapping is superseded by the compiler's rule handlers.
