---
category: Compiler / VM / language pipeline
---

# F10 mode transitions never fire on the GrammarLexer's built-in NEWLINE token

`GrammarLexer::tokenize_standard` emits a NEWLINE for a bare `\n` *before* it
tries any token pattern, then `continue`s — it never calls
`apply_transitions`. So a `.tokens` file cannot write
`on NEWLINE -> set-mode beg`, and defining `NEWLINE = /\n/` as a pattern does
nothing either: the built-in check wins first. The next line starts in
whatever mode the previous line ended in.

Found while porting the Ruby lexer to `ruby<release>.tokens` (LANG84 R0):
Ruby's lexer is back in EXPR_BEG after a newline, so a line starting with
`?a` or `:sym` (or, in Ruby 0.49, `-2`) after a line ending in a value lexes
differently from the real lexer.

What to do: treat it as a documented divergence and pin it with a test (the
Ruby pairs do), or change the engine to apply transitions to built-in
NEWLINE tokens — but `lexer` is a crate almost everything depends on, so
that is its own PR with an F10 amendment, not a drive-by in a language PR.
Skip patterns cannot help: they never trigger transitions either.
