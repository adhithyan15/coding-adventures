# S Parser

A grammar-driven parser for the historical
[S programming language](https://en.wikipedia.org/wiki/S_(programming_language))
(Bell Labs, 1976) — the ancestor of R.

## What it does

Turns the token stream from `coding-adventures-s-lexer` into a parse tree
(`GrammarASTNode`) using the generic `GrammarParser` from the `parser` crate. It
hand-writes no parsing logic: the S syntax lives in `code/grammars/s.grammar`,
compiled ahead of time into the embedded `src/_grammar.rs`.

## How it fits in the stack

```text
code/grammars/s.grammar   (parser grammar — single source of truth)
        |  compiled ahead of time by grammar-tools
        v
src/_grammar.rs           (embedded ParserGrammar; do not edit by hand)
        |
        v
s-lexer → Vec<Token> → parser::GrammarParser → GrammarASTNode
        |
        v
s-runtime → s-repl
```

## The tree

Each `GrammarASTNode` carries a `rule_name` (the grammar rule that matched) and
children that are deeper nodes or raw tokens. Operator precedence is encoded by
nesting depth, following the grammar's cascade:

```
assignment   <-  _  <<-  ->     (loosest, right-associative)
comparison   == != < > <= >=
range        :
additive     + -
multiplicative * /
unary        - (prefix)
power        ^                   (right-associative)
postfix      f(...)  x[...]      (tightest, left-associative)
primary      atoms, function, if/for/while, blocks, groups
```

## Usage

```rust
use coding_adventures_s_parser::parse_s;

let ast = parse_s("x <- c(1, 2, 3)\nmean(x)\n");
assert_eq!(ast.rule_name, "program");
```

Use `try_parse_s` for a `Result` instead of a panic.

## Regenerating the embedded grammar

`src/_grammar.rs` is generated from `code/grammars/s.grammar` via
`code/scripts/generate-compiled-grammars.sh` (or
`grammar-tools compile-grammar code/grammars/s.grammar -o src/_grammar.rs`).

## Testing

```sh
cargo test -p coding-adventures-s-parser
```

See [`code/specs/S00-s-language.md`](../../../specs/S00-s-language.md) for the
full specification.
