# swi-prolog-lexer

`swi-prolog-lexer` tokenizes SWI-Prolog source using its own grammar file:

```text
code/grammars/prolog/swi.tokens
```

This package is separate from `iso-prolog-lexer` and `prolog-lexer` so SWI
lexical behavior can evolve independently.

It also keeps SWI-specific symbolic forms owned by the dialect, including
CLP(FD) range syntax such as `1..4`.

## Quick Start

```python
from swi_prolog_lexer import tokenize_swi_prolog

tokens = tokenize_swi_prolog(":- initialization(main).\n")
```
