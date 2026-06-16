# R Parser

A grammar-driven parser for the
[R language](https://en.wikipedia.org/wiki/R_(programming_language)) — "an
implementation of the S language" (Ihaka & Gentleman, 1993).

## What it does

Turns the token stream from `coding-adventures-r-lexer` into a parse tree
(`GrammarASTNode`) using the generic `GrammarParser`, driven by the embedded
`r.grammar` (`src/_grammar.rs`). It hand-writes no parsing logic.

## Built to share the S evaluator

R shares S's semantics, so `r.grammar` uses the **same rule names** as
`s.grammar` (`assignment`, `comparison`, `additive`, …, `postfix`,
`call_suffix`, `primary`, …). The `s-runtime` tree-walker dispatches on
`rule_name`, so the same evaluator runs R programs once item R-3 wires it up.
The only grammar differences from S are where R departs from S:

| | S | R |
|---|---|---|
| Assignment ops | `<- <<- ->` (and `_`) | `<- <<- -> ->>` and **`=`** |
| Typed `NA` | — | `NA_integer_`, `NA_real_`, `NA_character_` |

Inside a call, `f(x = 1)` is still a *named argument*: the `arg` rule tries
`NAME = expr` before the positional `expr`, so `=`-as-assignment at the top level
and `=`-as-named-arg inside calls coexist.

## Usage

```rust
use coding_adventures_r_parser::parse_r;

let ast = parse_r("data_frame <- c(1, 2, 3)\nmean(data_frame)\n");
assert_eq!(ast.rule_name, "program");
```

Use `try_parse_r` for a `Result` instead of a panic.

## Regenerating the embedded grammar

`src/_grammar.rs` is generated from `code/grammars/r.grammar` with
`grammar-tools compile-grammar code/grammars/r.grammar -o src/_grammar.rs` —
never hand-edit it.

## Testing

```sh
cargo test -p coding-adventures-r-parser
```

See [`code/specs/R00-r-language.md`](../../../specs/R00-r-language.md).
