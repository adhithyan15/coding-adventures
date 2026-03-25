# VHDL Lexer (Elixir)

Tokenizes VHDL source code using the grammar-driven lexer engine, with automatic case normalization for VHDL's case-insensitive semantics.

## Usage

```elixir
# Basic tokenization
{:ok, tokens} = CodingAdventures.VhdlLexer.tokenize("entity counter is end counter;")
# => [%Token{type: "KEYWORD", value: "entity"}, %Token{type: "NAME", value: "counter"}, ...]

# Case insensitivity — these produce identical token streams
{:ok, t1} = CodingAdventures.VhdlLexer.tokenize("ENTITY Counter IS END Counter;")
{:ok, t2} = CodingAdventures.VhdlLexer.tokenize("entity counter is end counter;")
# t1 and t2 have the same token values (all lowercase)
```

## Case Normalization

VHDL is case-insensitive: `ENTITY`, `Entity`, and `entity` all mean the same thing. After tokenization, this lexer normalizes all NAME and KEYWORD token values to lowercase. Extended identifiers (`\name\`) preserve their original case, as required by the VHDL standard.

## Key Differences from Verilog Lexer

- **No preprocessor** — VHDL has no preprocessor directives. No `define, `ifdef, or `include.
- **Case normalization** — NAME and KEYWORD values are lowercased after tokenization.
- **Character literals** — Single characters in tick marks: `'0'`, `'1'`, `'X'`, `'Z'`.
- **Bit string literals** — Base-prefixed strings: `B"1010"`, `X"FF"`, `O"77"`.
- **Keyword operators** — Logical operations are keywords (`and`, `or`, `xor`), not symbols.
- **Different operators** — `:=` (variable assign), `/=` (not equal), `=>` (arrow), `<>` (box).

## How It Works

1. Reads `vhdl.tokens` from the shared grammars directory
2. Delegates to `GrammarLexer.tokenize/2` for grammar-driven tokenization
3. Post-processes tokens to normalize NAME and KEYWORD values to lowercase
4. Grammar is cached via `persistent_term` for fast repeated access

## Dependencies

- `grammar_tools` — parses `.tokens` files
- `lexer` — grammar-driven tokenization engine
