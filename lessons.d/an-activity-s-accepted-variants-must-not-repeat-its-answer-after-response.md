---
category: Testing & coverage
---

# An activity's accepted variants must not repeat its answer after response normalization

Book generation rejected a new Marwadi activity even though its `answer` and
one `accepted` spelling differed visibly by terminal punctuation. Activity
compilation normalizes response punctuation before checking uniqueness, so the
two values became the same response. An exact copy of the answer in `accepted`
has the same failure more obviously.

The fix was to leave the canonical target only in `answer` and reserve
`accepted` for genuinely distinct romanized variants. Before generating a
book, compare answer variants after case, whitespace, and terminal-punctuation
normalization rather than comparing their source strings.
