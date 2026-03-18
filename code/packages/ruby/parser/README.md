# coding_adventures_parser

Recursive descent parser with hand-written and grammar-driven modes. This is the Ruby port of the Python `parser` package.

## What It Does

Builds Abstract Syntax Trees (ASTs) from token streams. Provides two approaches:

1. **RecursiveDescentParser** -- hand-written parser producing typed AST nodes
2. **GrammarDrivenParser** -- reads .grammar files and produces generic ASTNode objects

## AST Nodes

- `NumberLiteral` -- integer literal (value)
- `StringLiteral` -- string literal (value)
- `Name` -- variable reference (name)
- `BinaryOp` -- binary operation (left, op, right)
- `Assignment` -- variable assignment (target, value)
- `Program` -- root node (statements)

## Operator Precedence

- `*`, `/` bind tighter than `+`, `-`
- Parentheses override precedence

## Usage

```ruby
require "coding_adventures_parser"

tokens = CodingAdventures::Lexer::Tokenizer.new("x = 1 + 2 * 3").tokenize
ast = CodingAdventures::Parser::RecursiveDescentParser.new(tokens).parse
```

## Dependencies

- `coding_adventures_lexer`
- `coding_adventures_grammar_tools`
