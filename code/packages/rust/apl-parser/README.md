# coding-adventures-apl-parser

APL parser backed by `code/grammars/apl/apl.grammar`, compiled to Rust and
statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Where this fits

Consumes the token stream from `apl-lexer` (MA-4c) and drives it through
`apl.grammar`'s two-nonterminal design (`value_expr`/`function_expr`, see
[MA05 §3](../../../specs/MA05-apl-language.md)) to produce a `GrammarASTNode`
CST rooted at `program` — the second of the two frontend crates for APL
(MA-4d); the sibling `apl-runtime` crate (MA-4e) will walk this tree to
evaluate.

## Usage

```rust
use coding_adventures_apl_parser::try_parse_apl;

let tree = try_parse_apl("A←⍳5\nB←+/A\n")?;
assert_eq!(tree.rule_name, "program");
```

`parse_apl`/`create_apl_parser` panic on a lexical or syntax error;
`try_parse_apl` returns a `Result` instead.

## Recursion-depth guard

`create_apl_parser` opts the shared `GrammarParser` into a recursion-depth
cap (`MAX_RULE_DEPTH = 100`), empirically derived via the same binary-search
methodology as `macsyma-parser`/`matlab-parser`/`wolfram-parser`'s own caps
— not copied from them. See `src/lib.rs`'s `MAX_RULE_DEPTH` doc comment for
the full derivation. Two distinct input shapes drive `value_expr` deep —
parenthesised nesting `((((…))))` and a flat, unparenthesised dyadic chain
`1+1+1+…+1` (the latter recurses through `value_expr`'s own right-recursive
continuation, with no `(` anywhere in the source) — and they have
*different* native-stack crash floors (209 for parens, ~136 for a flat
chain). `0.1.0` shipped a cap derived from parens alone; `0.1.1` corrected
it against the lower, binding flat-chain floor after the gap was found while
building `apl-runtime` on top of this crate. See the `CHANGELOG` for the
full incident.
