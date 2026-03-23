# JSON Lexer (Elixir)

Thin wrapper around the grammar-driven lexer engine for JSON tokenization.

## Usage

```elixir
{:ok, tokens} = CodingAdventures.JsonLexer.tokenize(~s({"key": 42}))
# => [%Token{type: "LBRACE"}, %Token{type: "STRING", value: "key"}, ...]
```

## How It Works

Reads `json.tokens` from the shared grammars directory and delegates to `GrammarLexer.tokenize/2`. The grammar is cached via `persistent_term` for fast repeated access.

## Dependencies

- `grammar_tools` — parses `.tokens` files
- `lexer` — grammar-driven tokenization engine
