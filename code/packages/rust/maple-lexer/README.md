# coding-adventures-maple-lexer

Maple tokenizer backed by `code/grammars/maple/maple.tokens`, compiled to
Rust and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Scope

Covers the MP-1-scoped subset fixed by
[MA09 §3](../../../specs/MA09-maple-language.md): ordinary-parenthesis
function/procedure application (`f(x, y)`), the single `:=` assignment
operator, the arrow/functional operator `->` (Maple's real general-purpose
function-definition spelling, `f := (x, y) -> e`), `=` as the *equation*
operator (never assignment), comparison (`< > <= >= <>`), the word-spelled
logical keywords `and`/`or`/`not`, the `if`/`then`/`elif`/`else`/`end`/`fi`
conditional keywords, the `true`/`false` boolean literals, `[a, b, c]`
square-bracket **list** literals and `{a, b, c}` curly-brace **set**
literals (two distinct aggregate types — see below), both statement
terminators `;` and `:`, and arithmetic `+ - * / ^` (`^` only — no `**`
synonym).

## Why Maple needs a lexer of its own, not "REDUCE again"

[MA09 §1](../../../specs/MA09-maple-language.md) confirms Maple is not a
close-enough cousin of REDUCE to reuse `reduce-lexer`'s grammar wholesale,
despite sharing `:=` assignment and `and`/`or`/`not` keywords:

- **Two distinct bracket-delimited aggregate types, not REDUCE's/Derive's
  one.** `[a, b, c]` is Maple's ordered, duplicates-preserved **list**
  (Programming Guide §4.3) — the *opposite* bracket choice from Derive's
  `[a,b,c]` *vector* literal (same spelling, different meaning). `{a, b,
  c}` is Maple's unordered, duplicates-removed **set** — the same bracket
  REDUCE uses for its own *list* literal (same spelling, different
  meaning again). This lexer keeps `LBRACKET`/`RBRACKET` and
  `LBRACE`/`RBRACE` as four distinct token types; which aggregate a
  bracket pair denotes is carried entirely by which bracket was used, not
  by any lexer-level disambiguation.
- **A real `ARROW` (`->`) token neither REDUCE's nor Derive's `.tokens`
  file needs.** Real Maple's general-purpose function definition is the
  arrow/functional operator, `f := (x, y) -> e` (Help page
  `operators/functional`) — `f(x) := expr` in real Maple is instead a
  narrower **remember-table** patch onto an *already-existing* procedure
  (the `remember` Help page), not a general definition. This lexer does
  not (and cannot, at the lexing stage) tell those two `:=` uses apart —
  it emits the same `ASSIGN` token either way; the arrow-operator-vs-
  remember-table distinction is invisible until MP-3/MP-4 see the whole
  statement shape.

## Maple's keywords are lowercase — like REDUCE, unlike Derive

Like [`reduce-lexer`](../reduce-lexer) (whose logical/statement keywords
are lowercase `and`/`or`/`not`/`if`/`then`/`else`), and unlike
[`derive-lexer`](../derive-lexer) (whose boolean-algebra keywords
`AND`/`OR`/`NOT` are UPPERCASE, matching Derive's own conventionally
uppercase built-in names), Maple's real surface spells its word operators
and statement keywords in lowercase: `and`, `or`, `not`, `if`, `then`,
`elif`, `else`, `end`, `fi`, `true`, `false` (MA09 §3, citing Programming
Guide §3.10/§5.6 and the `if`/`type/truefalseFAIL` Help pages).
`maple.tokens`'s `keywords:` block lists the lowercase spellings and
matches case-sensitively, so `AND`/`IF`/etc. in uppercase lex as ordinary
`NAME`s here.

## `^` only — no `**` synonym

Unlike REDUCE's manual (which states `^` and `**` are literally the same
power operator, one precedence tier, kept as REDUCE's own distinct
`CARET`/`POW` tokens), real Maple documents no `**` synonym for `^`
(MA09 §3's own note on the `arithop`/precedence pages). This grammar has
no `POW` token at all — `a ** b` lexes as two separate `TIMES` tokens
(`a`, `*`, `*`, `b`), never a single power operator, since inventing one
"for completeness" would add a Maple operator that does not exist.

## No significant newlines — the same statement model as REDUCE/Macsyma

Unlike `derive-lexer`/`wolfram-lexer` (whose worksheet-style grammars have
a significant top-level `NEWLINE`, needing a bracket-interior newline-
dropping post-tokenize hook), Maple statements are separated by `;` or `:`
(Programming Guide §5.3 "Statement Separators" — `;` displays the result,
`:` suppresses it) — a newline is never significant, and real Maple's own
interactive session has no `#n:`/`In[n]:=` numbered-worksheet-prompt
convention either (MA09 §5). This mirrors `reduce-lexer`'s/
`macsyma-lexer`'s identical `;`/`$`(or `:`)-terminated statement model
exactly: `maple.tokens` emits no `NEWLINE` token at all (every newline is
ordinary skipped whitespace), so this crate needs nothing beyond a bare
`GrammarLexer::new` — no post-tokenize hook, unlike `derive-lexer`.

`;` and `:` are kept as two distinct token types (`SEMI`/`COLON`) even
though a later parser may treat both as statement terminators — the same
discipline `reduce-lexer` documents for its own `SEMI`/`DOLLAR` split:
"the parser, not this lexer, is where the two spellings collapse onto one
production."

## `end`/`fi` — two ways to close an `if`, one token shape each

Real Maple closes an `if` statement with either `end if` (two keywords in
a row — `end`, then the already-existing `if` keyword) or the standalone
`fi` keyword ("if" reversed — the `if` Help page confirms `fi` is short
for `end if`; MA09 §3). This lexer emits `end` and `if` as two independent
`KEYWORD` tokens; it does not special-case the `end if` sequence into one
token. Composing `end` + `if` into one production (vs. accepting bare
`fi` as the alternative) is left entirely to the parser (MP-3).

## Usage

```rust
use coding_adventures_maple_lexer::tokenize_maple;

let tokens = tokenize_maple("f := (x, y) -> x + y;\nf(1, 2);");
```

`tokenize_maple` panics on a malformed source string; use
`create_maple_lexer` directly if you need the `Result`-returning
`GrammarLexer::tokenize` instead, or the crate-level `try_tokenize_maple`
convenience wrapper.

## Where this fits

`maple-lexer` is the first of Maple's frontend crates (MP-2); the sibling
`maple-parser` crate (MP-3) will consume this crate's token stream against
`code/grammars/maple/maple.grammar` to build the `GrammarASTNode` CST a
future `maple-runtime` (MP-4) will lower into `symbolic_ir::IRNode` and
evaluate with `symbolic_vm::VM`'s shared `SymbolicBackend`, reused
unchanged.
