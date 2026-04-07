# BF00 — Brainfuck Lexer

## Overview

Brainfuck has exactly eight commands. Every other character in a Brainfuck source file is a comment. This makes it the simplest possible language to tokenize — and therefore the ideal first test of the grammar-driven lexer infrastructure.

This spec adds a proper `brainfuck.tokens` grammar file and a `Brainfuck.Lexer` module that produces standard `Token.t()` structs. This replaces the ad-hoc character scanning currently embedded in `Brainfuck.Translator` and wires Brainfuck into the same lexer pipeline used by every other language in the toolchain.

## Why a Separate Lexer?

The existing `Translator` module contains this implicit tokenization logic:

```elixir
# From Translator — character scanning baked into compilation
@char_to_op %{ ">" => @right, "<" => @left, ... }

defp translate_chars([char | rest], instructions, stack, index) do
  case Map.get(@char_to_op, char) do
    nil -> translate_chars(rest, instructions, stack, index)  # skip comment char
    op  -> ...
  end
end
```

This conflates *recognising* tokens with *compiling* them. Separating the concerns gives:

1. **A proper token stream** for the parser and LSP bridge (spec `LS01`)
2. **Source positions** on every token — line and column — needed for the debug sidecar and diagnostic squiggles
3. **Reuse** — the same lexer is consumed by the parser, the bytecode compiler, and the language server

## Token Types

Brainfuck has nine token types emitted by the lexer:

| Token type | Source character | Meaning |
|---|---|---|
| `RIGHT` | `>` | Move data pointer right |
| `LEFT` | `<` | Move data pointer left |
| `INC` | `+` | Increment current cell |
| `DEC` | `-` | Decrement current cell |
| `OUTPUT` | `.` | Output current cell as ASCII |
| `INPUT` | `,` | Read one byte into current cell |
| `LOOP_START` | `[` | Begin loop |
| `LOOP_END` | `]` | End loop |
| `EOF` | — | End of input (always the last token) |

Comment characters (everything else) are consumed by the lexer and discarded. They never appear in the token stream. This matches the pattern used by Lisp comments (`;`-to-end-of-line) in `lisp.tokens` — by putting comments in the `skip:` section, the lexer infrastructure handles them transparently.

## The Grammar File: `brainfuck.tokens`

```
# Token definitions for Brainfuck
# @version 1
#
# Brainfuck has exactly eight meaningful characters. Everything else is a
# comment — the programmer's way of annotating their code since Brainfuck
# has no dedicated comment syntax. Comments are skipped at the lexer level
# so they never reach the parser.
#
# Example source showing commands and embedded comments:
#
#   +++   set cell 0 to 3
#   [     loop while cell 0 is nonzero
#     >+  move right, increment
#     <-  move left, decrement
#   ]
#
# Tokens are listed in operator-first order. Single-character literals
# are used throughout — Brainfuck has no multi-character tokens.

# ---------------------------------------------------------------------------
# Pointer movement
# ---------------------------------------------------------------------------
RIGHT = ">"
LEFT  = "<"

# ---------------------------------------------------------------------------
# Cell arithmetic
# ---------------------------------------------------------------------------
INC   = "+"
DEC   = "-"

# ---------------------------------------------------------------------------
# I/O
# ---------------------------------------------------------------------------
OUTPUT = "."
INPUT  = ","

# ---------------------------------------------------------------------------
# Loop control
# ---------------------------------------------------------------------------
LOOP_START = "["
LOOP_END   = "]"

# ---------------------------------------------------------------------------
# Skip patterns — consumed silently, never emitted as tokens
# ---------------------------------------------------------------------------
# Any character that is not one of the eight commands is a comment. The
# character class [^><+\-.,\[\]] matches everything except commands.
# This includes letters, digits, punctuation, spaces, and newlines.
#
# Why skip rather than emit? The lexer infrastructure's skip: section
# discards tokens before they reach the token stream. This keeps the
# parser grammar clean — it never has to say "instruction or comment"
# in every single production rule.
#
# The LSP bridge derives comment source ranges directly from the source
# text and the command token positions rather than needing explicit
# COMMENT tokens (see LS01 and BF00's semantic_tokens notes below).

skip:
  WHITESPACE = /[ \t\r\n]+/
  COMMENT    = /[^><+\-.,\[\] \t\r\n]+/
```

### Why two skip patterns?

Splitting whitespace and non-command non-whitespace into two patterns preserves correct line/column tracking. The lexer increments the line counter when it encounters `\n`. If COMMENT consumed `\n` as well, the line counter would drift. Separating them ensures:

