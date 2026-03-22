# TOML Lexer (Elixir)

Thin wrapper around the grammar-driven lexer engine for TOML tokenization.

## Usage

```elixir
{:ok, tokens} = CodingAdventures.TomlLexer.tokenize(~s(title = "TOML Example"))
# => [%Token{type: "BARE_KEY", value: "title"}, %Token{type: "EQUALS"}, ...]
```

## How It Works

Reads `toml.tokens` from the shared grammars directory and delegates to `GrammarLexer.tokenize/2`. The grammar is cached via `persistent_term` for fast repeated access.

TOML is newline-sensitive, so the lexer emits `NEWLINE` tokens between lines. The skip pattern only covers spaces, tabs, and comments (not newlines).

TOML has four string types with different escape semantics. The grammar uses `escapes: none` to defer escape processing to the parser's semantic layer.

## Token Types

- **Strings:** `BASIC_STRING`, `LITERAL_STRING`, `ML_BASIC_STRING`, `ML_LITERAL_STRING`
- **Numbers:** `INTEGER`, `FLOAT` (including hex, octal, binary, inf, nan)
- **Booleans:** `TRUE`, `FALSE`
- **Date/Time:** `OFFSET_DATETIME`, `LOCAL_DATETIME`, `LOCAL_DATE`, `LOCAL_TIME`
- **Keys:** `BARE_KEY`
- **Delimiters:** `EQUALS`, `DOT`, `COMMA`, `LBRACKET`, `RBRACKET`, `LBRACE`, `RBRACE`
- **Whitespace:** `NEWLINE` (emitted), `WHITESPACE` and `COMMENT` (skipped)

## Dependencies

- `grammar_tools` — parses `.tokens` files
- `lexer` — grammar-driven tokenization engine
