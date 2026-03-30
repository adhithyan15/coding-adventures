# CodingAdventures::LispLexer

Perl module — grammar-driven Lisp/Scheme tokenizer.

## What it does

Converts Lisp/Scheme source text into a flat arrayref of token hashrefs:

```perl
use CodingAdventures::LispLexer;

my $tokens = CodingAdventures::LispLexer->tokenize('(define x 42)');
# [
#   { type => 'LPAREN', value => '(',      line => 1, col => 1 },
#   { type => 'SYMBOL', value => 'define', line => 1, col => 2 },
#   { type => 'SYMBOL', value => 'x',      line => 1, col => 9 },
#   { type => 'NUMBER', value => '42',     line => 1, col => 11 },
#   { type => 'RPAREN', value => ')',      line => 1, col => 13 },
#   { type => 'EOF',    value => '',       line => 1, col => 14 },
# ]
```

Whitespace and `;` line comments are silently consumed.

## Token types

| Type   | Matches                                        | Example        |
|--------|------------------------------------------------|----------------|
| NUMBER | `/-?[0-9]+/`                                   | `42`, `-7`     |
| SYMBOL | `[a-zA-Z_+\-*\/=<>!?&][a-zA-Z0-9_+\-*\/=<>!?&]*` | `define`, `+`, `null?` |
| STRING | `/"([^"\\]|\\.)*"/`                            | `"hello"`      |
| LPAREN | `(`                                            | `(`            |
| RPAREN | `)`                                            | `)`            |
| QUOTE  | `'`                                            | `'`            |
| DOT    | `.`                                            | `.`            |
| EOF    | (sentinel, always last)                        | (end)          |

## Dependencies

- `CodingAdventures::GrammarTools` — parses `lisp.tokens`
- `CodingAdventures::Lexer` — lexer infrastructure

## How it fits in the stack

```
lisp.tokens  (grammar file in code/grammars/)
     ↓
LispLexer    (this module)
     ↓
LispParser   (builds ASTs from the token stream)
```
