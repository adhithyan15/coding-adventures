## HL-C10Q — the last two merge-conflict surfaces are sharded

HL21 removed the conflict points in the curriculum's **data** and left the two
files every author touches regardless of what they are working on. Measured over
the last 200 human-languages commits on main: this `BACKLOG.md` was touched by
**100** of them and `human-language-data/CHANGELOG.md` by **75**, while the 23
per-language `<track>/CHANGELOG.md` files were touched **4–11 times each**. The
per-language changelogs are already partitioned by track and were left alone;
sharding a file that does not conflict buys nothing.

The corroborating experiment was PR #12690 itself. It went `DIRTY` three separate
times while two concurrent Spanish tranches were in flight, and every time the
conflicting file was **only** `CHANGELOG.md` — not one of its ~4,000 shard files
ever conflicted. A PR whose entire purpose was to remove a conflict point was
blocked three times by a conflict point it did not remove.

**Two things this migration had to get right that HL21's did not.**

First, these documents are **newest-first**. HL21's ledgers append, so a new
element takes the next ordinal and nobody renames anything. A backlog prepends,
so under ascending filename order the newest entry needs the *smallest* ordinal
and every author reaches into a shrinking gap that two of them would both grab.
The fix is that the ordinal is a **recency rank** — topmost section, highest
number — and the join walks it downward. Prepending is an append again.

Second, prose has no ids. HL21 derives a shard filename from an authored `id`
field; 106 of this file's 107 sections start `## HL-…` and one does not (*Three
rules this work keeps re-deriving*), so an id-based scheme would have had to
special-case it. The filename is derived from the heading text and the *result*
is validated instead — and the identity is an 8-hex digest of the heading, not
the ASCII-folded slug, because `source-verified Tamil ர` and `source-verified
Tamil த` fold to the same slug.

**The migration introduces no normalization.** The sharder partitions bytes at
heading boundaries and rebuilds by concatenation, so both regenerated monoliths
are byte-identical to the pre-migration files.

**Found while measuring, not fixed here:** `human-language-data/CHANGELOG.md`
carries **three** separate "Unreleased" sections — `## Unreleased` at line 3, a
second `## Unreleased` at line 952, and `## [Unreleased]` at line 1021. Two of
the three are almost certainly the residue of exactly the bad merges this work
exists to prevent: someone resolved a conflict by keeping both sides, heading
included. They survive untouched because byte-exactness outranks tidiness;
consolidating them is a separate deliberate commit for a quiet moment, and it has
to decide what those sections were meant to be released *as*.

**Also worth knowing:** sharding does not reduce CI cost for either file. Both
sit inside directories CI treats as opaque path prefixes, so a changelog-only
edit still triggers the 23-book XeLaTeX compile and a full `npm ci` +
`vitest --coverage`. The cost sharding removes is human serialization, not CI
minutes. Narrowing those path triggers is separate work.

