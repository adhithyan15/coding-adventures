# TOML Parser

A grammar-driven parser for [TOML](https://toml.io/) v1.0.0 that produces an Abstract Syntax Tree (AST).

## What it does

This crate parses TOML source text into a tree structure (AST) that reflects the nested organization of the configuration data. It uses the `toml-lexer` crate for tokenization and loads the `toml.grammar` file for grammar rules, feeding both into the generic `GrammarParser` from the `parser` crate.

## How it fits in the stack

```text
Source text  ("title = \"TOML\"\n[server]\nport = 8080")
      |
      v
toml-lexer           -> Vec<Token>
      |
      v
toml.grammar         -> ParserGrammar (12 rules)
      |
      v
parser::GrammarParser -> GrammarASTNode tree
      |
      v
toml-parser          (this crate — thin glue layer)
```

## Grammar rules

The TOML grammar has ~12 rules:

```ebnf
document           = { NEWLINE | expression } ;
expression         = array_table_header | table_header | keyval ;
keyval             = key EQUALS value ;
key                = simple_key { DOT simple_key } ;
simple_key         = BARE_KEY | BASIC_STRING | LITERAL_STRING | ... ;
table_header       = LBRACKET key RBRACKET ;
array_table_header = LBRACKET LBRACKET key RBRACKET RBRACKET ;
value              = STRING | INTEGER | FLOAT | BOOLEAN | DATE | array | inline_table ;
array              = LBRACKET array_values RBRACKET ;
array_values       = { NEWLINE } [ value { COMMA value } [ COMMA ] ] ;
inline_table       = LBRACE [ keyval { COMMA keyval } ] RBRACE ;
```

## Usage

```rust
use coding_adventures_toml_parser::parse_toml;

let ast = parse_toml("[server]\nhost = \"localhost\"\nport = 8080");
assert_eq!(ast.rule_name, "document");
```

Or use the factory function for fine-grained control:

```rust
use coding_adventures_toml_parser::create_toml_parser;

let mut parser = create_toml_parser("key = 42");
let ast = parser.parse().expect("parse failed");
```

## Key differences from json-parser

- **12 rules vs 4** — TOML's grammar is more complex than JSON's.
- **Newline-delimited** — expressions are separated by newlines, not just structural tokens.
- **Table headers** — `[table]` and `[[array-of-tables]]` have no JSON equivalent.
- **Key types** — bare, quoted, and dotted keys vs JSON's quoted-only.
- **Start symbol is `document`** — not `value` as in JSON.

## Running tests

```bash
cargo test -p coding-adventures-toml-parser -- --nocapture
```
