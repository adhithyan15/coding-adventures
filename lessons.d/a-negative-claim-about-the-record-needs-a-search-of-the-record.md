---
category: Repo policy / workflow reminders
---

# A negative claim about the record needs a search of the record, not of the files you happened to open

While correcting the CLOC19 catch-param docs (CCR-022), a review observed that a
rationale I had written — "these five fixtures were excluded from the
`upstream_refuses` cohort because their refusal is harness incompleteness" —
appeared nowhere in the record. I checked three sources: CCR-081's CHANGELOG
entry, the manifest's `local_boundary`, and the captured evidence artifact. It
was absent from all three. I concluded the decision had never been made and
rewrote the text to say the class "was never scanned for it".

That inverted a true statement into a false one. The rationale is recorded twice
on issue #15868 — once in the 2026-09-21 sweep comment, which tabulates the five
fixtures by name under "Harness incompleteness, not refusal", and once in a
comment I wrote myself 29 minutes earlier, which says "I excluded them
deliberately". The same sentence I was rewriting cited #15868. I did not open it.

**The shape of the mistake.** Three consecutive correction rounds on that change
each introduced a new false claim, and all three were checkable facts asserted
without checking: the name of a test that pins a guard, a crate's test count, and
this one. The third is the worst because it is a *negative existential* — "this
is not recorded anywhere" — which is only as good as the search behind it. A
positive claim that turns out wrong is usually caught by the next person who
reads the thing it describes. A negative claim licenses deleting or reversing
something true, and the evidence that would refute it is by construction
somewhere you did not look. It is also the mistake most likely to happen while
fixing another mistake: a reviewer says "the record does not support this", the
reviewer is reporting what *they* searched, and adopting that as a finding
without repeating the search across the whole corpus is how a correction becomes
a regression.

**What to do instead.** Before writing "X is not recorded / never decided / has
no issue / appears nowhere", enumerate what "the record" is here — in this repo
that is at minimum the specs, the CHANGELOGs, the manifest and artifacts, and the
GitHub issues and PR comments — and search all of it; a `gh`-less session can
read issue comments through the API, so there is no excuse for skipping them.
Any issue or PR the sentence itself cites is mandatory reading before the
sentence ships: if you are pointing a reader at #N as the place this lives, you
have to have looked at #N. Prefer a positive, bounded claim to a negative
unbounded one — "absent from the CHANGELOG, the manifest and the evidence
artifact" is checkable and stays true, "never scanned for" is neither. When a
review says a claim is unsupported, treat that as a hypothesis to test rather
than a finding to adopt, because reviewers search a subset too. And remember
that your own recent comments are part of the record: search history before
concluding something was never considered, since the author you are contradicting
may be you, an hour ago.
