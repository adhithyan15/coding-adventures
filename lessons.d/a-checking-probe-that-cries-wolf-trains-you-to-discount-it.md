---
category: Testing & coverage
---

# A checking probe that cries wolf trains you to discount it

Working through four documentation changes in one session, I wrote a probe before nearly every claim
— heading counts, paragraph structure, use-versus-mention of retracted figures, needle presence. That
is the habit this corpus keeps recommending, and I am not arguing against it. But I kept a ledger of
what those probes actually reported, and the ledger is not flattering.

**Counted for 2026-09-16, scope: probes I wrote to check a finished claim, this session only.**
Measuring instruments *inside* the work — the census that re-derived a count, the control whose arms
refused to differ — are outside this ledger; both found real errors, and counting them would make it
4/2/2 over n=8. The row below is about probes written to audit something already written.

    false alarms raised by my own probes      4
    real defects caught by my own probes      0
    real defects caught by security review    2

A false alarm here means the probe reported a defect and the content was correct. This is a **ledger
of one session, n=6 — not a rate.** I am not claiming probes are usually wrong; I am recording that
in this session every defect my probes flagged was imaginary and every real one came from elsewhere.

**The causes split evenly, and only one half is cheap to fix.**

**Two were LABEL errors: the probe measured correctly and printed a word meaning the opposite.** One
printed `one day earlier removed: False (want False)` — where `False` meant the phrase was *absent*,
i.e. the fix had landed, and I read it as a failure. Another printed `stale claim gone: 1 (want 0)` —
where `1` meant the phrase was present, but only as a quotation inside the very sentence retracting
it. Both numbers were right. Both sentences around them were wrong, in opposite directions.

*The fix is mechanical: print the predicate and the wanted value beside the number, so the line says
what was measured rather than what I hoped it meant.* `occurrences of "one day earlier" = 0 (want 0,
absent means fixed)` cannot be misread the way `removed: False` can.

**Two were PREDICATES TOO CRUDE to separate legal structure from a defect.** A bold-line probe
flagged a paragraph as welded because a `**` line followed a non-blank line — but that was a
mid-sentence bold run continuing `…moves the count from` into `**100 to 84**`, which is ordinary
Markdown. And a use-versus-mention regex flagged a live assertion of a retracted figure, because
flattening had collapsed a Markdown table into one long pseudo-sentence and the digits inside
`1 H1, 0 H2, 0 H3` matched. Neither predicate could tell a legal construct from the defect it was
hunting. That half is not cheap: it needs the predicate to model the document's structure, not its
characters.

**Why this matters more than the wasted minutes.** Four alarms, none real, is training data. By the
fourth I was reaching for "the probe is probably wrong again" *before* reading the text — and on that
same day two genuinely wrong numbers shipped past me into a commit and were caught by review, not by
me. An instrument that fails loudly and often erodes exactly the reflex it exists to create. The cost
is not the false alarm; it is the true one you have already decided to disbelieve.

**What I am NOT concluding.** Not that the probes should go. Two of this session's real errors were
found *by* measurement — a census that re-derived a count, and a control whose arms refused to differ.
The distinction that survives is between a probe that **measures** and a probe that **judges**:
counting is where this session's useful probes were, while classifying what an occurrence *means* —
assertion versus quotation, weld versus legal run — is where all four false alarms came from. That is
not a promise that counting is safe: a count with the wrong needle fails too, as the substring probe
below and [[a-needle-counted-against-raw-text-misses-every-phrase-the-source-wrapped]] both show.
Measure with a probe. Judge by reading the line it points at.

**Two more, logged while writing this entry, which is the least surprising sentence here.** A probe
meant to confirm this file's title slugs back to its filename **crashed** — it imported `lessons.py`
via `importlib`, and the module's dataclass blew up on `cls.__module__` being `None` under
`exec_module`. A crashing probe is at least honest; it reports nothing rather than reporting wrongly.
The fix was to run the tool as a subprocess and read what it says, which is also how the real defect
surfaced: `validate` rejected my hand-typed filename because the title slugged to a different stem.
Separately, a probe comparing figures between this shard and its commit message printed substring
hits — searching for `4` and `2` and counting every digit in the file — so its output was noise
dressed as agreement. **Neither is a false alarm in the ledger above**, because neither reported a
defect that was not there; they are two further failure modes — a probe that dies, and a probe whose
answer means nothing. Note where the second belongs: a substring count is a **measuring** probe with
the wrong needle, not a judging one, which is why the measure/judge split above is a description of
this session rather than a guarantee.

Related: [[a-needle-counted-against-raw-text-misses-every-phrase-the-source-wrapped]] — the same
session, and the counting half of this: there the probe's arithmetic was wrong; here the arithmetic
was right and the interpretation was not.

Related: [[writing-a-lesson-down-does-not-stop-you-rebuilding-the-bug-assert]] — its remedy is to
state a rule as an executable check rather than prose. This is the failure mode on the other side of
that advice: a check whose verdict is unreadable is not better than the prose it replaced.

Related: [[an-instrument-that-enforces-a-convention-nobody-wrote-down-manufactures-defects]] — the
same session's other instrument that accused correct prose, and the complementary cause. There the
predicate encoded a convention I had invented; here the predicate was fine and its *label* lied. Its
remedy is to count the convention across the corpus before enforcing it; this one's is to print the
predicate and the wanted value beside the number.
