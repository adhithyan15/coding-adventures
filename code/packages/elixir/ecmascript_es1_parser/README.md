# ecmascript_es1_parser

Parses ECMAScript 1 (1997) source code into ASTs using the grammar-driven parser approach.

Combines the ES1 lexer with the grammar-driven parser engine to produce a full parse tree from ES1 JavaScript source code.

## Dependencies

- parser (grammar-driven parser engine)
- ecmascript_es1_lexer (ES1 tokenizer)

## Usage

```elixir
{:ok, ast} = CodingAdventures.EcmascriptEs1Parser.parse("var x = 1 + 2;")
```

## Development

```bash
# Run tests
bash BUILD
```
