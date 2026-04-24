# iso-prolog-parser

`iso-prolog-parser` parses ISO/Core Prolog source using its own lexer package
and grammar file:

```text
code/packages/python/iso-prolog-lexer
code/grammars/prolog/iso.grammar
```

This package is separate from `prolog-parser` on purpose. Every Prolog dialect
should get its own parser package and grammar file, while sharing semantic
lowering where compatible.

Executable source parsing now accepts file-scoped directives such as
`:- op(500, yfx, ++).` and returns the final `parsed.operator_table` after
those directives have been applied. It also expands DCG rules (`-->`) into
ordinary executable clauses during source parsing.

## Quick Start

```python
from logic_engine import solve_all
from iso_prolog_parser import parse_iso_source

parsed = parse_iso_source(
    """
    parent(homer, bart).
    parent(bart, lisa).
    ancestor(X, Y) :- parent(X, Y).
    ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).
    ?- ancestor(homer, Who).
    """,
)

query = parsed.queries[0]
answers = solve_all(parsed.program, query.variables["Who"], query.goal)

assert [str(answer) for answer in answers] == ["bart", "lisa"]
```
