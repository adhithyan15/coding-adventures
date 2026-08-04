# coding-adventures-scilab-lexer

Scilab tokenizer backed by `code/grammars/scilab/scilab.tokens`, compiled to
Rust and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Where this fits

This is the first crate of Scilab's frontend — MA-10b from
[`MA10-scilab-language.md`](../../../specs/MA10-scilab-language.md), the spec
that first establishes *why* Scilab needs its own frontend at all rather than
a thin MATLAB-rewrite shim the way `octave-runtime` is (MA10 §1). The crate
layout mirrors MA10 §6's rollout: `scilab-lexer` (this crate, MA-10b) →
`scilab-parser` (MA-10c) → `scilab-runtime`/`scilab-repl` (MA-10d) →
`scilab-to-semantic-ir` (MA-10e).

`scilab.tokens` is **forked from** `code/grammars/matlab/matlab.tokens` at
the grammar-**source** level (copied, then diverged) — this crate does
**not** depend on `matlab-lexer` at build time (MA10 §5). The grammar *shape*
(matrix literals, ranges, the operator cascade) is a legitimate MATLAB-family
inheritance; the *language* is not, which is why the fork happens at the
text level rather than the Rust-crate level.

## Scope

Covers the MA10 §4-scoped first cut: dense-matrix arithmetic (`+ - * / \ ^`,
elementwise `.* ./ .\ .^`, transpose `'`/`.'`), matrix literals and ranges
(`[1 2 3]`, `[1;2;3]`, `a:b`, `a:step:b`), comparisons (`== ~= <> < <= > >=`
— both not-equal spellings), logical operators (`& | ~ && ||`), `$`-based
last-index indexing, assignment, `if/elseif/else/end` and
`select/case/else/end` (with the optional `then` linker), `while/end` and
`for/end` (with the optional `do` linker), `break`/`continue`,
`function ... endfunction`, `//`/`/* */` comments, the eight `%`-prefixed
special constants, and single-/double-quoted strings.

## Five things that are genuinely NOT MATLAB (MA10 §3)

1. **Comments are `//` and `/* ... */`, not `%`/`%{ %}`.** Unlike MATLAB's
   `%{`/`%}` (which `matlab-lexer`'s own `strip_block_comments` enforces must
   sit alone on their line), Scilab's `/* ... */` may appear **inline**,
   sharing a line with real code on both sides — no such restriction exists
   here, so this crate needs no dedicated block-comment-stripping pre-pass at
   all; `BLOCK_COMMENT` is an ordinary `skip:` regex in `scilab.tokens`.
2. **`'...'` and `"..."` are the SAME token type (`STRING`)**, unlike
   MATLAB's CHARARRAY-vs-STRING(-scalar) split. The `'`/`.'`
   transpose-vs-string-open ambiguity still needs MA01 §3's context-hook
   *strategy* (reimplemented independently here, not shared code — see
   [`protect_quotes`](src/lib.rs)), but `"` has no such ambiguity and is a
   plain regex rule, same shape as MATLAB's own `DQ_STRING` — except that,
   unlike MATLAB, `DQ_STRING` is deliberately left un-aliased at the grammar
   level so this crate's own `collapse_dq_string_escapes` post-hook can
   actually collapse the doubled `""` escape (the shared `GrammarLexer`
   engine strips outer quotes automatically but does not know about that
   convention on its own).
3. **`PERCENT_CONST`** — a closed, fixed eight-word vocabulary
   (`%pi %e %i %inf %nan %eps %t %f`) with no MATLAB analogue at all. Since
   Scilab has no `%`-comment (the *opposite* of MATLAB, where `%` always
   means "comment starts here"), there is no conflict to guard against.
4. **`$`** — a single, unambiguous last-index token. Unlike MATLAB's own
   `end` (both block-terminator keyword *and* last-index sentinel,
   disambiguated only by the parser), `$` never needs context sensitivity.
5. **`<>`** — a second not-equal spelling alongside `~=`, both valid. Kept as
   its own distinct token (`NE_ALT`), not aliased to `NE` — the same
   deferral discipline `maple.tokens` documents for its own `;`/`:` pair
   ("the parser, not this lexer, is where the two spellings collapse onto
   one production").

## One deliberate omission: no `AT` (`@`) token

MATLAB's `@` (function handles) is out of this cut's scope (MA10 §4 never
lists function handles); Scilab's own deprecated legacy `@`-for-`~` spelling
is explicitly deferred too (MA10 §1 finding 6). Since neither meaning is in
scope, `@` is simply absent from `scilab.tokens` — it falls through to an
honest lex error rather than silently inheriting either meaning.

## What is deliberately NOT here (MA10 §4's deferred list)

The Kronecker trigraphs `.*.`/`./.`/`.\.`; the `end`-as-last-index
convergence (this cut's `end` is only ever the generic block-closer); the
deprecated legacy `**` spelling for `^` (so `a ** b` lexes as two bare `STAR`
tokens, mirroring `maple.tokens`'s own documented non-treatment); `switch`/
`otherwise`/`return`/`global`/`persistent`/`try`/`catch` (Scilab has no
`switch`/`otherwise` at all — its own construct is `select`/`case`/`else`,
MA10 §1 finding 4 — and the rest are simply outside this cut's in-scope
surface); and the general `%name` sigil-dispatch mechanism beyond the fixed
eight-word `PERCENT_CONST` vocabulary. Each is a simple omission — no
special-cased rejection logic — so any of these constructs fails honestly at
parse time rather than being silently misinterpreted (MA10 §4, citing MA06
§4's "absence, not special-cased exclusion" discipline).

## `endfunction` is its own keyword, distinct from generic `end`

Real Scilab's historical/still-preferred function terminator is
`endfunction` (MA10 §1 finding 7) — kept as its own `KEYWORD` value here,
never conflated with the generic block-closer `end` that `if`/`while`/
`for`/`select` all reduce to, since `scilab-parser` (MA-10c) needs that
distinction as a separate grammar production.

## Usage

```rust
use coding_adventures_scilab_lexer::tokenize_scilab;

let tokens = tokenize_scilab("x = %pi * 2\ny = x'\n");
```

`tokenize_scilab` panics on a malformed source string; use
`create_scilab_lexer` directly if you need the `Result`-returning
`GrammarLexer::tokenize` instead, or the crate-level `try_tokenize_scilab`
convenience wrapper.
