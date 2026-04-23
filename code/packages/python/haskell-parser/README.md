# Haskell Parser

Parses Haskell source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `lang_parser` package. It loads `haskell.grammar` and delegates all parsing to the generic engine.

## How It Fits in the Stack

```
Haskell source code
    |
    v
haskell_lexer.tokenize_haskell()            -- tokenizes using haskell{version}.tokens
    |
    v
haskell{version}.grammar (grammar file)
    |
    v
grammar_tools.parse_parser_grammar()   -- parses the .grammar file
    |
    v
lang_parser.GrammarParser              -- generic parsing engine
    |
    v
haskell_parser.parse_haskell()              -- thin wrapper (this package)
    |
    v
ASTNode tree                           -- generic AST
```

## Version Support

Supports Haskell versions: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`.
Default is Haskell 21 (latest).

## Usage

```python
from haskell_parser import parse_haskell

ast = parse_haskell('public class Hello { }')
print(ast.rule_name)  # "program"

# Use a specific Haskell version
ast = parse_haskell('var x = 1;', '10')
```

## Dependencies

- `coding-adventures-haskell-lexer` -- tokenizes Haskell source code
- `coding-adventures-parser` -- provides `GrammarParser` and `ASTNode`
- `coding-adventures-grammar-tools` -- parses `.grammar` files
