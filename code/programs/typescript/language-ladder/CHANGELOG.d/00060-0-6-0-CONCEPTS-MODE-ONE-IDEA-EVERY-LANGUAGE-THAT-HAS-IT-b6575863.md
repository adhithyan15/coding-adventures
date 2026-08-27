## 0.6.0 — Concepts mode: one idea, every language that has it

The app could drill letters, and drill lessons. It could not yet do the thing
the curriculum's shared `concept_tag`s exist for: **compare**.

- **New "Concepts" mode.** Canonical concept tags are deliberately identical
  across tracks, which makes them a join key — *gracias / merci / danke /
  धन्यवाद / നന്ദി* are one concept realized eighteen ways. The mode lists every
  concept **two or more languages** share (**39** of them, from **701** lessons)
  and expands each into a side-by-side table.
- **It calls the package's own `languagesForConcept`.** That function has
  shipped since HL01, tested and documented as "what the companion app calls,"
  with **no caller**. This is the caller — and `buildDataset` beside it, so the
  join is the package's tested logic rather than a second implementation that
  could drift from it.
- Each row carries **headword**, **romanization** (only when it differs from the
  headword — for Latin-script tracks the package sets them equal, and repeating
  it is noise), and gloss. The **etymology hooks** follow the comparison, which
  is where they earn their keep: *gracias* ← *gratia* "favour", *merci* ←
  *mercēs* "wages, price", *danke* ← *denken* "to think", *спасибо* ← *спаси
  Бог* "God save you". One courtesy, four unrelated ideas.
- **Concepts only one language realizes are dropped.** Not a special case for
  namespaced tags — a language floor removes them naturally, because a card with
  nothing to compare against isn't a card.

### Prerequisite gating (the other half)

`scheduler.ts` is generic over a numeric index and has no idea that "the
preterite of *comer*" presupposes *comer*. New `concepts.ts` supplies that
knowledge before the scheduler ever sees the pool:

- A lesson unlocks when every id in its `prerequisites` has been **seen**.
- **Unknown prerequisite ids count as satisfied.** A curriculum typo, or a
  prerequisite pointing at an unwritten lesson, should degrade to "shown
  slightly early" — never to "silently unreachable forever," which is the
  failure nobody notices.
- **The gate fails open.** `unlockedOrAll` falls back to every lesson if the
  gated pool is empty (a prerequisite cycle would do it), because practice
  stalling completely is worse than practising something early. Tested with an
  actual cycle.
- "Seen" is computed from **review history** (`reps`/`lapses`/`box`), never from
  `dueAtSession` — the 0.5.0 bug that reported the whole curriculum as started
  after one reload.
- `reviewTargets` maps `reviews_of` onto scheduler indices and is tested, but
  **nothing calls it yet** — it is groundwork for having the app follow the
  syllabus's own "answering this should refresh those" instead of waiting on a
  Leitner interval. Said plainly because an earlier draft of this entry claimed
  the app already did that, which was false. (`ConceptCard.namespaced` is
  likewise computed and not yet rendered.)

### The bug this shipped with, and how it was caught

The first cut gated the **pick**: choose from the rotation, and if the chosen
lesson is locked, substitute the first unlocked one. That is wrong in a way that
is invisible by inspection — the same pick is rejected every turn, so the
substitute is served over and over. A review simulation of the real curriculum
measured it serving **one Arabic lesson 34 times in 40**, wiping out both the
0.5.0 rotating cursor and cross-language interleaving.

The fix gates the **pool**: `nextDue` now takes an `accept` predicate and skips
locked indices *during* the scan, so the cursor keeps advancing; the
nothing-due fallback runs `pickNext` over the unlocked states rather than
grabbing `open[0]`.

The regression test took three attempts to make honest, which is worth
recording. Versions 1 and 2 **passed against the broken implementation** — the
fixture's chain order happened to match its pool order, so the rotation landed
on unlocked lessons anyway. Only a fixture whose dependency order runs
*counter* to pool order (as the real curriculum's does) reproduces it. The test
now fails on the broken version and passes on the fixed one, and both were
verified by actually injecting the old code.

### Notes

- Tests: **97**, up from 75 — `tests/concepts.test.ts`, including checks against
  the **real curriculum** (every card genuinely spans ≥2 languages; gating opens
  a non-empty but non-total pool on a fresh profile; everything is reachable
  once everything is seen).
- `Lesson` gained `romanization`, `script` and `etymologyHook`, which the cards
  need. `tests/lessons.test.ts` now builds fixtures through a defaulting helper
  so adding a field doesn't touch every test.
- **Verified in a browser**, not just built: the 0.5.0 `process is not defined`
  bug was a successful build that died on load, and only a real page load
  catches that class of error. Both new deep imports (`parse.ts`, `queries.ts`)
  are pure modules; console is clean.
- **Known cost, unchanged:** the eager `import.meta.glob` still inlines every
  lesson, so the bundle is ~1.72 MB (542 kB gzipped). Concepts mode makes the
  parsed corpus more valuable, not less, but a lazy glob or a build-time index
  remains the right fix. Deferred, not hidden.

