---
category: Compiler / VM / language pipeline
---

# Custom token types (DIMENSION, HASH_COLOR, TOKEN_REF, …) do NOT get new `TokenType` enum variants

The GrammarLexer maps custom-named regex tokens to `type_ = TokenType::Name` with `type_name = Some("DIMENSION")` etc. To detect them in a Rust compiler, use `t.type_name.as_deref() == Some("TOKEN_REF")`, NOT `t.type_ == TokenType::TokenRef` (which doesn't exist). Tests must check `t.type_ == TokenType::Name && t.type_name.as_deref() == Some("DIMENSION")`.
