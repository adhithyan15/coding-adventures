# CSS Parser

Parses CSS text (Level 3) into ASTs using the grammar-driven parser — a thin
wrapper that loads `css.grammar` and feeds it to the generic `GrammarParser`.

## What Is This?

This package is a **thin wrapper** around the grammar-driven `GrammarParser`. It
tokenizes CSS using the `css-lexer` package, then parses the token stream using
the EBNF rules defined in `css.grammar`. The result is a generic `ASTNode` tree.

CSS is the most complex grammar in the collection — a deliberate stress test
for the parser infrastructure. It exercises backtracking, literal matching,
deep nesting, and diverse rule structures.

## How It Fits in the Stack

```
css.tokens          css.grammar
    │                    │
    ▼                    ▼
css_lexer          grammar_tools
(tokenize_css)     (parse_parser_grammar)
    │                    │
    └──────┬─────────────┘
           ▼
    GrammarParser
           │
           ▼
    css_parser.parse_css()
           │
           ▼
      ASTNode tree
```

## Usage

```python
from css_parser import parse_css

ast = parse_css('h1 { color: red; }')
print(ast.rule_name)  # "stylesheet"
```

### Complex CSS

```python
from css_parser import parse_css

css = """
@media screen and (min-width: 768px) {
    .container > .content {
        font-size: 16px;
        color: #333;
    }
}
"""
ast = parse_css(css)
```

## Installation

```bash
pip install coding-adventures-css-parser
```

## Dependencies

- `coding-adventures-css-lexer` — tokenizes CSS text
- `coding-adventures-grammar-tools` — parses the `.grammar` file
- `coding-adventures-lexer` — provides the token types
- `coding-adventures-parser` — provides the `GrammarParser` engine
