# ecmascript_es3_parser

Parses ECMAScript 3 (1999) source code into ASTs using the grammar-driven parser approach.

Combines the ES3 lexer with the grammar-driven parser engine. Supports all ES1 features plus `try`/`catch`/`finally`/`throw`, strict equality, `instanceof`, and regex literals.

## Dependencies

- parser (grammar-driven parser engine)
- ecmascript_es3_lexer (ES3 tokenizer)

## Usage

```elixir
{:ok, ast} = CodingAdventures.EcmascriptEs3Parser.parse("try { x(); } catch(e) { }")
```

## Development

```bash
# Run tests
bash BUILD
```
