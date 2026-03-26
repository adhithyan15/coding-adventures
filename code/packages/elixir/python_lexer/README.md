# Python Lexer (Elixir)

Thin wrapper around the grammar-driven lexer engine for Python tokenization.

## Usage

```elixir
{:ok, tokens} = CodingAdventures.PythonLexer.tokenize("x = 1 + 2")
```

## How It Works

Reads `python.tokens` from the shared grammars directory, parses it with `grammar_tools`, and delegates tokenization to the shared `lexer` package. The parsed grammar is cached with `persistent_term` for repeated use.

## Dependencies

- `grammar_tools` — parses `.tokens` files
- `lexer` — grammar-driven tokenization engine
