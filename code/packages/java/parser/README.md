# parser (Java)

Generic grammar-driven parser infrastructure for the coding-adventures project.

## What it does

- **ASTNode** — Generic AST node with rule name, children (nodes or tokens), and position tracking
- **GrammarParser** — Recursive descent parser driven by a ParserGrammar from a `.grammar` file
- Packrat memoization for efficient parsing
- Supports all EBNF constructs: sequence, alternation, repetition, optional, lookaheads, separated repetition

## Usage

```java
import com.codingadventures.parser.*;
import com.codingadventures.grammartools.*;
import com.codingadventures.lexer.*;

ParserGrammar grammar = ParserGrammarParser.parse(grammarFileContent);
GrammarParser parser = new GrammarParser(grammar);
ASTNode ast = parser.parse(tokens);
```

## Layer

TE (text/language layer) — depends on grammar-tools and lexer.
