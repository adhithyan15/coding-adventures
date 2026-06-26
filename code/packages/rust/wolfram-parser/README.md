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

The W-11 pure-function syntax adds a `slot` atom (`#`, `#n`, `##`) and a
low-binding postfix `amp` level for the `&` postfix
(`amp = comparison AMP { AMP } { amp_apply } | comparison`), placed just below
`logical_not` and above the comparison/arithmetic stack. So `&` binds looser than
every arithmetic/comparison operator but tighter than `,`, making `#^2 &`,
`# + 1 &`, and `Mod[#,2]==0 &` pure functions of the *whole* body; the
`amp_apply` suffix lets a pure function be applied immediately (`(#^2)&[5]`). The
runtime lowers `#`/`#n` to `Slot[n]`, `##` to `SlotSequence[1]`, and `body &` /
`Function[…]` to a `Function` it resolves by substitution at apply time.

The W-21 pattern operator sugar adds four rules at Wolfram precedence
(loosest→tightest): `//.` joins `/.` in `replaceall`; the new `condition` rule
(`patt /; test`, right-assoc) sits between `rule` and the new `alternatives`
rule (`a | b | c`, infix/left-assoc, looser than `||`); and the new
`patterntest` rule (`patt ? fn`, left-assoc) is inserted between `mapapply` and
`postfix` so `?` binds tightest (just above application). The runtime lowers each
to the W-20 head it desugars to — `Alternatives`, `Condition`, `PatternTest`,
`ReplaceRepeated` — so `a | b` is identical to `Alternatives[a, b]`, etc.

## Where it fits

```
wolfram.grammar → wolfram-parser (this crate) → wolfram-runtime (W-4, → symbolic-ir)
```

## Testing

```
cargo test -p coding-adventures-wolfram-parser
```
