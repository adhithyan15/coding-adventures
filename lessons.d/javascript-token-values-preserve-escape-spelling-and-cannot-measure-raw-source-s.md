---
category: Rust
---

# JavaScript token values preserve escape spelling and cannot measure raw source spans

An external native raw-span observer initially assumed the generic lexer's
default escape processing applied to JavaScript. Its assertion that a source
literal containing `A\nB` produced a three-byte decoded token failed. The
actual ES2025 grammar sets `escapes: none`: quotes are stripped, while the
four-byte interior spelling `A\nB` is preserved. The source literal including
quotes occupies six bytes. A literal containing lambda and an emoji similarly
has six token-value bytes and eight raw source bytes.

At reviewed head `7debd2f26bd36ed243b9cf8273c6c576cd59a886`, the actual compiler's
`lexer_token` origins publish `lexeme_byte_len = token.value.len()`. This is
neither a raw source span nor a universal decoded semantic length. The native
observer and actual traced CLI now agree on all eleven token origins, and the
checked Rust graph importer validates the emitted snapshot. Plain and traced
JS, map, and manifest bytes remain equal. This narrow witness does not establish
output-byte lineage; source-map mappings remain empty.

Read the selected language grammar's escape policy before interpreting token
values. Capture exact UTF-8 start/end at match acceptance, bind them to consumed
content identity, and keep scalar source columns separate from UTF-16 map
columns. The callback has a private end cursor accessible indirectly through
its suffix API, but EOF bypasses callbacks and synthetic/preprocessed tokens
need an explicit policy. Do not infer complete raw-span coverage from the
existence of an AST `Span` type or from token-value lengths.

Evidence: external audit `CV03-raw-span-observation-latest.json`, pointing to
the source/head/library-hash-bound native observation and checked CLI reload.
This observation was staged outside the reviewed CV02 worktree and is now
recorded with the next specification, preserving the approved predecessor head.
