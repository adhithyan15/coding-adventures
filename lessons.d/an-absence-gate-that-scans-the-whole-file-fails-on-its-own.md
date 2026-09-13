---
category: Testing & coverage
---

# An absence-gate that scans the whole file fails on its own documentation

2026-09-13.

Three gates in one change were written as "this token must not appear in the
file" and all three failed the same way: on the comment explaining why the token
is banned.

- `openCard` must not be answered — failed on the comment saying `openCard` is
  deliberately not answered, because it is a `Notify`.
- `\A`/`\z` must not appear in the Dart handler (they are identity escapes
  there, not anchors) — failed on the comment recording that measurement.
- `$target.part` must not be the export's staging name — failed on the comment
  explaining that a guessable staging name is symlink-redirectable.

Banning a token also bans documenting the decision to avoid it, which makes the
file worse and the gate no stronger. The thing that is load-bearing is whether
the token reaches CODE.

Scan code lines only. `code_lines()` in
`code/programs/mosaic/engram-app/tests/package_compiles.rs` is the helper;
whole-line `//` comments are dropped and nothing else is parsed.

The same shape applies to positive gates. `assert_contains(&handler,
"confirmDelete")` passed against a branch nothing could reach, because the
string was in the file — presence in the text is not reachability in the
program. When what matters is dispatch, read the dispatch: collect the quoted
kinds from the lines carrying the `->` or the `kind ==`, and assert the SET.
