# coding-adventures-idl-lexer

IDL (Interactive Data Language) tokenizer backed by
`code/grammars/idl/idl.tokens`, compiled to Rust and statically linked into
the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Where this fits

This is the first crate of IDL's frontend — MA-12b from
[`MA12-idl-language.md`](../../../specs/MA12-idl-language.md), the spec that
fixes IDL's version-era decision (targeting IDL 5.0-and-later's
square-bracket-subscript language, MA12 §1), confirms `array-runtime` needs
no changes for IDL's value model (MA12 §2), and fixes the one genuinely new
grammar/evaluator problem this repo's array family has not seen before — a
keyword-argument calling convention layered over a procedure/function split
(MA12 §3) — entirely at the design level, before any lexer/parser/runtime
code lands. The crate layout mirrors MA12 §6's rollout: `idl-lexer` (this
crate, MA-12b) → `idl-parser` (MA-12c) → `idl-runtime`/`idl-repl` (MA-12d) →
`idl-to-semantic-ir` (MA-12e).

Unlike every other array-family frontend already in this repo (`apl-lexer`,
`j-lexer`, `q-lexer`, `scilab-lexer`), IDL's *surface* is an Algol/Fortran-
family imperative grammar — statements, `PRO`/`FUNCTION` definitions,
`IF`/`FOR`/`WHILE`/`REPEAT` blocks, an infix operator-precedence cascade with
word operators (`EQ`/`AND`/...) — closer in shape to this repo's
`algol-lexer`/`dartmouth-basic-lexer` than to any array-family lexer (MA12
§5). So `idl.tokens` is written to IDL's own shape rather than forked from
an array sibling.

## Scope

Covers the MA-12b-scoped lexical surface fixed by
[MA12 §6](../../../specs/MA12-idl-language.md#6-crate-layout-and-rollout-one-item--one-pr):
`;` line comments, `$` line continuation (a real token, not stripped — see
below), `&` statement separator, single- and double-quoted strings (unified
to one `STRING` token type, no escape mechanism), the word operators
`EQ NE LT LE GT GE` (comparison) and `AND OR NOT XOR` (logical), the two
matrix-product operators `#`/`##` (longest-match-first), `[`/`]` subscript
brackets, ordinary arithmetic/punctuation (`+ - * / ^ = , : ( )`), integer
and floating-point numeric literals, identifiers, and the closed
control-flow/definition keyword set `IF THEN ELSE ENDIF ENDELSE FOR DO
ENDFOR WHILE ENDWHILE REPEAT UNTIL ENDREP BREAK CONTINUE BEGIN END PRO
FUNCTION RETURN`.

This crate only tokenizes. There is no `idl-parser`/`idl.grammar` here (that
is a separate follow-on task, MA-12c) and no recursion-depth cap (that is a
parser-level concern for MA-12c, the same split `apl-lexer`/`j-lexer` vs.
`apl-parser`/`j-parser` already establish in this repo).

## The `/`-before-identifier question: resolved as a parser concern, not a lexer one

MA12 §3 item 3 documents IDL's `/KEYWORD` boolean shorthand (`PLOT, x,
/YLOG` means `PLOT, x, YLOG=1`), and asks whether the lexer can tell a
boolean-keyword `/` apart from an ordinary division `/`. Q (MA11) had an
analogous-looking problem — `/` as a comment-opener vs. the REDUCE adverb —
which `q-lexer` resolves with `GrammarLexer` pre/post-tokenize hooks keyed
on **whitespace adjacency** (is there a space immediately before this `/`?).

That strategy does **not** transfer to IDL, and this crate does not try to
force it to. MA12 §3 item 3 itself says the distinguishing signal here is
"grammatical position, not whitespace" — and checking that claim directly
confirms it: `PLOT, x, /YLOG` (boolean shorthand) and `x = a/YLOG` (ordinary
division) both have `/` glued to an identifier with **zero** intervening
whitespace, on **either** side. No adjacency test — no hook operating on
raw characters or a flat token list — can tell these two apart, because the
only fact that distinguishes them is whether the parser is currently inside
a call's argument-list production at that source position, which is
information only a parser (with its own production stack) has. So:

- `idl.tokens`/`idl-lexer` emit `SLASH` as one ordinary, unconditional
  division-operator token, in every position, with **no** pre/post-tokenize
  hook at all.
- The `/KEYWORD`-vs-division production belongs entirely to `idl-parser`
  (MA-12c), which is the first layer with enough context (an
  argument-list production) to make the call correctly.

This is also why `idl-lexer` needs **no custom lexer code whatsoever** —
unlike `q-lexer` (two hooks) and `scilab-lexer` (one hook, for `'`
transpose-vs-string), `idl.tokens` is entirely declarative and
`create_idl_lexer` installs nothing beyond the compiled grammar. IDL has no
transpose operator to collide with `'`, and `;` (IDL's comment marker) has
no other meaning anywhere in this cut's grammar the way Q's `/` does, so
even the comment-stripping is an ordinary `skip:` regex.

## Case-insensitivity

IDL is documented as not case sensitive for its language surface (keywords,
procedure/function/variable names), except for the contents of a quoted
string. `idl.tokens` sets `# @case_insensitive true` (no
`case_sensitive: false`) — the same combination `dot.tokens`,
`excel.tokens`, and `spice/berkeley.tokens` already use in this repo. That
combination makes **only** `keywords:`-block lookup case-insensitive
(`if`/`If`/`IF` all promote to `KEYWORD("IF")`), while ordinary `NAME`
tokens and `STRING` contents keep the **exact** case the source text used —
deliberately narrower than copying `dartmouth_basic.tokens`'s/
`cobol.tokens`'s own `case_sensitive: false` mechanism, which case-folds
every pattern's matched value, including plain identifiers. See
`code/grammars/idl/idl.tokens`'s own header comment for the full reasoning.

## Usage

```rust
use coding_adventures_idl_lexer::tokenize_idl;

let tokens = tokenize_idl("PRO GREET, name\n  PRINT, name\nEND");
```

`tokenize_idl` panics on a malformed source string; use `create_idl_lexer`
directly (or `try_tokenize_idl`) if you need the `Result`-returning form
instead.

## Where this fits

`idl-lexer` is the first of IDL's frontend crates
([MA-12b](../../../specs/MA12-idl-language.md#6-crate-layout-and-rollout-one-item--one-pr)),
following MA-12a's design spec. The sibling `idl-parser` crate (MA-12c) will
consume this crate's token stream against `code/grammars/idl/idl.grammar` —
including the one genuinely new grammar production this language needs, the
procedure-call statement and its keyword-argument argument-list (MA12 §3)
— to build the `GrammarASTNode` CST that a future `idl-runtime` (MA-12d)
will evaluate, alongside `idl-repl` and `idl-to-semantic-ir` (MA-12e), per
[HML00](../../../specs/HML00-historical-math-languages-roadmap.md) Wave 6.
