# Warrant v0/v1 canonical grammars

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

## v1 additions (VER04j WX-B and WX-C)

The files also carry the v1 surface additively: the optional second header word
(`warrant NAME [ NAME ]`) and the literal tokens `U16_LITERAL U32_LITERAL
I8_LITERAL I16_LITERAL I32_LITERAL I64_LITERAL HEX_LITERAL`. The grammar cannot
see which word a header carries, so *which* second word is allowed (`v1` only,
else `W0101`) and *which* literals a v0 document may use are enforced by the
hand-written parser in `rust/warrant-grammar`. Every v0 lexeme is unchanged and
the `warrant-syntax-v1` corpus is untouched.

## The profile header (VER33 section 4.9, ADL-001a)

The header is now `warrant NAME [ v1 ] [ profile NAME ]`: one more optional
clause, `profile_clause = "profile" NAME`, and one more reserved word,
`profile`.  A document that opts in says

```text
warrant logic_gates_model v1 profile algorithm
```

and the algorithm profile is a **restriction plus additions** that also tightens
meaning (owner decision Q8): the guarded parser refuses `measure` (`W0802`),
unbounded loops and recursion (`W0803`), floats (`W0804`) and effects or state
(`W0805`), each with its own code, and a bad clause is `W0801`.  As with the
version word, the grammar cannot see which profile names are legal, so the
guarded parser in `rust/warrant-grammar` enforces them; `profile` is read only
in the header, so no document body changes.  A document without the clause
parses and checks byte for byte as before.  The syntax corpus has the
grammar-level cases; the diagnostics corpus is
[`warrant-profile-algorithm-v1`](../../specs/fixtures/warrant-profile-algorithm-v1/README.md).

Edits to these files move the pins in `rust/warrant-grammar/tests/wxd_syntax.rs`
and the embedded `_tokens.rs` / `_parser.rs` (`regenerate --check`).
