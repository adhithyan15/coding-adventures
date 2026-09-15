# Lookaheads are already atomic in ECMAScript — the fix for a per-position scan is a BOUND, not an atomic group

`/!\[[^\]]*\]\(/` on a body full of `![` with no `]` took 49 seconds for 400 KB. The
reflex fix is the atomic-group idiom, `(?=([^\]\n]*))\1`, and I shipped it. It changed
nothing measurable: 54 seconds after, 54 before.

A lookahead in JS is already atomic — the engine does not backtrack into it — so there was
no backtracking left to remove. The cost was never backtracking. It was that an unbounded
`*` scans to end-of-text at EVERY starting position, and `!` appeared 200,000 times.
`{0,200}` is what made it linear (303 ms on 1.6 MB).

- **Distinguish "backtracks catastrophically" from "does O(n) work per start position".**
  Only the first is fixed by atomicity; the second needs a bound on the quantifier.
- **Measure the fix, do not reason about it.** Both readings were one `node -e` away, and
  the wrong fix survived a security review round because the story was plausible.
- **A bound on a quantifier is a new false negative unless you handle the cap.** `{0,200}`
  made a figure with 250 characters of alt text report as "no figure", which gated off the
  cues that depended on it and marked the lesson drivable. Treat "the cap was reached" as
  evidence the thing IS there, not evidence it is absent.
