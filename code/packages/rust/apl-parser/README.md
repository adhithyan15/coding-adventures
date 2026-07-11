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
cap (`MAX_RULE_DEPTH = 150`), empirically derived via the same binary-search
methodology as `macsyma-parser`/`matlab-parser`/`wolfram-parser`'s own caps
— not copied from them. See `src/lib.rs`'s `MAX_RULE_DEPTH` doc comment for
the full derivation, including a genuinely counter-intuitive finding: APL's
much shallower grammar (no precedence cascade at all) turned out to have a
*lower* raw crash floor than the other three languages' deeper cascades, the
opposite of the natural "fewer rule calls per level → higher floor" guess —
confirmed only by measuring, not by reasoning about the rule-chain shape.
