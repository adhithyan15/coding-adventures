# coding-adventures-wolfram-lexer

The tokenizer for the **Wolfram Language** (Mathematica) M-expression subset —
W-2 of the Wolfram frontend.

Wolfram's surface is built around `head[arg, …]` (square-bracket application),
`{a, b}` list braces, the replacement operators `/.` `->` `:>`, the pattern
blanks `_`/`x_`, and (W-6) the operator sugar `/@` (Map), `@@` (Apply), and `[[`
(Part). This crate is a thin wrapper over the generic `GrammarLexer`
(a sibling of `r-lexer` / `macsyma-lexer`) with the committed `_grammar.rs`
compiled from [`code/grammars/wolfram.tokens`](../../../grammars/wolfram.tokens),
plus one hook: it drops `NEWLINE` tokens inside an open `(`, `[`, `{`, or `[[`
so a grouping, application, list, or part expression may span lines (top-level
newlines terminate a statement).

The part-sugar opener `[[` is one token (`LDBRACKET`), but there is deliberately
no `]]` token — a closing `]]` lexes as two ordinary `]` (`RBRACKET`), so the
tail of nested ordinary application `f[g[x]]` is never mis-lexed.

## Where it fits in the stack

```
wolfram.tokens  →  wolfram-lexer (this crate)  →  wolfram-parser (W-3)
                                                →  wolfram-runtime (W-4, lowers to symbolic-ir)
```

## Usage

```rust
use coding_adventures_wolfram_lexer::tokenize_wolfram;

let tokens = tokenize_wolfram("f[x_] := x^2\n");
// f  [  x  _  ]  :=  x  ^  2
assert_eq!(tokens[1].effective_type_name(), "LBRACKET");
assert_eq!(tokens[3].effective_type_name(), "BLANK"); // `x_` is NAME then BLANK
```

`try_tokenize_wolfram` returns a `Result` instead of panicking on an
unrecognized character.

## Testing

```
cargo test -p coding-adventures-wolfram-lexer
```
