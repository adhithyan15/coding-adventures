---
category: Rust
---

# A mechanical edit that adds a struct field to literals must anchor on the literal's opening, not on a field name

LANG84 E1 added `guard: TokenGuard` to grammar-tools' `TokenDefinition`.
Every committed generated `_grammar*.rs` builds that struct with an explicit
literal, so each literal needed one new line. Regenerating them all was
ruled out because it would have pulled in unrelated source drift (#14202).

**What went wrong.** The first insertion script added the new line after
every line matching `alias: …,` that was followed by `}`. That pattern also
matched the *declaration* of the lexer's private `CompiledPattern` struct
(`alias: Option<String>,` was its last field). The script inserted
`guard: Default::default(),` into a struct definition, and the crate stopped
compiling.

**Fix.** The script now tracks the nearest enclosing `{`. It inserts only
when that line ends with `TokenDefinition {` (or `ModeTransition {`) and is
not a `struct` declaration, and it resets on every `}`. It also skips files
that build the unrelated `state_machine::TokenDefinition`.

**Do differently.**
- Anchor a line-based edit of struct literals on the literal's opening line,
  not on a field name. Field names recur in declarations, other structs,
  doc comments and format strings.
- Check the result with a dry run that counts insertions per file, then
  run `cargo check --keep-going --all-targets` over every crate that depends
  on the changed struct.
- Make the generator emit the new field in a form that needs no import
  (`Default::default()`, fully qualified paths), so the mechanical edit is
  exactly what a regeneration would write.
- Remember that hash-named dedup modules (`g_<hash>`) are renamed by any
  generated-text change.
