# Python Parser (TypeScript)

Parses Python source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It demonstrates a core principle of the grammar-driven architecture: the same parser engine that parses one language can parse Python by simply loading a different `.grammar` file.

No new parser code is needed. The `python.grammar` file in `code/grammars/` declares Python's grammar rules in EBNF notation, and the `GrammarParser` interprets those rules at runtime.

## How It Fits in the Stack

```
Python source code
    |
    v
tokenizePython()                -- tokenizes using python.tokens
    |
    v
python.grammar (grammar file)
    |
    v
parseParserGrammar()            -- parses the .grammar file
    |
    v
GrammarParser                   -- generic parsing engine
    |
    v
parsePython()                   -- thin wrapper (this package)
    |
    v
ASTNode tree                    -- generic AST
```

## Usage

```typescript
import { parsePython } from "@coding-adventures/python-parser";

const ast = parsePython("x = 1 + 2");
console.log(ast.ruleName); // "program"
```

## Dependencies

- `@coding-adventures/python-lexer` -- tokenizes Python source code
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
