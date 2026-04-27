# grammar-tools (Kotlin)

Parser and validator for `.tokens` and `.grammar` file formats.

## What it does

- `parseTokenGrammar()` — Parses `.tokens` files into `TokenGrammar`
- `parseParserGrammar()` — Parses `.grammar` files into `ParserGrammar`
- `validateTokenGrammar()` — Lint pass for token grammars
- `validateParserGrammar()` — Lint pass for parser grammars
- `crossValidate()` — Checks consistency between token and parser grammars

## Layer

TE (text/language layer) — foundational infrastructure for lexer/parser generation.
