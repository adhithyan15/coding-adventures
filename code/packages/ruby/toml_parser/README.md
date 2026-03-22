# TOML Parser (Ruby)

Parses TOML v1.0.0 text into ASTs using the grammar-driven parser engine.

## Usage

```ruby
require "coding_adventures_toml_parser"

ast = CodingAdventures::TomlParser.parse('[server]\nhost = "localhost"')
# => ASTNode(rule_name: "document", children: [...])
```

## How It Works

The parser operates in a pipeline:

1. **Tokenize** — `toml.tokens` + `GrammarLexer` → token stream
2. **Parse** — `toml.grammar` + `GrammarDrivenParser` → AST

This is the syntax phase only. Semantic validation (key uniqueness, table
consistency) is handled by the Python reference implementation's converter.

## Dependencies

- `coding_adventures_toml_lexer` — TOML tokenizer
- `coding_adventures_parser` — grammar-driven parser engine
- `coding_adventures_grammar_tools` — parses `.grammar` files
