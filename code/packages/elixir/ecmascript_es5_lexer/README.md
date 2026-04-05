# ecmascript_es5_lexer

Tokenizes ECMAScript 5 (2009) source code using the grammar-driven lexer approach.

ES5 (ECMA-262 5th Edition) adds the `debugger` keyword, getter/setter syntax, and string line continuations on top of ES3. The real ES5 innovations were semantic (strict mode, JSON, property descriptors).

## Dependencies

- grammar_tools (parses `.tokens` files)
- lexer (grammar-driven tokenizer engine)

## Usage

```elixir
{:ok, tokens} = CodingAdventures.EcmascriptEs5Lexer.tokenize("debugger;")
```

## Development

```bash
# Run tests
bash BUILD
```