- `WHITESPACE` handles all line endings → line counter stays accurate
- `COMMENT` handles all other non-command characters on the same line

## Public API

```elixir
defmodule Brainfuck.Lexer do
  @moduledoc """
  Tokenises Brainfuck source code into a stream of Token structs.

  Each of the eight Brainfuck commands becomes a token with its type,
  value (the character itself), and source position (1-based line and
  column). Comment characters are discarded by the lexer — they never
  appear in the returned list.

  The final token in every successful result is always EOF.
  """

  alias CodingAdventures.Lexer.Token

  @spec tokenize(String.t()) :: {:ok, [Token.t()]} | {:error, String.t()}
  def tokenize(source)
end
```

### Example

```elixir
{:ok, tokens} = Brainfuck.Lexer.tokenize("++[>+<-]")

# [
#   %Token{type: "INC",        value: "+", line: 1, column: 1},
#   %Token{type: "INC",        value: "+", line: 1, column: 2},
#   %Token{type: "LOOP_START", value: "[", line: 1, column: 3},
#   %Token{type: "RIGHT",      value: ">", line: 1, column: 4},
#   %Token{type: "INC",        value: "+", line: 1, column: 5},
#   %Token{type: "LEFT",       value: "<", line: 1, column: 6},
#   %Token{type: "DEC",        value: "-", line: 1, column: 7},
#   %Token{type: "LOOP_END",   value: "]", line: 1, column: 8},
#   %Token{type: "EOF",        value: "",  line: 1, column: 9}
# ]
```

Comments are transparently discarded:

```elixir
{:ok, tokens} = Brainfuck.Lexer.tokenize("++ increment\n[loop]")

# [
#   %Token{type: "INC",        value: "+", line: 1, column: 1},
#   %Token{type: "INC",        value: "+", line: 1, column: 2},
#   # "increment" was skipped — no COMMENT token emitted
#   %Token{type: "LOOP_START", value: "[", line: 2, column: 1},
#   %Token{type: "LOOP_END",   value: "]", line: 2, column: 6},
#   %Token{type: "EOF",        value: "",  line: 2, column: 7}
# ]
```

Note that despite skipping `" increment\n"`, the line counter correctly shows `[` on line 2, column 1. This works because `WHITESPACE` in the skip pattern consumes the newline and the lexer infrastructure increments the line counter.

## Implementation

The implementation delegates to the shared grammar-driven lexer infrastructure, identical to all other lexers in the toolchain:

```elixir
defmodule Brainfuck.Lexer do
  @grammar_path Path.join([
    :code.priv_dir(:brainfuck),
    "../../grammars/brainfuck.tokens"
  ])

  def tokenize(source) do
    with {:ok, grammar} <- CodingAdventures.Lexer.load_grammar(@grammar_path) do
      CodingAdventures.Lexer.GrammarLexer.tokenize(source, grammar)
    end
  end
end
```

No custom tokenisation logic. No manual character scanning. The grammar file *is* the specification; the implementation is boilerplate.

## Note on LSP Semantic Tokens

Since COMMENT characters are skipped by the lexer, the LSP bridge (spec `LS01`) cannot recover them as tokens. Instead, the bridge derives comment source ranges by computing the complement: every source position not covered by a command token is a comment.

```elixir
# In the Brainfuck LSP bridge
def semantic_tokens(source, command_tokens) do
  command_positions = MapSet.new(command_tokens, fn t ->
    {t.line, t.column}
  end)

  # Walk source char by char; anything not a command char is a comment run
  comment_ranges = derive_complement_ranges(source, command_positions)

  sem_tokens =
    Enum.map(command_tokens, &to_semantic_token/1) ++
    Enum.map(comment_ranges, &comment_range_to_token/1)

  {:ok, Enum.sort_by(sem_tokens, &{&1.line, &1.character})}
end
```

This is the standard pattern for languages where "comments" are defined negatively (everything that isn't a command) rather than by an explicit comment syntax.

## Relationship to Existing Code

This lexer **replaces** the character-scanning loop in `Brainfuck.Translator`. After this spec is implemented:

- `Brainfuck.Translator` is deleted (superseded by `BF02` compiler)
- `Brainfuck.Lexer` is the new entry point for source → tokens
- `Brainfuck.Parser` (spec `BF01`) consumes the token stream
- `Brainfuck.Compiler` (spec `BF02`) compiles the AST to bytecode

The opcodes (`Brainfuck.Opcodes`) and handlers (`Brainfuck.Handlers`) are unchanged — only the front-end pipeline changes.
