# ecmascript_es5_parser

Parses ECMAScript 5 (2009) source code into ASTs using the grammar-driven parser approach.

Combines the ES5 lexer with the grammar-driven parser engine. Supports all ES3 features plus the `debugger` statement and getter/setter property definitions.

## Dependencies

- parser (grammar-driven parser engine)
- ecmascript_es5_lexer (ES5 tokenizer)

## Usage

```elixir
{:ok, ast} = CodingAdventures.EcmascriptEs5Parser.parse("debugger;")
```

## Development

```bash
# Run tests
bash BUILD
```
