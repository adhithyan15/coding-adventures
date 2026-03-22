# TOML Parser (Elixir)

Thin wrapper around the grammar-driven parser engine for TOML parsing.

## Usage

```elixir
{:ok, ast} = CodingAdventures.TomlParser.parse(~s(title = "TOML Example"))
# => %ASTNode{rule_name: "document", children: [%ASTNode{rule_name: "expression", ...}]}
```

## How It Works

Combines `TomlLexer.tokenize/1` with `GrammarParser.parse/2` using `toml.grammar` from the shared grammars directory. The grammar is cached via `persistent_term`.

The TOML grammar has 11 rules covering:
- **Document structure:** `document`, `expression`
- **Key-value pairs:** `keyval`, `key`, `simple_key`
- **Table headers:** `table_header`, `array_table_header`
- **Values:** `value`, `array`, `array_values`, `inline_table`

TOML is newline-sensitive — the grammar uses NEWLINE tokens to delimit expressions within a document. Array-of-tables headers (`[[name]]`) are parsed as two LBRACKET tokens, disambiguated from nested arrays by context.

## Dependencies

- `grammar_tools` — parses `.grammar` files
- `lexer` — grammar-driven tokenization engine
- `parser` — grammar-driven parsing engine
- `toml_lexer` — TOML tokenization
