---
category: Repo policy / workflow reminders
---

# Writing a lesson down does not stop you rebuilding the bug - assert it in the instrument

A census over `adj-facts-stdlib` has to decide whether a table row opens a
provenance block. The obvious test is wrong:

```python
if s.endswith("{"):
    braced += 1
```

A single-line row block is legal and common:

```
row (venus, 2) { source "Venus is the second planet from the Sun, and the sixth largest planet." }
```

That line opens a block and `endswith("{")` calls it bare. `astronomy/planets.adj`
came out as **1 of 8 rows converted** when it is 8 of 8.

The "1" is the tell. That file mixes both forms: `row (mercury, 1) {` opens a
multi-line block, and the other seven rows are single-line. The broken test
counted the one row whose brace happened to land at end-of-line, which is why
the result looked like a plausible partial conversion rather than an obvious
failure.

This is not a repeat of an earlier bug, and the accurate version is worse.

An earlier census in this corpus undercounted the same file for a *different*
reason — it keyed on a needle of `\n` plus eight spaces plus `source "`, which
reads nothing out of an inline block. That was written up as
[[a-census-keyed-on-one-of-a-construct-s-two-spellings-undercounts]], created
2026-09-14 and amended 2026-09-15. Its corrected rule says, in as many words:

> **Test for the construct, not for what it holds or how it is laid out.**
> "Does any `row (…)` line open a brace?" is right for all three shapes.

Rebuilding the census on 2026-09-16 — one day after that amendment — I set out
to implement exactly that sentence, and wrote `endswith("{")`. I believed I
*was* testing whether the line opens a brace.

So the prose rule was correct, recent, about this corpus, about this construct,
and I had it. It still admitted a wrong implementation, because "opens a brace"
and "ends with a brace" read the same to someone typing quickly and are not the
same predicate. A sentence cannot reject a misreading of itself; a check can.

It was not cosmetic. The corpus-wide counts were wrong by three files
(81 converted / 2 partial / 266 not, against the correct 84 / 1 / 264). The
three, named so the claim is checkable: `astronomy/planets.adj` (reported
partial, actually converted), `biology/kingdoms.adj` and
`metrology/si-base-units.adj` (both reported unconverted, both converted). The
latter two are the same files the earlier lesson records being **published on
the tracking issue as unconverted work** — so a differently-broken instrument put
the identical two files back into a work queue they had already left.

```python
# A ROW OPENS A BLOCK IF IT CONTAINS "{", NOT IF IT ENDS WITH ONE.
# Single-line row blocks are legal: row (venus, 2) { source "..." }
# An endswith test buckets every one of them as unconverted, and the
# prose rule "does the row line open a brace?" does not rule it out.
if "{" in s:
    braced += 1
```

**How to apply.** A lesson in a notes file is documentation, not a guard. When a
rule is about one line of code, state it as a **check with a control**, not as a
sentence: pick a file whose answer is known independently and assert the
instrument reproduces it before any number from that instrument is believed.
Here that is one line — `planets.adj` must read 8 of 8 — and it would have
failed immediately on the wrong predicate. The prose record is for the reader
deciding what to build; the control is for the author already building it, and
that is the person who gets it wrong.

**And note what this lesson cannot do for itself.** `endswith("{")` appears in
no committed *instrument* in this repository — only in the prose and code
blocks of this shard: the census was ad-hoc, written in a scratch directory,
and thrown away. So there is no line here to attach a comment to, and the next
rebuild starts from prose again — which is precisely the failure being
described. A rule that only survives in notes will be
re-implemented from notes. If an instrument is going to be rebuilt more than
once, the durable fix is to **commit the instrument**, control and all, rather
than to write a better note about it.

Two things actually caught it, and both are worth copying. The instrument
printed the row TEXT beside its verdict, so `bare` appeared next to a line
visibly containing `{`. And before any number was trusted, it was run against a
file whose answer was known independently — `planets.adj` must read 8 of 8.

A control made of remembered expectations mostly tests your memory: the same run
included a second file I predicted would read "4 of 6", which reads 2 of 6, and
both the broken and the fixed test agreed on it. That case discriminated
nothing. One case in the control did the whole job.

One last thing, because it happened while this file was being written. The
`venus` line above was first quoted here as `...and the sixth largest." }`. The
file says `...and the sixth largest planet." }`. The quote came from a listing
that truncated the line at 78 characters, and the tail was completed from
inference rather than read — inside a lesson about not trusting an instrument's
abbreviated output. Caught by diffing the quoted string against the file's bytes
and printing the first diverging character, which is the same habit the lesson
recommends and the reason it is worth recommending. The earlier shard quotes
that same line correctly; the copy that went wrong was the one made from a
screen rather than from the file.

That happened three times while this one file was being written: the `venus`
line completed from a truncated listing; a bolded quotation of the sibling that
said "a tracking issue" where the sibling says "the"; and a sentence claiming
`endswith("{")` appeared in no committed file, which its own code blocks
falsified four lines above it. Each was caught by comparing against the source
bytes, and none by re-reading what I had written. Re-reading finds the errors
you can still see; a diff finds the ones you have stopped seeing.
