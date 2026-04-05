# ecmascript_es1_lexer

Tokenizes ECMAScript 1 (1997) source code using the grammar-driven lexer approach.

ES1 (ECMA-262 1st Edition) is the very first standardized version of JavaScript, covering basic keywords, operators, string/numeric literals, and identifiers.

## Dependencies

- grammar_tools (parses `.tokens` files)
- lexer (grammar-driven tokenizer engine)

## Usage

```elixir
{:ok, tokens} = CodingAdventures.EcmascriptEs1Lexer.tokenize("var x = 1 + 2;")
```

## Development

```bash
# Run tests
bash BUILD
```
