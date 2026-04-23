# iso-prolog-lexer

`iso-prolog-lexer` tokenizes ISO/Core Prolog source using its own grammar file:

```text
code/grammars/prolog/iso.tokens
```

This package is separate from `prolog-lexer` on purpose. Every Prolog dialect
should get its own lexer package and grammar file so dialect-specific lexical
behavior does not turn into a pile of conditionals in one package.

## Quick Start

```python
from iso_prolog_lexer import tokenize_iso_prolog

tokens = tokenize_iso_prolog("ancestor(X, Y) :- parent(X, Y).\n")
```
