# JSON Parser

A grammar-driven parser for [JSON](https://www.json.org/) (RFC 8259) that produces an Abstract Syntax Tree (AST).

## What it does

This crate parses JSON source text into a tree structure (AST) that reflects the nested organization of the data. It uses the `json-lexer` crate for tokenization and loads the `json.grammar` file for grammar rules, feeding both into the generic `GrammarParser` from the `parser` crate.

## How it fits in the stack

```text
Source text  ("{\"name\": \"Alice\", \"age\": 30}")
      |
      v
json-lexer           → Vec<Token>
      |
      v
json.grammar         → ParserGrammar (4 rules: value, object, pair, array)
      |
      v
parser::GrammarParser → GrammarASTNode tree
      |
      v
json-parser          (this crate — thin glue layer)
```

## Grammar rules

The JSON grammar has only four rules:

```ebnf
value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
object = LBRACE [ pair { COMMA pair } ] RBRACE ;
pair   = STRING COLON value ;
array  = LBRACKET [ value { COMMA value } ] RBRACKET ;
```

The mutual recursion between `value`, `object`, and `array` allows arbitrarily deep nesting.

## Usage

```rust
use coding_adventures_json_parser::parse_json;

let ast = parse_json("{\"key\": [1, 2, 3]}");
assert_eq!(ast.rule_name, "value");
```

Or use the factory function for fine-grained control:

```rust
use coding_adventures_json_parser::create_json_parser;

let mut parser = create_json_parser("{\"key\": 42}");
let ast = parser.parse().expect("parse failed");
```

## Key differences from starlark-parser

- **4 rules vs ~40** — JSON's grammar is dramatically simpler than Starlark's.
- **No statements** — JSON has only values (no assignment, no control flow).
- **No indentation** — No INDENT/DEDENT tokens to handle.
- **Start symbol is `value`** — not `file` as in Starlark.

## Running tests

```bash
cargo test -p coding-adventures-json-parser -- --nocapture
```
