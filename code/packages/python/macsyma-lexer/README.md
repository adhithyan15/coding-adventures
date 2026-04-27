# macsyma-lexer

A thin wrapper around the repo's generic grammar-driven `GrammarLexer`
that tokenizes MACSYMA/Maxima source text.

## What this package is

This package exists almost entirely to route `macsyma.tokens` into the
shared `GrammarLexer` engine. The real work — the regex engine, the
DFA, the keyword reclassification — lives in the `lexer` package. This
wrapper just:

1. Locates `code/grammars/macsyma/macsyma.tokens` on disk.
2. Parses it into a `TokenGrammar`.
3. Hands that grammar to `GrammarLexer(source, grammar)`.

Everything else is shared infrastructure.

## Usage

```python
from macsyma_lexer import tokenize_macsyma

tokens = tokenize_macsyma("f(x) := x^2 + 1; diff(f(x), x);")
for tok in tokens:
    print(tok.type, tok.value)
```

## MACSYMA lexical quirks

- `:=` is function definition (one token), distinct from `:`
  (assignment) and `=` (symbolic equation).
- `#` is not-equal — a historical MACSYMA choice that confuses
  readers used to `/=` or `!=`.
- `%pi`, `%e`, `%i` are single `NAME` tokens — the `%` prefix marks
  system-defined constants.
- Statements end with `;` (display result) or `$` (suppress result).
  Both are preserved as distinct tokens so downstream layers can
  choose output behavior.
- Comments are C-style `/* ... */`.

## Dependencies

- `coding-adventures-grammar-tools` — parses `.tokens` files.
- `coding-adventures-lexer` — the generic grammar-driven lexer engine.
- (transitive) `coding-adventures-directed-graph`,
  `coding-adventures-state-machine`.
