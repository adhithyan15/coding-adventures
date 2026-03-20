# JSON Parser

Parses JSON text (RFC 8259) into ASTs using the grammar-driven parser — a thin
wrapper that loads `json.grammar` and feeds it to the generic `GrammarParser`.

## What Is This?

This package is a **thin wrapper** around the grammar-driven `GrammarParser`. It
tokenizes JSON using the `json-lexer` package, then parses the token stream using
the EBNF rules defined in `json.grammar`. The result is a generic `ASTNode` tree.

## How It Fits in the Stack

```
json.tokens + json.grammar  (declarative definitions)
    |              |
    v              v
json_lexer     grammar_tools.parse_parser_grammar()
    |              |
    v              v
list[Token]    ParserGrammar
    |              |
    +------+-------+
           |
           v
    GrammarParser  (generic engine from parser package)
           |
           v
    json_parser.parse_json()  (this thin wrapper)
           |
           v
       ASTNode tree
```

## Usage

```python
from json_parser import parse_json

ast = parse_json('{"name": "Ada", "age": 36}')
print(ast.rule_name)  # "value"
print(ast.children[0].rule_name)  # "object"
```

## Installation

```bash
pip install coding-adventures-json-parser
```

## Dependencies

- `coding-adventures-json-lexer` — tokenizes JSON text
- `coding-adventures-grammar-tools` — parses the `.grammar` file
- `coding-adventures-lexer` — provides the token types
- `coding-adventures-parser` — provides the `GrammarParser` engine
