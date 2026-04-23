# prolog-parser

`prolog-parser` parses a first executable Prolog syntax subset with
`code/grammars/prolog.grammar` and lowers it onto the existing `logic-engine`
runtime.

This package is intentionally not a second solver. It consumes tokens from
`prolog-lexer`, parses them with the shared grammar-driven `GrammarParser`,
builds `logic-engine` facts, rules, goals, terms, and variables, and lets the
already-tested backtracking engine execute the result.

## What It Supports

- facts: `parent(homer, bart).`
- rules: `ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).`
- top-level queries: `?- ancestor(homer, Who).`
- atoms, quoted atoms, variables, anonymous variables, integers, floats, strings
- compound terms and canonical Prolog lists, including `[Head | Tail]`
- conjunction with `,`
- disjunction with `;`
- grouped goals with parentheses
- cut with `!`
- unification and disequality goals: `X = homer`, `X \= marge`

Out of scope for this first parser pass: DCGs, full ISO operator precedence,
modules, directives, arithmetic syntax, and parser-level CLP(FD) syntax.

## Quick Start

```python
from logic_engine import solve_all
from prolog_parser import parse_source

parsed = parse_source(
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

## API

- `parse_ast(source)` returns the generic grammar AST produced from
  `code/grammars/prolog.grammar`.
- `lower_ast(ast)` lowers a compatible Prolog grammar AST into executable
  `logic-engine` objects. Dialect parser packages use this to share semantics
  while owning their own lexer/parser grammar files.
- `create_prolog_parser(source)` returns a configured grammar-driven parser.
- `parse_source(source)` returns a `ParsedSource` with an executable
  `logic-engine` `Program`, parsed clauses, and parsed top-level queries.
- `parse_program(source)` returns a `Program` and rejects query statements.
- `parse_query(source)` returns one `ParsedQuery` and rejects clauses.

Each `ParsedQuery` exposes a `variables` dictionary so callers can ask the
engine for the values of source-level variables by name.
