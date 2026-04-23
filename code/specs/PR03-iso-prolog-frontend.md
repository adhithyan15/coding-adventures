# PR03 - ISO/Core Prolog Frontend

## Overview

PR02 identified dialect-specific lexer and parser layers as the first practical
step toward supporting the Prolog dialect ecosystem. PR03 implements the first
dialect slice: ISO/Core Prolog.

The design rule is simple:

```text
each dialect gets its own token grammar, parser grammar, lexer package, and
parser package
```

Shared lowering is allowed when the grammar AST is compatible with the current
engine semantics, but dialect frontends should not all live inside one large
package and should not rely on one shared grammar file forever.

## Scope

Add dedicated ISO/Core grammar files:

```text
code/grammars/prolog/iso.tokens
code/grammars/prolog/iso.grammar
```

Add dedicated ISO/Core packages:

```text
code/packages/python/iso-prolog-lexer
code/packages/python/iso-prolog-parser
```

The first ISO grammar files intentionally start as an explicit fork of the
current Prolog subset. That gives us a stable place to add ISO-specific
operators, directives, flags, errors, streams, DCGs, and standard predicates
without adding conditionals to the generic `prolog-lexer` and `prolog-parser`
packages.

## Public API

### `iso-prolog-lexer`

```python
create_iso_prolog_lexer(source: str) -> GrammarLexer
tokenize_iso_prolog(source: str) -> list[Token]
```

### `iso-prolog-parser`

```python
create_iso_prolog_parser(source: str) -> GrammarParser
parse_iso_ast(source: str) -> ASTNode
parse_iso_source(source: str) -> ParsedSource
parse_iso_program(source: str) -> Program
parse_iso_query(source: str) -> ParsedQuery
```

`iso-prolog-parser` reuses the generic `prolog-parser.lower_ast(...)` semantic
lowering function. This keeps parser ownership separate while avoiding a second
copy of the runtime lowering logic.

## Generic Parser Support

`prolog-parser` exports:

```python
lower_ast(ast: ASTNode) -> ParsedSource
```

Dialect parser packages can use this only when their grammar AST remains
compatible with the current lowerer. Once a dialect grows syntax that needs
different semantics, that dialect package should add its own expansion/lowering
layer before calling the shared engine.

## Non-Goals

This PR does not implement:

- full ISO operators
- `op/3`
- directives
- DCG expansion
- modules
- streams and I/O
- exceptions and ISO error terms
- complete ISO standard predicates

Those belong in follow-up ISO-specific batches now that the package and grammar
boundaries exist.

## Follow-Up Pattern

Future dialects should follow the same shape:

```text
code/grammars/prolog/swi.tokens
code/grammars/prolog/swi.grammar
code/packages/python/swi-prolog-lexer
code/packages/python/swi-prolog-parser
```

This keeps SWI, GNU, Scryer, Trealla, XSB, Ciao, ECLiPSe, Tau, and other
dialects independently evolvable.
