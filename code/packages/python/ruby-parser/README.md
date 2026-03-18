# Ruby Parser

Parses Ruby source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `lang_parser` package. It demonstrates a core principle of the grammar-driven architecture: the same parser engine that parses Python can parse Ruby by simply loading a different `.grammar` file.

No new parser code is needed. The `ruby.grammar` file in `code/grammars/` declares Ruby's grammar rules in EBNF notation, and the `GrammarParser` interprets those rules at runtime.

## How It Fits in the Stack

```
Ruby source code
    |
    v
ruby_lexer.tokenize_ruby()          -- tokenizes using ruby.tokens
    |
    v
ruby.grammar (grammar file)
    |
    v
grammar_tools.parse_parser_grammar() -- parses the .grammar file
    |
    v
lang_parser.GrammarParser            -- generic parsing engine
    |
    v
ruby_parser.parse_ruby()            -- thin wrapper (this package)
    |
    v
ASTNode tree                         -- generic AST
```

## Usage

```python
from ruby_parser import parse_ruby

# Parse a simple assignment
ast = parse_ruby('x = 1 + 2')
print(ast.rule_name)  # "program"

# Parse multiple statements
ast = parse_ruby('x = 1\ny = 2')
print(len(ast.children))  # 2 statements

# Parse method calls
ast = parse_ruby('puts("hello")')
```

## Installation

```bash
pip install coding-adventures-ruby-parser
```

## Dependencies

- `coding-adventures-ruby-lexer` -- tokenizes Ruby source code
- `coding-adventures-parser` -- provides `GrammarParser` and `ASTNode`
- `coding-adventures-grammar-tools` -- parses `.grammar` files
