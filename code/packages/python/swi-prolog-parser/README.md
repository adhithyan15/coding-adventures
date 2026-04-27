# swi-prolog-parser

`swi-prolog-parser` parses SWI-Prolog source using its own lexer package and
grammar file:

```text
code/packages/python/swi-prolog-lexer
code/grammars/prolog/swi.grammar
```

The first slice supports ordinary facts, rules, queries, SWI comments,
backquoted strings, and top-level `:- ... .` directives. Parsed sources expose
the directive list as metadata, and file-scoped `op/3` directives are applied
while parsing so later clauses use the updated operator table. DCG rules
(`-->`) are also expanded into ordinary executable clauses during parsing.

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
