# coding-adventures-wolfram-parser

The parser for the **Wolfram Language** (Mathematica) M-expression subset — W-3
of the Wolfram frontend.

A thin wrapper over the generic `GrammarParser` (a sibling of `r-parser` /
`macsyma-parser`) with the committed `_grammar.rs` compiled from
[`code/grammars/wolfram.grammar`](../../../grammars/wolfram.grammar). It tokenizes
with `wolfram-lexer`, then parses into a `GrammarASTNode` rooted at `program`.

```rust
use coding_adventures_wolfram_parser::parse_wolfram;

let ast = parse_wolfram("f[x_] := x^2\n");
assert_eq!(ast.rule_name, "program");
```

Everything in Wolfram is `head[args]`; the tree's rule names (`replaceall`,
`rule`, `additive`, `power`, `mapapply`, `postfix`, `list`, …) are the surface
forms the W-4/W-6 `wolfram-runtime` lowers into canonical `symbolic-ir` heads.
`try_parse_wolfram` returns a `Result` instead of panicking.

The W-6 operator sugar lives in two rules: the infix `mapapply` level
(`f /@ x`, `f @@ x`) sits between `power` and `postfix`, and `postfix` carries
the `[[ … ]]` part sugar (`x[[i]]`) alongside `f[…]` application. The runtime
desugars these to the `Map`/`Apply`/`Part` heads, so each is identical to its
long form.

## Where it fits

```
wolfram.grammar → wolfram-parser (this crate) → wolfram-runtime (W-4, → symbolic-ir)
```

## Testing

```
cargo test -p coding-adventures-wolfram-parser
```
