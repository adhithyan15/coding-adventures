---
category: Rust
---

# A lexical Rust gate must normalise r#ident to the bare identifier, or every name-matching rule is bypassed

`vos-shape-check` (VOS-D002) first lexed a raw identifier `r#unwrap` as the
identifier text `"r#unwrap"`, on the reasoning that `r#unsafe` must never be
mistaken for the keyword `unsafe`. That is right for keywords and wrong for
everything else: in Rust `r#name` and `name` are the SAME identifier, so
`o.r#unwrap()`, `r#f32`, `r#usize`, `r#Vec`, `r#div`, `r#std`, `r#Cell`, `mod
r#ghost` and `fn r#alpha` all slipped past every rule that matches a name. The
security review reproduced a false PASS for each.

Fix: the lexer strips `r#` when the remaining name is not a keyword and keeps
`r#` only for keywords (`r#unsafe`, `r#fn`), which are identifiers, not the
keyword. Test every name-matching rule with the `r#` spelling in one table
(`tests/security.rs`, item 2), and assert that a raw keyword stays clean.

Do differently: whenever a gate compares token text against a name list, ask
what other spellings the compiler treats as the same name (raw identifiers,
unicode escapes in strings are not names, but `r#` is) and normalise once, in
the lexer, not in each rule.
