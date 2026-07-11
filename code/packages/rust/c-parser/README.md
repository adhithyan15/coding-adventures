# c-parser (coding-adventures-c-parser)

Parser for the **C integer-core subset** (SIR27) — the syntactic layer of the
`c-to-semantic-ir` frontend.  It tokenizes with `coding-adventures-c-lexer` and
feeds the tokens to the generic `parser::GrammarParser` driving the compiled
`c.grammar` (`code/grammars/c/c.grammar`).

Implements the syntactic part of
[SIR27](../../../specs/SIR27-c-to-semantic-ir.md).

## API

```rust
use coding_adventures_c_parser::{parse_c, try_parse_c, create_c_parser};
let cst = parse_c("int main(void) { return 2 + 3 * 4; }");
assert_eq!(cst.rule_name, "translation_unit");
```

The result is the generic `parser::grammar_parser::GrammarASTNode` CST
(`rule_name` + `children`); consumers walk it by `rule_name`.  The full C
expression precedence cascade is encoded in the grammar; a `(T)e` cast is
disambiguated from a parenthesised expression by the type keyword after `(`.

## Regenerating the grammar

`src/_grammar.rs` is generated from `code/grammars/c/c.grammar`:

```
grammar-tools compile-grammar code/grammars/c/c.grammar -o code/packages/rust/c-parser/src/_grammar.rs
```
