# VHDL Lexer

A Ruby gem that tokenizes VHDL (VHSIC Hardware Description Language) source code using the grammar-driven lexer engine.

## Overview

This gem is a wrapper around `coding_adventures_lexer`'s `GrammarLexer`. It loads the `vhdl.tokens` grammar file and feeds it to the general-purpose lexer engine to tokenize VHDL source code.

VHDL differs from Verilog in several key ways:
- **Case insensitive** -- `ENTITY`, `Entity`, and `entity` are identical. This lexer normalizes NAME and KEYWORD values to lowercase.
- **No preprocessor** -- VHDL has no preprocessor directives (no `define, `ifdef, etc.).
- **Ada-like syntax** -- verbose, strongly typed, with explicit declarations.
- **Based literals** -- `16#FF#`, `2#1010#` instead of Verilog's `8'hFF`, `4'b1010`.
- **Bit string literals** -- `X"FF"`, `B"1010"` for hardware values.
- **Character literals** -- `'0'`, `'1'`, `'Z'` for std_logic values.
- **Keyword operators** -- `and`, `or`, `xor`, `not` instead of `&`, `|`, `^`, `~`.

## Usage

```ruby
require "coding_adventures_vhdl_lexer"

# Basic tokenization
tokens = CodingAdventures::VhdlLexer.tokenize("signal clk : std_logic;")
tokens.each { |t| puts t }

# Case insensitivity -- these produce identical tokens
tokens1 = CodingAdventures::VhdlLexer.tokenize("ENTITY counter IS")
tokens2 = CodingAdventures::VhdlLexer.tokenize("entity counter is")
# Both produce keyword "entity", name "counter", keyword "is"
```

## Dependencies

- `coding_adventures_grammar_tools` -- reads the `.tokens` grammar file
- `coding_adventures_lexer` -- the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
