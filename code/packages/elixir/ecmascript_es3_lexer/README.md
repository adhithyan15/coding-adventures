# ecmascript_es3_lexer

Tokenizes ECMAScript 3 (1999) source code using the grammar-driven lexer approach.

ES3 (ECMA-262 3rd Edition) made JavaScript a complete language by adding strict equality (`===`/`!==`), `try`/`catch`/`finally`/`throw`, regex literals, and `instanceof`.

## Dependencies

- grammar_tools (parses `.tokens` files)
- lexer (grammar-driven tokenizer engine)

## Usage

```elixir
{:ok, tokens} = CodingAdventures.EcmascriptEs3Lexer.tokenize("x === 1")
```

## Development

```bash
# Run tests
bash BUILD
```
