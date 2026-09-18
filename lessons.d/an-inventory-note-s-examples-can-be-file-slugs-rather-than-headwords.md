---
category: Repo policy / workflow reminders
---

# An inventory note's examples can be file slugs rather than headwords, so re-derive the claim from the lesson files before scoping from it

`ML-A1-N-06`'s note called the Malayalam plural *"the cheapest grammar lesson
available to this track"*, and gave its evidence: the suffix `-kal` already sat
inside four taught headwords — *nirangal*, *maasangal*, *shareera-bhaagangal*,
*kaalangal* — so *"the learner has met the suffix four times as part of a word."*

**All four are file slugs.** `ML-C11-nirangal.md` is named for colours and its
headword is the four colour words; `ML-C16-kollavarsham-maasangal.md` is named
for the months and its headword is the twelve month names. The suffix is in the
ASCII filename, which is written for the repository and never shown to a learner.
A token sweep of all 393 lesson files found `-kal` in exactly **two** Malayalam
words — *thinkal* (Monday) and *makal* (daughter) — and neither is a plural.
Met zero times, not four.

The note was not lying; it was written by reading a directory listing. That is
the failure mode to watch: **a file slug is prose about a lesson, and prose about
a lesson is not evidence about the language.** The gap between "this lesson is
*about* colours" and "this lesson *teaches* the word *nirangal*" is invisible in
`ls` output and decisive for scoping.

The check costs one grep. Parse `^headword:` out of the lesson files and search
the parsed values as tokens — never search filenames, and never take an example
list in a note as a corpus observation.

Re-deriving it also **found the better lesson**, which is the real argument for
doing it. The same sweep turned up `ningal`, the respectful *you* from the second
chapter, which ends in the plural `-ngngal` and had been in the learner's mouth
for ninety chapters. The chapter became a reveal instead of an introduction. A
wrong note does not only cost you a bad plan; re-deriving it is how you find the
thing the note missed.

This is the third inventory note in this track measured and found materially
wrong (after `ML-A1-F-28` and `ML-A1-N-02`). Treat a note as a pointer at where
to look, never as a finding.
