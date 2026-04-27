# PR01 - Prolog Parser Foundation

## Overview

PR00 gave the project a grammar-driven Prolog lexer. PR01 adds the first parser
layer above it: `code/grammars/prolog.grammar`, parsed by the shared generic
`GrammarParser`, plus a Python lowering layer that maps the resulting AST onto
the existing Python logic engine.

The goal is not full ISO Prolog yet. The goal is the first honest bridge:

```text
source text
    -> prolog-lexer tokens
    -> prolog.grammar AST
    -> prolog-parser clauses and queries
    -> logic-engine Program and GoalExpr values
    -> solve_all / solve_n
```

## Scope

The first parser pass supports the syntax needed to write and execute ordinary
small Prolog programs:

- facts
- rules
- top-level queries
- atoms and quoted atoms
- variables and anonymous variables
- integers, floats, and strings
- compound terms
- canonical list syntax, including `[Head | Tail]`
- conjunction with `,`
- disjunction with `;`
- grouped goals with parentheses
- cut with `!`
- unification with `=`
- disequality with `\=`

Every parsed clause and query is lowered to existing `logic-engine` objects. The
parser does not introduce a second runtime or a second unification model.

The grammar file is the syntax source of truth for this subset. Python code is
reserved for semantic lowering into `logic-engine` values and should not grow a
parallel hand-written syntax parser unless the shared grammar runtime cannot
express a specific construct.

## Public API

```python
parse_ast(source: str) -> ASTNode
create_prolog_parser(source: str) -> GrammarParser
parse_source(source: str) -> ParsedSource
parse_program(source: str) -> Program
parse_query(source: str) -> ParsedQuery
```

`ParsedSource` contains:

- an executable `Program`
- the parsed `Clause` objects
- the parsed top-level queries

`ParsedQuery` contains:

- the executable `GoalExpr`
- a `variables` dictionary mapping source variable names to `LogicVar` objects

## Variable Semantics

Named variables share identity only within one clause or one query.

```prolog
same(X, X).
```

Both `X` occurrences refer to the same logic variable.

Anonymous variables are fresh per occurrence:

```prolog
pair(_, _).
```

The two `_` occurrences do not refer to the same variable.

## Non-Goals

This foundation parser intentionally does not implement:

- DCG lowering
- modules
- directives
- consult/include semantics
- full ISO operator declarations
- arithmetic expression parsing
- parser-level CLP(FD) operators
- every Prolog quoting and escape edge case

Those can layer on after the syntax-to-engine bridge exists.

## Example

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
assert solve_all(parsed.program, query.variables["Who"], query.goal)
```

## Test Strategy

Tests should prove:

- facts execute through `logic-engine`
- recursive rules and parsed queries enumerate answers
- zero-arity facts and goals work
- named variables share identity within a statement
- anonymous variables are fresh per occurrence
- list syntax lowers to canonical logic lists
- grouped disjunction and cut preserve solver behavior
- malformed or unsupported syntax raises source-positioned parse errors
