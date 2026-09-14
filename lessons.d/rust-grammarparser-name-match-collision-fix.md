---
category: Compiler / VM / language pipeline
---

# Rust GrammarParser NAME-match collision fix

(parser/grammar_parser.rs `match_token_reference`): when the grammar expects literal `NAME`, reject tokens whose `type_name` is set (e.g. a `QUOTE` token whose `type_: Name` is just the enum-fallback). The original logic only excluded type_name'd tokens for non-NAME custom types, so a Twig `'foo` would lex `'` as `(type_=Name, type_name="QUOTE")` and then incorrectly match the `NAME` slot in `atom = ... | NAME`. Symmetric tightening: a custom Name-based type reference (e.g. `AT_KEYWORD`) requires `type_name == expected_type` — bare-Name tokens no longer cross-pollute custom types.
