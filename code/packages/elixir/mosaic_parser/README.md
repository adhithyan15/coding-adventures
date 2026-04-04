# Mosaic Parser (Elixir)

Thin wrapper around the grammar-driven parser engine for Mosaic parsing.

Mosaic is a Component Description Language (CDL) for declaring UI component
structure with named typed slots. This package handles the syntactic layer —
turning a token stream from `mosaic_lexer` into an AST.

## Usage

```elixir
source = ~S"""
component ProfileCard {
  slot display-name: text;
  slot avatar-url: image;

  Column {
    Image { src: @avatar-url; }
    Text  { content: @display-name; }
  }
}
"""

{:ok, ast} = CodingAdventures.MosaicParser.parse(source)
ast.rule_name  # => "file"
```

## AST Structure

The root node always has `rule_name: "file"`. Key grammar rules:

| Rule                  | Matches                                              |
|-----------------------|------------------------------------------------------|
| `file`                | Top-level: optional `import_decl`* + `component_decl` |
| `component_decl`      | `component Name { slot_decl* node_tree }`            |
| `slot_decl`           | `slot name: type [= default];`                       |
| `node_element`        | `Name { node_content* }`                             |
| `property_assignment` | `(name|keyword): value;`                             |
| `slot_ref`            | `@name`                                              |
| `slot_reference`      | `@name;` as a child element                          |
| `when_block`          | `when @flag { node_content* }`                       |
| `each_block`          | `each @list as item { node_content* }`               |

## How It Works

1. `MosaicLexer.tokenize/1` converts source text into a token list.
2. `GrammarParser.parse/2` applies the rules from `mosaic.grammar` to produce
   an `ASTNode` tree.
3. Both the lexer grammar and parser grammar are cached via `:persistent_term`.

## Dependencies

- `grammar_tools` — parses `.grammar` files into `ParserGrammar` structs
- `lexer` — grammar-driven tokenization engine
- `parser` — grammar-driven parsing engine (`GrammarParser`, `ASTNode`)
- `mosaic_lexer` — Mosaic-specific tokenization
- `directed_graph` — used internally by `grammar_tools`
