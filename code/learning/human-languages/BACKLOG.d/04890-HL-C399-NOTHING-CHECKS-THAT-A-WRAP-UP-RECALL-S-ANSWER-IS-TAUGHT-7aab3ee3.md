## HL-C399 — nothing checks that a Wrap-up Recall's answer is taught by its own lesson

**Status: CLOSED (2026-09-19) for the observed regression.** The Malayalam
corpus suite now pins all three parts of the chapter-93 repair: the answer names
only the respectful *you*, it does not reintroduce *we*, and the recall metadata
does not assess the unavailable `ML-LEX-NJAANGAL-01` atom. A corpus-wide prose
inference rule was deliberately not added: bold recall answers include ordinary
English explanations, derived forms, and earlier-walk knowledge, so equating
every emphasized span with a headword would produce thousands of false
positives. The existing block-knowledge closure remains the general structural
gate; the semantic incident now has direct regression coverage.

Found in Malayalam chapter 93, by a **security reviewer**, after the full suite,
`validate` and all twelve gates had passed the file.

### What happened

`ML-C93-angal` was drafted revealing the plural ending on two words the learner
already owned, **ningal** (*you*) and **njangal** (*we*). `validate` rejected
njangal: chapter 71 sits on `SPINE-READ-SIGNS-AND-NOTICES`, which the walk
reaches AFTER `SPINE-DEFINITE-REFERENCE`, so its atom is not available at
chapter 93. The body prose was rewritten and the atom dropped from the
frontmatter.

**The Wrap-up Recall was not.** It went on asking:

> Where else have you been saying that ending? (**On the end of *you* and
> *we***.)

The lesson no longer teaches *we*. A learner reaching that question is asked to
retrieve something the lesson never gave them.

### Why nothing caught it

Everything that could have looked at it was satisfied:

- `validate` checks the **frontmatter** atom lists and the `hl-knowledge`
  comments. The stale claim was in **prose**, and the comment above it was
  already correct.
- The **gates** check structure, modality, narration hashes, shards and
  figures — none reads a recall answer.
- The **full suite** checks banned words, info dumps, forward references, glyph
  coverage and pins. A word in the learner's own language, spelled in English,
  trips none of them.

So the gap is real and general: **a Wrap-up Recall is prose that makes a claim
about what the lesson taught, and no check ties that claim to the lesson's own
atoms.** Any lesson whose scope shrinks in revision can leave one behind.

### Shape of a fix

The recall blocks are regular enough to check mechanically. The answers in
`**bold**` inside a `## Wrap-up Recall` block name things; a check could require
that every headword, romanization and atom-bearing term named there belongs to a
lesson at or before this one on the walk — the same availability computation
`validate` already performs for the frontmatter, pointed at the prose.

Until then the manual rule is narrower and cheap: **when an atom is cut from a
lesson mid-draft, re-read the Warm-up and the Wrap-up Recall before committing.**
Those two blocks make claims about the rest of the file and are the last places
a revision reaches.
