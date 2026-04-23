# swi-prolog-parser

`swi-prolog-parser` parses SWI-Prolog source using its own lexer package and
grammar file:

```text
code/packages/python/swi-prolog-lexer
code/grammars/prolog/swi.grammar
```

The first slice supports ordinary facts, rules, queries, SWI comments,
backquoted strings, and top-level `:- ... .` directives. Directives are exposed
as parsed metadata and are not executed yet.

## Quick Start

```python
from swi_prolog_parser import parse_swi_source

parsed = parse_swi_source(
    """
    :- initialization(main).
    parent(homer, bart).
    ?- parent(homer, Who).
    """,
)

assert len(parsed.directives) == 1
```
