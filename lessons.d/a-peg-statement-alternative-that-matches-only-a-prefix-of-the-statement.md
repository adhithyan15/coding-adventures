---
category: Compiler / VM / language pipeline
---

# A PEG statement alternative that matches only a prefix of the statement commits, so a call alternative tried before the expression alternative needs lookaheads for every operator that can continue it

**What went wrong.** `statement` in `ruby.grammar` lists `method_call`
before `expression_stmt`.  For `f() + f`, `method_call` matched `f()`, the
choice committed, and the next "statement" started at `+`: a parse panic, at
the start of a statement and inside a block or `->` body (#18778).  Worse,
`f() - 1` parsed as two statements, `f()` and `-1`, with no error.  The same
happened to `method_call` inside `modifier_statement`.

**Fix.** The `method_call` alternatives are followed by negative lookaheads
for every operator that `ternary` … `term` can continue an expression with
(`+ - * / << == != <= >= < > && || and or ... .. ?`).  When one follows, the
alternative fails and `expression_stmt` parses the whole expression.  `[` is
deliberately not in the list (see the lesson on line-start brackets).

**Do differently.** In a PEG, an alternative that can match a *prefix* of
what a later alternative matches wins and never backtracks.  For each such
pair, either put the longer alternative first or guard the shorter one with
lookaheads for every token that can extend it, and test a statement that
continues past the prefix.
