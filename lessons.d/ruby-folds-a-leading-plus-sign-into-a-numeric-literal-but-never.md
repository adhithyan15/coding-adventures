---
category: Ruby
---

# Ruby folds a leading plus sign into a numeric literal but never a minus sign

`Ripper.lex("w = +4")` gives one `on_int "+4"`, but `Ripper.lex("x = -1")`
gives `on_op "-"` then `on_int "1"`. parse.y's `+` case jumps to
`start_num` with the sign in expression-start position (and, in EXPR_ARG,
only when a space precedes the `+` and none follows: `foo +1`), while `-`
returns `tUMINUS_NUM` as its own token. It applies to every numeric form:
`+0x1F`, `+1.5`, `+0b1` are single literals. Ruby 0.49 was symmetric (it
folded `-1` too).

Found by the R0 Ripper differential (LANG84). A lexer that treats the two
signs alike disagrees with Ruby on `+4`, and one that folds in identifier
(ARG) position gets `a+1` wrong. The 3.3 pairs fold `+` only in their `beg`
mode and never in `arg`, leaving the space-sensitive `foo +1` to E6.
