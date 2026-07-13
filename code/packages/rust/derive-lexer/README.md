# coding-adventures-derive-lexer

Derive tokenizer backed by `code/grammars/derive/derive.tokens`, compiled to
Rust and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Scope

Covers the D-1-scoped subset fixed by
[MA07 §3](../../../specs/MA07-derive-language.md): ordinary-parenthesis
function application (`DIF(u, x)`, not Wolfram's `f[x]`), the single `:=`
assign/define operator (used identically for `x := 5` and
`F(x) := x^2 + 1`; the D-3 parser, not this lexer, disambiguates variable
assignment from function definition by what shape precedes it), `=` as the
*equation* operator (never assignment), comparison (`<= < > >=`), the
boolean-algebra keywords `AND`/`OR`/`NOT`, `[a, b, c]`/`[a, b, c; d, e, f]`
vector/matrix literal delimiters, and arithmetic `+ - * / ^`.

## `:=` is genuinely one token doing two jobs

Unlike most languages, Derive spells variable assignment and function
definition with the exact same operator — `x := 5` and `F(x) := x^2 + 1`
both use `:=`. This crate does not attempt to distinguish them (there is
nothing for a *lexer* to distinguish — both are the identical two-character
`:=` token); the D-3 parser tells them apart by whether the thing to the
left of `:=` is a bare `NAME` or a `NAME(...)` call.

## Boolean keywords are case-sensitive reserved words, not conventionally-cased symbols

Derive's built-in function names (`SIN`, `DIF`, `INT`) are conventionally,
but not enforced, uppercase — an ordinary `NAME` either way, exactly like
Wolfram's `Sin` vs. `sin` distinction. `AND`/`OR`/`NOT`, however, are the
one place this subset's grammar (MA07 §3's own wording: "Derive's
boolean-algebra keywords, not symbols") promotes specific spellings to a
dedicated `KEYWORD` token type via `derive.tokens`'s `keywords:` block —
matched case-sensitively, so lowercase `and`/`or`/`not` lex as ordinary
`NAME`s, never as the keyword.

## No `{ }`, no `[[ ]]` — only `(`/`[` need bracket-depth tracking

Unlike Wolfram (which needs `(`, `[`, `{`, and the two-level `[[` part-sugar
opener all tracked for its bracket-interior-newline hook), Derive's surface
has only two bracket pairs: `( )` for grouping/application and `[ ]` for
vector/matrix literals. `drop_bracketed_newlines` here tracks just those
two.

## Usage

```rust
use coding_adventures_derive_lexer::tokenize_derive;

let tokens = tokenize_derive("F(x) := x^2 + 1\nF(3)\n");
```

`tokenize_derive` panics on a malformed source string; use
`create_derive_lexer` directly if you need the `Result`-returning
`GrammarLexer::tokenize` instead, or the crate-level `try_tokenize_derive`
convenience wrapper.

## Where this fits

`derive-lexer` is the first of Derive's frontend crates (D-2); the sibling
`derive-parser` crate (D-3) will consume this crate's token stream against
`code/grammars/derive/derive.grammar` to build the `GrammarASTNode` CST a
future `derive-runtime` (D-4) will lower into `symbolic_ir::IRNode` and
evaluate with `symbolic_vm::VM`.
