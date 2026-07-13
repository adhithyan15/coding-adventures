# S Lexer

A grammar-driven lexer (tokenizer) for the historical
[S programming language](https://en.wikipedia.org/wiki/S_(programming_language))
created at Bell Labs by John Chambers, Rick Becker, and Allan Wilks (1976) —
the direct ancestor of R.

## What it does

Tokenizes S source text into a stream of typed tokens. It does not hand-write
tokenization rules: it loads the compiled `s.tokens` grammar and feeds it to the
generic `GrammarLexer` from the `lexer` crate, then applies a single
post-tokenize hook for S's context-sensitive newline rule.

## How it fits in the stack

```text
code/grammars/s.tokens   (token grammar — single source of truth)
        |  compiled ahead of time by grammar-tools
        v
src/_grammar.rs          (embedded TokenGrammar; do not edit by hand)
        |
        v
lexer::GrammarLexer      (tokenizes using the embedded grammar)
        |
        v
s-lexer (this crate)     (adds the bracket-interior newline hook)
        |
        v
s-parser → s-runtime → s-repl
```

## The historical `_` assignment operator

In historical S the underscore **is** the assignment operator, identical to
`<-`:

```s
x _ c(1, 2, 3)     # assigns c(1,2,3) to x
```

That is why an underscore was never part of an S identifier — and why R
programmers are still told to avoid `_` in names. This lexer is faithful: `_`
is the `UNDERSCORE` token and the `NAME` pattern excludes it.

## Newline handling

S ends a statement at a newline, except inside `( )` or `[ ]` (so calls and
indices may span lines). Inside `{ }`, newlines stay significant (they separate
a block's statements, as in R). The `drop_bracketed_newlines` hook implements
exactly this by tracking parenthesis/bracket depth.

## Usage

```rust
use coding_adventures_s_lexer::tokenize_s;

let tokens = tokenize_s("x <- c(1, 2, 3)\n");
for t in &tokens {
    println!("{} {:?}", t.effective_type_name(), t.value);
}
```

Use `try_tokenize_s` to get a `Result` instead of panicking on a lexical error.

## Regenerating the embedded grammar

`src/_grammar.rs` is generated from `code/grammars/s.tokens`. Regenerate it with
`code/scripts/generate-compiled-grammars.sh` (or the Rust grammar-tools CLI
`grammar-tools compile-tokens code/grammars/s.tokens -o src/_grammar.rs`).

## Testing

```sh
cargo test -p coding-adventures-s-lexer
```

See [`code/specs/S00-s-language.md`](../../../specs/S00-s-language.md) for the
full language specification.
