---
category: Repo policy / workflow reminders
---

# Reinforcement debt is the one pre-A1 criterion that can be cleared without buying new debt, so clear it before authoring vocabulary

HL09 §3.1 is a conjunction of five criteria. Two of them interact, and the
direction matters:

- **Vocabulary** counts distinct headwords, and only `CONTENT_TYPES`
  (`word`, `phrase`) produce one. Every new word lesson also introduces
  two or three atoms, and every atom wants **two** later revisits.
- **Reinforcement** counts atoms at or below the level revisited fewer than
  twice. `practisedAtoms` credits one revisit per **lesson**, not per mention,
  so one review lesson listing eleven atoms discharges eleven debts.

So authoring vocabulary makes reinforcement worse — measured on Telugu, 79 new
headwords would have taken reinforcement from 50 to roughly 130 — while
authoring `review` lessons moves reinforcement and **cannot** move vocabulary,
the atom budget, or the headword count. Clear reinforcement first; it is the
only criterion that is free.

**Before authoring anything for a track, dump its actual defect list.** The
per-track ladder in `report-cli` prints one blocker per track (HL-C427), so a
track with two reads as having one. `measureContinuity(lessons).reinforcement`,
filtered to `revisits < 2` and to atoms whose introducing lesson is at or below
the level, gives the real list with the lesson and sequence of each — and the
shape of that list is what tells you where the review lessons go.

Two gates that only fire on new review lessons, both learned the hard way:

- `delivery: script` belongs to `type: writing` only. `modality-manifest`
  asserts the two sets are equal per track, so a `type: review` script lesson
  carrying it fails. Copy `TE-S167-script-recall-vowels`, not a writing lesson.
- Naming a chapter number in prose is capped per track (HL-C102). A review
  lesson naturally wants to write "chapter 2 gave you…"; write "the name
  exchange gave you…" instead. Chapter numbers move, lesson content does not.
