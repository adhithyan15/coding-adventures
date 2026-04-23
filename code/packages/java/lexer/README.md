# lexer (Java)

Generic lexer/tokenizer infrastructure for the coding-adventures project.

## What it does

- **Token** — Immutable token with type, value, line, column, and flags
- **GrammarLexer** — Tokenizes source code using a TokenGrammar from a `.tokens` file
- First-match-wins pattern matching with priority ordering
- Keyword promotion, type aliases, reserved keyword detection
- Context-sensitive keyword flags, newline-preceded flags
- Error recovery patterns for graceful degradation

## Usage

```java
import com.codingadventures.lexer.*;
import com.codingadventures.grammartools.*;

TokenGrammar grammar = TokenGrammarParser.parse(tokensFileContent);
GrammarLexer lexer = new GrammarLexer(grammar);
List<Token> tokens = lexer.tokenize("1 + 2");
```

## Layer

TE (text/language layer) — depends on grammar-tools.
