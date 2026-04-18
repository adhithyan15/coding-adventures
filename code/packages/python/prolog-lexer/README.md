# prolog-lexer

`prolog-lexer` tokenizes Prolog source code using the shared grammar-driven
lexer infrastructure.

The package recognizes the lexical structure needed for the first Prolog
frontend pass:

- atoms and quoted atoms
- variables and the anonymous variable `_`
- integers and floats
- strings
- queries (`?-`) and rules (`:-`)
- list punctuation like `[`, `]`, and `|`

## Quick Start

```python
from prolog_lexer import tokenize_prolog

tokens = tokenize_prolog("ancestor(X, Y) :- parent(X, Y).\n")
```

## Dependencies

- `grammar-tools`
- `lexer`

## Development

```bash
bash BUILD
```
