# Java Parser

Parses Java source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `lang_parser` package. It loads `java.grammar` and delegates all parsing to the generic engine.

## How It Fits in the Stack

```
Java source code
    |
    v
java_lexer.tokenize_java()            -- tokenizes using java{version}.tokens
    |
    v
java{version}.grammar (grammar file)
    |
    v
grammar_tools.parse_parser_grammar()   -- parses the .grammar file
    |
    v
lang_parser.GrammarParser              -- generic parsing engine
    |
    v
java_parser.parse_java()              -- thin wrapper (this package)
    |
    v
ASTNode tree                           -- generic AST
```

## Version Support

Supports Java versions: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`.
Default is Java 21 (latest).

## Usage

```python
from java_parser import parse_java

ast = parse_java('public class Hello { }')
print(ast.rule_name)  # "program"

# Use a specific Java version
ast = parse_java('var x = 1;', '10')
```

## Dependencies

- `coding-adventures-java-lexer` -- tokenizes Java source code
- `coding-adventures-parser` -- provides `GrammarParser` and `ASTNode`
- `coding-adventures-grammar-tools` -- parses `.grammar` files
