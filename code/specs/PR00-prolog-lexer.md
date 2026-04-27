# PR00 — Prolog Lexer

## Overview

This package tokenizes Prolog source code into a flat stream of tokens using
the shared grammar-driven lexer infrastructure.

It is the first frontend package built on top of the new logic-programming
foundation (`symbol-core` and `logic-core`). Its job is intentionally narrow:

- read Prolog source text
- recognize lexical units such as atoms, variables, numbers, punctuation, and
  clause separators
- discard trivia like whitespace and `%` line comments
- produce source-positioned tokens for the parser layer

This package does **not** interpret or execute Prolog. It does not know about
unification, clause databases, or search. It is purely the lexical front door.

## Why Start With A Lexer?

The previous PR established the semantic core:

- symbols
- logic terms
- substitutions
- unification
- goals
- search

The next step is to let source code enter that stack.

A Prolog parser cannot work directly on raw text because raw text is ambiguous
at the character level. The parser needs to see structured units like:

```prolog
ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).
```

as:

```text
ATOM("ancestor")
LPAREN
VARIABLE("X")
COMMA
VARIABLE("Y")
RPAREN
RULE(":-")
ATOM("parent")
...
DOT
EOF
```

That is the lexer's job.

## Scope

The first version supports the lexical subset needed for the first parser pass:

- lowercase atoms like `parent`, `likes`, `foo_bar`
- quoted atoms like `'Hello world'`
- variables like `X`, `Person`, `_Tmp`
- the anonymous variable `_`
- integers and decimal floats
- double-quoted strings
- punctuation for terms, rules, queries, and lists
- common symbolic operator-atoms like `=`, `\\=`, `=<`, `>=`, `+`, `-`, `*`
- `%` line comments

This version does **not** yet aim for full ISO Prolog lexical coverage. In
particular, it does not promise:

- character code constants
- full block-comment support
- every implementation-specific operator family
- every radix notation
- every Unicode identifier rule

Those can be added later without invalidating the basic package design.

## Layer Position

```
source text
    ↓
PR00 prolog-lexer        ← this package
    ↓
future prolog-parser
    ↓
lowering to logic-core terms and rules
```

## The Core Lexical Distinction

Prolog's most important lexical distinction is:

- **atoms** start lowercase or are quoted
- **variables** start uppercase or underscore

Examples:

```text
parent      → ATOM
homer       → ATOM
'Hello'     → ATOM
X           → VARIABLE
Person      → VARIABLE
_Tmp        → VARIABLE
_           → ANON_VAR
```

This is why a lexer is so useful here. The parser should not need to decide
whether `X` is a variable or atom by inspecting characters manually.

## Token Set

The initial token set is:

| Token | Example | Meaning |
|---|---|---|
| `ATOM` | `parent`, `'hello world'`, `\\=` | atom or symbolic operator atom |
| `VARIABLE` | `X`, `Person`, `_Tmp` | named variable |
| `ANON_VAR` | `_` | anonymous variable |
| `INTEGER` | `42` | integer literal |
| `FLOAT` | `3.14` | floating-point literal |
| `STRING` | `"hello"` | double-quoted string |
| `QUERY` | `?-` | top-level query introducer |
| `RULE` | `:-` | head/body separator |
| `DCG` | `-->` | grammar-rule separator (recognized early for future work) |
| `LPAREN` | `(` | term arguments start |
| `RPAREN` | `)` | term arguments end |
| `LBRACKET` | `[` | list start |
| `RBRACKET` | `]` | list end |
| `BAR` | `|` | list tail separator |
| `COMMA` | `,` | argument / goal separator |
| `SEMICOLON` | `;` | disjunction separator |
| `CUT` | `!` | cut |
| `DOT` | `.` | clause terminator |
| `EOF` | — | end of input |

## Grammar File

The package will be a thin wrapper over a new grammar file:

- `code/grammars/prolog.tokens`

That file defines the lexical patterns in priority order.

Priority matters. For example:

- `?-` must be recognized before a symbolic atom pattern that might also match
  `?`
- `:-` must be recognized before `:` and `-`-like symbolic atoms
- `-->` must be recognized before `--` or `-`
- `.` as a clause terminator must win over a general symbolic-atom rule

## Public API

The package should provide the same style of API as the other grammar-driven
lexers in the repo:

```python
from prolog_lexer import create_prolog_lexer, tokenize_prolog

lexer = create_prolog_lexer("parent(homer, bart).\\n")
tokens = lexer.tokenize()

tokens = tokenize_prolog("?- ancestor(homer, X).\\n")
```

Recommended functions:

```python
def create_prolog_lexer(source: str) -> GrammarLexer: ...
def tokenize_prolog(source: str) -> list[Token]: ...
```

## Examples

### Fact

```prolog
parent(homer, bart).
```

Token stream:

```text
ATOM("parent")
LPAREN
ATOM("homer")
COMMA
ATOM("bart")
RPAREN
DOT
EOF
```

### Rule

```prolog
ancestor(X, Y) :- parent(X, Y).
```

Token stream:

```text
ATOM("ancestor")
LPAREN
VARIABLE("X")
COMMA
VARIABLE("Y")
RPAREN
RULE
ATOM("parent")
LPAREN
VARIABLE("X")
COMMA
VARIABLE("Y")
RPAREN
DOT
EOF
```

### Query

```prolog
?- member(X, [a, b | T]).
```

Token stream:

```text
QUERY
ATOM("member")
LPAREN
VARIABLE("X")
COMMA
LBRACKET
ATOM("a")
COMMA
ATOM("b")
BAR
VARIABLE("T")
RBRACKET
RPAREN
DOT
EOF
```

## Error Model

The package should reuse the shared lexer error model:

- invalid / unmatched character sequences raise `LexerError`
- malformed quoted forms that do not match any token pattern also raise
  `LexerError`

The lexer should fail early and precisely rather than inventing fallback tokens.

## Test Strategy

Required tests:

- simple fact tokenization
- rule tokenization with `:-`
- query tokenization with `?-`
- lowercase atoms vs uppercase variables
- `_` vs `_Tmp`
- quoted atoms remain `ATOM`
- integers and floats tokenize distinctly
- strings tokenize as `STRING`
- list punctuation `[`, `]`, `|`
- `%` line comments are skipped
- source positions for a multi-line example
- factory helper `create_prolog_lexer()` works

## Future Extensions

- block comments
- full symbolic-operator coverage
- character-code literals
- implementation-specific dialect toggles
- block comment / nested comment dialect support
- lexical modes for quoted backtick strings if we ever target specific Prologs

## Package Location

The first implementation should live in:

- `code/packages/python/prolog-lexer`

The parser layer will come next, but the lexer should already be useful on its
own for:

- source highlighting
- token inspection in tests
- early syntax experiments for the upcoming parser
