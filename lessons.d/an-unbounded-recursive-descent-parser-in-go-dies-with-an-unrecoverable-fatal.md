---
category: Security boundaries
---

# An unbounded recursive-descent parser in Go dies with an unrecoverable fatal stack overflow, not a panic

`go/parser`'s packrat `GrammarParser` recursed once per rule level with no
bound. Input like `f(` plus 200,000 `[` then the closers ended in
`fatal error: stack overflow` once the goroutine stack hit Go's 1 GB cap.
That is a runtime `throw`, not a panic: `recover()` cannot catch it and the
whole process dies, so one hostile request took down every consumer
(json-parser and all the other `*-parser` packages).

What to do: count rule depth yourself and return an error past a limit
(`Options.MaxDepth`, default 10,000; measured about 1 KB of stack per level, so
the default uses roughly 16 MB). Two traps in doing it:

- The refusal happens deep inside optionals and repetitions that backtrack past
  it and succeed on a truncated prefix, so the depth flag must be sticky and
  checked first in `Parse` (the Rust `GrammarParser` hit the same thing).
- Reading past the end of the token list used to repeat the last token, so a
  token list with no EOF (a lone `(`) recursed forever with no hostile nesting
  at all. Past-the-end must be a synthetic EOF.

- A depth counter on the call stack does not bound the TREE. Left-recursion
  seed growing is a loop (call depth stays 1) that builds one tree level per
  iteration, and any recursive consumer of the finished tree (position
  computation, walkers) then overflowed at about 7M terms. Charge growth steps
  to the same budget and make tree walkers iterative.
- Counting rule calls alone also misses recursion inside a rule body (nested
  groups/optionals); charge each rule for its element nesting too.

Keep a hostile-nesting test (a million brackets) that asserts the error
category, not just `err != nil`.
