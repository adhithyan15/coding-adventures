# PR04 - SWI-Prolog Frontend

## Overview

PR03 established the dialect rule: each Prolog dialect gets its own token
grammar, parser grammar, lexer package, and parser package. PR04 adds the next
dialect slice for SWI-Prolog.

SWI is a practical priority because it is one of the most widely used Prolog
systems. It also gives us early pressure on dialect-owned features such as
comments, directives, flags, modules, quasi quotations, and SWI-specific
built-ins without mixing that behavior into ISO/Core packages.

## Scope

Add dedicated SWI grammar files:

```text
code/grammars/prolog/swi.tokens
code/grammars/prolog/swi.grammar
```

Add dedicated SWI packages:

```text
code/packages/python/swi-prolog-lexer
code/packages/python/swi-prolog-parser
```

This first SWI slice supports:

- facts, rules, queries, lists, equality, disjunction, conjunction, and cut
- `%` line comments
- `/* ... */` block comments
- SWI-style backquoted strings as string tokens
- top-level `:- goal.` directives collected as parsed metadata

## Public API

### `swi-prolog-lexer`

```python
create_swi_prolog_lexer(source: str) -> GrammarLexer
tokenize_swi_prolog(source: str) -> list[Token]
```

### `swi-prolog-parser`

```python
create_swi_prolog_parser(source: str) -> GrammarParser
parse_swi_ast(source: str) -> ASTNode
parse_swi_source(source: str) -> ParsedSwiSource
parse_swi_program(source: str) -> Program
parse_swi_query(source: str) -> ParsedQuery
```

`ParsedSwiSource` mirrors the generic executable source shape and adds:

```python
directives: tuple[ParsedSwiDirective, ...]
```

Directives are parsed and preserved, but they are not executed yet. This lets
future batches add directive semantics incrementally.

## Non-Goals

This PR does not implement:

- SWI module semantics
- `use_module/1` or `use_module/2`
- `op/3` dynamic operator declarations
- DCG expansion
- quasi quotations
- dicts
- attributed variables
- constraint libraries
- foreign predicate loading
- directive execution

Those belong in SWI-specific follow-up batches now that the package and grammar
boundaries exist.
