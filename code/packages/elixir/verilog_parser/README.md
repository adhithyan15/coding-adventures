# Verilog Parser (Elixir)

Thin wrapper around the grammar-driven parser engine for Verilog parsing.

## Usage

```elixir
{:ok, ast} = CodingAdventures.VerilogParser.parse(~s({"name": "Alice", "age": 30}))
# => %ASTNode{rule_name: "value", children: [%ASTNode{rule_name: "object", ...}]}
```

## How It Works

Combines `VerilogLexer.tokenize/1` with `GrammarParser.parse/2` using `verilog.grammar` from the shared grammars directory. The grammar is cached via `persistent_term`.

## Dependencies

- `grammar_tools` — parses `.grammar` files
- `lexer` — grammar-driven tokenization engine
- `parser` — grammar-driven parsing engine
- `verilog_lexer` — Verilog tokenization
