---
category: Repo policy / workflow reminders
---

# A frequency-ranked exam worklist exhausts itself; switch to the words that alone block an item

The Spanish A2 vocabulary work is driven by a book-bounded mock audit: it names
every lexeme the two DELE A2 papers need and the corpus does not teach, and the
selection rule was "teach the ones appearing in two or more failing items
first", because each of those buys down more items per word.

That rule works exactly twice. After 45 headwords, **145 of the 146 remaining
lexemes appeared in exactly one item**, and the single exception was a word
already taught above the A2 ceiling and therefore off limits. Ranking by
frequency had become ranking by nothing, and a third tranche picked that way
would have been picked at random while still looking principled.

**The rule that replaces it comes from how an item passes.** An item is red
until EVERY lexeme in its `requires` row is taught, so what matters is not how
often a word appears but how many words its item is still waiting on. Counting
the failing rows by size:

| shape of the failing row | items |
|---|---|
| blocked by exactly ONE missing word | 41 of 79 |
| blocked by exactly TWO | 16 |
| three or more | 22 |

Thirty-nine of those 41 were teachable, and **each one clears a whole item by
itself** — enough to take the failing count from 79 to 40 in a single tranche.
Nothing a frequency ranking would have surfaced comes close.

**The general shape.** A worklist ranked by how often an item appears optimises
the wrong thing as soon as the head of the distribution is consumed; what stays
useful is ranking by *how close each blocked thing is to being unblocked*. The
same reasoning applies to any gate that passes only on full coverage of a set —
a failing test with one unimplemented dependency is cheaper than one with four,
whatever the popularity of the dependency.

**What to do.** When a derived worklist stops discriminating, say so and
re-derive it from the pass condition rather than continuing to apply a rule
that no longer separates anything. Check the distribution, not just the top of
the list: "145 of 146 are singletons" is one query, and it is the difference
between a principled tranche and a random one.
