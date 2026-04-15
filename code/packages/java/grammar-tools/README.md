# grammar-tools (Java)

Parser and validator for `.tokens` and `.grammar` file formats — the declarative language specifications used throughout the coding-adventures project.

## What it does

- **TokenGrammarParser** — Parses `.tokens` files into a `TokenGrammar` data structure
- **ParserGrammarParser** — Parses `.grammar` files into a `ParserGrammar` data structure
- **TokenGrammarValidator** — Lint pass for token grammars (duplicates, invalid regex, naming conventions)
- **ParserGrammarValidator** — Lint pass for parser grammars (undefined refs, unreachable rules)
- **CrossValidator** — Checks consistency between a `.tokens` and `.grammar` file pair

## Usage

```java
import com.codingadventures.grammartools.*;

// Parse a .tokens file
TokenGrammar tokens = TokenGrammarParser.parse(tokensFileContent);
List<String> tokenIssues = TokenGrammarValidator.validate(tokens);

// Parse a .grammar file
ParserGrammar grammar = ParserGrammarParser.parse(grammarFileContent);
List<String> grammarIssues = ParserGrammarValidator.validate(grammar, tokens.tokenNames());

// Cross-validate
List<String> crossIssues = CrossValidator.crossValidate(tokens, grammar);
```

## Layer

TE (text/language layer) — foundational infrastructure for lexer/parser generation.
