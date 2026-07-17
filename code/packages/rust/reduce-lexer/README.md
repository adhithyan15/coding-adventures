# coding-adventures-reduce-lexer

REDUCE tokenizer backed by `code/grammars/reduce/reduce.tokens`, compiled to
Rust and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Scope

Covers the R-1-scoped subset fixed by
[MA08 §3](../../../specs/MA08-reduce-language.md): ordinary-parenthesis
function/procedure application (`df(x, y)`, `h(l, m)`), the single `:=`
assignment operator, `=` as the *equation* operator (never assignment),
comparison (`< > <= >= neq`), the word-spelled logical keywords
`and`/`or`/`not`, the `if`/`then`/`else` conditional keywords, `{a, b, c}`
curly-brace list literals, the `.` cons operator, `<< ... >>` group-
statement delimiters, both statement terminators `;` and `$`, and
arithmetic `+ - * / ^ **`.

## REDUCE's keywords are lowercase — the mirror image of Derive's

Unlike [`derive-lexer`](../derive-lexer) (whose boolean-algebra keywords
`AND`/`OR`/`NOT` are UPPERCASE, matching Derive's own conventionally-
uppercase built-in names), REDUCE's real surface spells its word operators
and statement keywords in lowercase — `and`, `or`, `not`, `neq`, `if`,
`then`, `else` (manual §2.7/§5.3). `reduce.tokens`'s `keywords:` block
lists the lowercase spellings and matches case-sensitively, so `AND`/`IF`/
etc. in uppercase lex as ordinary `NAME`s here — exactly the opposite of
`derive-lexer`'s case rule.

## `^` and `**` are the same operator, kept as distinct tokens

REDUCE's manual states `^` and `**` are literally the same power operator
(one precedence tier). Rather than collapsing them into one token type,
this crate keeps `CARET`/`POW` distinct — mirroring how `macsyma-lexer`
keeps its own two statement terminators (`;`/`$`) as distinct `SEMI`/
`DOLLAR` tokens even though "the parser treats them identically." The
*parser* (R-3), not this lexer, is where the two power spellings collapse
onto one production.

## No significant newlines — the same statement model as Macsyma

Unlike `derive-lexer`/`wolfram-lexer` (whose worksheet-style grammars have
a significant top-level `NEWLINE`, needing a bracket-interior newline-
dropping post-tokenize hook), REDUCE statements are separated by `;` or
`$` (manual §5.1) — a newline is never significant. This mirrors
`macsyma-lexer`'s identical `;`/`$`-terminated statement model exactly:
`reduce.tokens` emits no `NEWLINE` token at all (every newline is ordinary
skipped whitespace), so this crate needs nothing beyond a bare
`GrammarLexer::new` — no post-tokenize hook, unlike `derive-lexer`.

## List literals use curly braces, not square brackets

`{a, b, c}` (manual §4.1) is REDUCE's list literal — **not** Derive's
`[a,b,c]` vector-literal syntax, and not APL/J/MATLAB array syntax either.
This subset's array-declaration syntax (`array a(10,10)`) is out of scope
(MA08 §4), so a subscripted read like `a(5)` lexes through the ordinary
`LPAREN`/`RPAREN` call syntax, never through a bracket.

## Usage

```rust
use coding_adventures_reduce_lexer::tokenize_reduce;

let tokens = tokenize_reduce("h(l, m) := l + m$\nh(1, 2)$");
```

`tokenize_reduce` panics on a malformed source string; use
`create_reduce_lexer` directly if you need the `Result`-returning
`GrammarLexer::tokenize` instead, or the crate-level `try_tokenize_reduce`
convenience wrapper.

## Where this fits

`reduce-lexer` is the first of REDUCE's frontend crates (R-2); the sibling
`reduce-parser` crate (R-3) will consume this crate's token stream against
`code/grammars/reduce/reduce.grammar` to build the `GrammarASTNode` CST a
future `reduce-runtime` (R-4) will lower into `symbolic_ir::IRNode` and
evaluate with `symbolic_vm::VM`'s shared `SymbolicBackend`, reused
unchanged.
