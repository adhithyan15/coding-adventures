## Unreleased — the pre-A1 assessment target, and why Spanish cannot borrow French's

Spanish had `task-shapes/a1.json` and nothing below it, so its pre-A1 rung had no
stated target at all. This adds `assessment-spec.md` (the seven-level contract) and
`task-shapes/pre-a1.json` (the four-skill inventory).

The interesting part is a real asymmetry with French. French points its pre-A1 at
**DILF A1.1**, an official France Éducation international diploma that exists
precisely because the French system wanted a rung below DELF A1. **Spanish has no
counterpart**: the Instituto Cervantes publishes six DELE diplomas, A1 through C2,
and nothing beneath A1 — the *para escolares* variants are age-adapted forms of A1
and A2/B1, not sub-levels. So Spanish follows HL18 §2 and names a clearly labelled
**project-defined** precursor instead of borrowing a target that does not exist.

The inventory is four sections, 25 points each, 100 total:

```
reading    8 min   known-word match (15) + choose-the-reply (10)
listening  7 min   recognise-greeting (15) + catch-the-name (10)
writing   10 min   delayed-recall (5) + dictation (10) + bounded-production (10)
speaking   5 min   greeting-and-identity (10) + familiar-questions (8) + short-request (7)
```

Its pass rule is deliberately **stricter than DELE's own**. DELE groups
reading+writing and listening+speaking, 30/50 per group and 60/100 overall, with no
published per-skill floor — which lets a strong reader carry a weak writer. This rung
requires all four skills to reach 60% independently. The awarding body's rule is still
recorded verbatim; where they disagree, the stricter one governs what the book may
claim, never what the diploma requires.

Scored pre-A1 writing excludes tracing and visible copying — those stay instructional
supports under HL16. Only delayed recall, dictation/transcription and bounded
independent production earn points, which is exactly what the `ES-W00-hola-*` runway
builds toward.

Unknowns stay unknown: audio delivery speed is `null` with a `notPublished` note
rather than a plausible-looking estimate.

`assessment.json` completes the tranche: the machine-readable contract for all
seven rungs, pre-A1 project-defined and A1–C2 on DELE, each naming its four-skill
task inventory, an independent 0.6 threshold per skill, the cumulative HL19 writing
stages for that level, and two timed mocks with rubric and answer key.

Its `task-shapes/`, `mocks/` and `assessment-spec.md#…` paths are **forward
declarations, not evidence** — a path names a required future artifact. That is the
documented contract shape, and `assessment-spec.md` states it explicitly so nobody
reads a filename as proof the mock exists. The `#a2`…`#c2` anchors resolve to real
per-level sections that say "not yet inventoried" rather than to one merged heading.

Spanish's outstanding pre-A1 plan items go **5 → 4**. Corpus-wide, tracks with a
validated four-skill contract go **12 → 13 of 23**, and written task shapes **17 → 18**.

