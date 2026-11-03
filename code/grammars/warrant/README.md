# Warrant v0 canonical grammars

`warrant.tokens` and `warrant.grammar` implement the surface syntax of
[VER04](../../specs/VER04-warrant-language.md), including significant layout,
typed declarations, claims, assumptions, labels, input/bound contracts and
operator precedence. Parsing does not enforce names, types or contracts.

The `rust/warrant-grammar` artifact package embeds these inputs through the
existing grammar-tools compiler. Regenerate from the Rust workspace with:

```sh
cargo run -p coding_adventures_warrant_grammar --example regenerate
cargo run -p coding_adventures_warrant_grammar --example regenerate -- --check
```

The [syntax corpus](../../specs/fixtures/warrant-syntax-v1/README.md) supplies
authored positive/negative outcomes and whole subtree witnesses. Source
admission, original-byte spans and parser-stack safety remain P006b work.
