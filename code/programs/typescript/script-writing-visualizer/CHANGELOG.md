# Changelog

## 0.9.4 — the session view-model (HL03 phase 6, slice 6a)

- `src/sessionplan.ts` — the seam that assembles the four engine modules into
  one session, with no DOM (the UI slice renders what this returns):
  `planSession(current, covered, lessons, activeCount)` returns the **teaching
  pass** (the current concept swept across the active chain, with connections)
  and the **review pass** (the covered grid the quiz draws from).
- `applyAnswer(progress, cell, correct, session, chosenKey?)` threads the state
  that makes review adaptive: a hit **promotes** the cell (comes back later), a
  miss **demotes** it (box 0, due now) and logs the confusion — so the next
  `pickNext` leans on what was just missed. Immutable; `initProgress` seeds it.
- Controls bite (fault-injected): a review pass that only covered the current
  concept fails the "spans every covered concept" test; a no-op `applyAnswer`
  fails the "missed cell outweighs a mastered one" test. Verified against the
  real curriculum (COURTESY-THANKS teaches across all ten, reviews alongside
  GREETING-HELLO). Pure, deterministic. Next: slice 6b renders it.

## 0.9.3 — the mistakes store (HL03 phase 5)

- `src/mistakes.ts` — records each quiz answer and, crucially, WHAT THE LEARNER
  CHOSE when wrong (the confusion — e.g. picking the French cognate's meaning
  for the Spanish word). `recordAnswer` appends immutably; `demote` feeds a miss
  back into the SRS (box→0, due now, lapse++) so the item resurfaces sooner in
  `pickNext`; `confusions` rolls the wrong answers into ranked "what you keep
  mixing up" pairs.
- Grounded: a confusion only ever appears if the learner actually made it — no
  pair is inferred. Pair keys use `JSON.stringify`, not delimiter-joining, so an
  id containing a comma can't collapse two distinct confusions into one.
- Controls bite (fault-injected): a no-op demote fails the "missed cell
  resurfaces" test (its draw weight must jump above a mastered cell's); a
  fabricated pair fails the "nothing invented" control. Pure, deterministic, no
  I/O — the caller passes the session index in.
- This completes the pure-logic layers of HL03 (phases 2–5). Next: phase 6, the
  UI that unifies the four modes into one curriculum-driven session.

## 0.9.2 — the SRS-weighted draw (HL03 phase 4, part 2 — quiz complete)

- Extends `src/quiz.ts` with the randomised cumulative quiz's draw:
  `pickNext(grid, states, session, rng)` selects a cell from the covered grid
  weighted by `cellWeight` — never-seen cells rank high, DUE cells rank higher
  the more overdue / lower-box / lapsed they are (the missed material review
  exists for), and not-yet-due cells sink to a floor so review stays
  interleaved. Per-cell Leitner state (`QuizState`, keyed by `cellKey`) reuses
  scheduler.ts's box/interval math. Deterministic via a seeded LCG (`makeRng`);
  the app never depends on `Math.random`.
- Two controls, both verified by injection: over many draws the sample spans
  MULTIPLE concepts AND languages (a collapsed draw fails); and the draw biases
  toward a missed/overdue cell over a mastered one by a wide margin (injecting
  uniform weighting fails it). This is the primary review mechanism from HL03 —
  "what is 5 in Telugu? 12 in Latin?" — now complete and pure.

## 0.9.1 — the covered grid (HL03 phase 4, part 1)

- `src/quiz.ts` — `coveredGrid(covered, lessons, activeCount)` enumerates every
  (concept × language) cell the learner has studied, each tied to the real
  lesson that answers it. This is the pool the randomised cumulative quiz will
  draw from ("what is 5 in Telugu? 12 in Latin?").
- Built by **reusing the teaching sweep**, not re-deriving the concept→language
  join: a cell exists exactly where a covered concept's sweep has a stop in an
  active language — so the review side can only ever ask about a (concept,
  language) the teaching side actually presents. Deterministic (concepts sorted,
  then chain order, then chapter/id). Plus `conceptsIn` / `languagesIn` and a
  stable `cellKey` for the SRS to track state per item.
- Verified against the real curriculum: COURTESY-THANKS covers all ten chain
  languages; two covered concepts interleave across both concepts and many
  languages. Controls bite — mislabelling a cell fails the grounding test, and
  collapsing the grid to one concept fails the interleave control. Pure, no UI.
- Next (part 2): the SRS-weighted `pickNext` draw over this grid.

## 0.9.0 — the session orchestrator: sweep + grounded connections (HL03 phase 3)

- `src/session.ts` — `buildSession(concept, lessons, activeCount)` takes the
  teaching sweep (phase 2) and annotates each stop with the **connections back
  to earlier languages in the sweep**: where two languages' lessons for the
  concept share an etymological root, the link is surfaced. This is the payoff
  of interleaving — meeting "thank you" in Telugu right after Kannada and Hindi,
  and being shown all three carry the Sanskrit root *dhanya*.
- **The grounding rule, enforced in code**: a connection exists *iff* the two
  stops' lessons literally share a root string (from `lesson.roots`). Nothing is
  inferred or invented; the reported `sharedRoots` is the exact set intersection,
  sorted. Connections always point backward in chain order.
- Verified against the real curriculum: Kannada and Telugu "thank you" link back
  to Hindi via `dhanya`. Controls bite — asserting a connection without a shared
  root fails the grounding test; over-reporting (union instead of intersection)
  fails the "never from thin air" control; a concept sharing no roots surfaces
  no link. Pure, deterministic, no UI.

## 0.8.1 — carry etymological roots on each lesson (HL03 phase 3 prerequisite)

- `Lesson` now carries `roots: string[]` — the etymological roots a lesson cites
  (e.g. `["bonus", "dies"]`). `toLesson` maps them from the frontmatter the same
  way it maps `prerequisites`; the human-language-data parser already extracted
  them, they just were not threaded into the app's `Lesson`.
- This is the **join key for cross-language connections** (the next phase): two
  lessons in different languages that share a root are etymologically linked.
- Tests: roots parse through (a lesson citing none gets `[]`), and — against the
  real curriculum — the Sanskrit root `dhanya` is carried by lessons in more
  than one chain language (Hindi/Kannada/Telugu). Both fail if roots aren't
  plumbed, confirmed by injection.

## 0.8.0 — the language chain and the teaching sweep (HL03 phase 2)

- First implementation piece of the unified language-learning app
  ([HL03](../../../specs/HL03-unified-language-learning-app.md)): a pure
  `sequence.ts` module encoding the fixed **language chain**
  (Spanish → Latin → French → German → Arabic → Hindi → Tamil → Kannada →
  Telugu → Malayalam) and the **teaching sweep** — for one concept, the active
  languages that teach it, walked in chain order.
- `teachingSweep(concept, lessons, active)` filters to the concept, restricts to
  the active chain prefix, skips languages that do not teach it, and orders the
  result by the chain (never by input order). `sweepableConcepts` lists concepts
  in book order (earliest chapter first). No UI — sequencing logic only.
- Verified against the real curriculum: `GREETING-HELLO` sweeps all ten
  languages in exact chain order. Every honesty check is paired with a control
  that fails on broken input — and writing this caught a redundant active-filter
  that would have made the "only active languages" test vacuous; removed so the
  test can actually fail.

## 0.7.1 — each script's at-a-glance identification signature

- Every script now carries a **`signature`** — the one visual feature that gives
  it away at a glance (Devanagari's head-line; Gujarati being Devanagari with
  that line erased; Arabic's joined right-to-left ribbon vs Hebrew's blocky
  separate letters; Cyrillic's Я/Ж/Д tells; Chinese's dense square blocks).
- Added to all seven script data files (`data/scripts/*.json`) and to the
  `ScriptData` type; a test asserts every script ships a non-empty signature.
- Each signature was written **against the rendered font**, not from memory —
  the same verify-by-looking discipline the stroke data uses. This is the data
  backbone for a future "spot the script" identification mode.

## 0.7.0 — read the letter's TRUE shape out of the font

- **`src/truetype.ts`** — a zero-dependency TrueType reader: table directory,
  `cmap` (formats 4 and 12), `loca`, `glyf`, simple and composite glyphs, the
  delta-encoded coordinate flags, and the on-curve midpoint TrueType implies
  between consecutive off-curve points. Outlines come back in font units
  (y-up, baseline 0); the renderer applies one `scale(1,-1)`.
- **Why not hand-drawn SVG paths.** A subtly wrong ண looks fine to anyone who
  cannot already read Tamil — the entire audience — so the error would ship as
  the lesson. Extracting from the vendored font makes shapes correct by
  construction and keeps them identical to what the app renders text with.
- **Hostile input is bounded.** Every count and offset in a font file is
  attacker-controlled if this is ever pointed at an untrusted font, and it runs
  in the browser. `cmap` ranges clamp to U+10FFFF; a single decrementing budget
  bounds total mapping ITERATIONS across both cmap readers (capping the map's
  size alone is not enough — re-mapping groups and format 4's BMP-bounded keys
  both cost work without growing it); a component budget bounds composite
  FAN-OUT, which the depth cap does not (N components at each of 6 levels is
  N⁶ visits — minutes of frozen main thread from a 632-byte file);
  non-ascending contour end points and scaled components are refused rather
  than drawn wrong.
- **Tests rasterise the font** — flatten the quadratics, scan-convert with the
  non-zero winding rule — so shape assertions are checked against what the
  glyph actually looks like. **The raster window is derived from the glyphs'
  own bounding boxes and the rasteriser throws if a glyph would be clipped.**
  A second guard checks the metric's INPUT: the final-stroke measure reports
  its sample count, and the assertion requires samples before believing the
  answer. Without it the measure anchored on the top bar — which overhangs the
  final vertical — collected nothing, and `Math.max([]) - Math.min([])` is
  `-Infinity`, which satisfies any upper bound. It measured nothing and
  reported agreement.
  The window guard exists because a hard-coded window (x ≤ 1030, against ண's true
  extent of 1631) silently amputated 37% of the letter and produced a
  confident, wrong description of its final stroke. A clipped raster does not
  look like an error; it looks like a letter.

## 0.6.1 — Tamil and Gujarati join the script list

- **Tamil** is new: `data/scripts/tamil.json` ships with the first handwriting
  lessons for **any Dravidian language** (`TA-W01`–`W04`). 11 letters and 4
  marks, `complete: false`.
- **Gujarati was already there and simply never wired in.** `gujarati.json` has
  existed since the Gujarati track was authored, but `SCRIPTS` in `src/data.ts`
  listed five scripts while `data/scripts/` held six. Both are now included, so
  Browse and Practice cover **seven** scripts.
- No logic changed — two imports and two array entries. `tests/core.test.ts` uses
  `arrayContaining`, so the script list is not pinned by count.

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

## 0.5.0 — Lessons mode: the whole curriculum, and a memory that survives reloads

The app could already schedule you. It could not **remember** you, and it had
never read a single lesson. Both are fixed.

- **New "Lessons" mode** drills the **written curriculum** — all **679 lessons
  across 18 languages** — instead of only script letters. It reads them from
  `@coding-adventures/human-language-data`, the package that has always shipped
  `frontmatter.ts` / `loader.ts` / `parse.ts` / `queries.ts` for exactly this
  purpose and had **zero consumers** until now.
- **Progress now persists** (`src/progress.ts`). Previously there was no storage
  layer at all, so every Leitner box reset on reload and nothing was ever really
  "tracked". State is saved to `localStorage` keyed by **lesson id**, never by
  array index — indices shift every time a lesson is added, and saving by
  position would silently reattribute your progress to the wrong lesson. Adding
  a lesson now simply means one more unseen item.
- **Cross-language interleaving comes free.** `interleave.buildPool` already
  round-robins across groups for scripts; grouping lessons by language and
  feeding it the same way yields Arabic → Bengali → French → German → Gujarati →
  Hindi in consecutive reviews. A **rotating cursor** walks that order, which
  also fixes the obvious failure mode: box 0/1 fall due again after one session,
  so a scan-from-the-front picker would hand you the lesson you just answered,
  forever.
- **`scheduler.ts`, `interleave.ts` and `drill.ts` are unchanged.** They are
  generic over a numeric index and never needed to know what an item is — which
  is precisely why lessons could reuse them. The new *logic* is pure and tested
  (`lessons.ts`, `progress.ts`, including the `nextDue` cursor scan); the only
  impure edges are a tiny `StorageLike` port and `main.ts`'s DOM shell, which
  remains untested as before.
- **Defensive loading.** Saved state is untrusted input (hand-edited,
  half-written by another tab, left over from an older build). Every field is
  validated, unknown ids are treated as fresh, `__proto__`/`constructor` keys are
  skipped, the item map is `Object.create(null)`, and a throwing or absent
  `localStorage` (Safari private mode, quota) degrades instead of crashing.
- Tests: **67**, up from 57 — `tests/lessons.test.ts` and `tests/progress.test.ts`.

### Known cost

`import.meta.glob(…, { eager: true })` inlines the full text of all 679 lesson
files, so the bundle is ~1.56 MB (480 kB gzipped) and every startup parses them
all — even in Browse mode, which doesn't need them. Only the frontmatter
survives parsing; the bodies are discarded. A lazy glob or a build-time JSON
index would fix both; deferred rather than hidden.

### Three bugs worth recording

- **`process is not defined` at startup.** Importing the package's barrel
  (`index.ts`) pulled `cli.ts` and `loader.ts` — `process`, `node:fs` — into the
  browser bundle. The build *succeeded* and the app then died on load with a
  blank page. Fixed by deep-importing the pure module
  (`.../src/parse.ts`). Caught only by opening the app in a browser; no test or
  build step would have found it.
- **`vitest.config.ts` does not inherit `vite.config.ts`'s `server.fs.allow`.**
  The lesson glob reaches outside the package root, so tests failed with "Denied
  ID" until the same allowance was declared in both configs.
- **The "don't persist untouched items" guard failed open after one reload.**
  It tested `dueAtSession <= 0`, but fresh items are seeded with the *current*
  session, so from session 1 onward every unseen lesson looked touched: the
  payload grew from ~100 bytes to ~48 kB and the "started" count reported the
  whole curriculum. Now keyed on review history (`reps`/`lapses`/`box`) only,
  with a reload round-trip test that would have caught it — the original test
  only covered session 0, the one case where the bug was invisible.

## 0.4.0 — Cross-script interleaving ("Mixed") practice

- **New "Mixed (all scripts)" practice scope** alongside "This script": Practice
  can now **interleave letters from every script in one session** — HL02's
  interleaving principle ("mixing forces discrimination and transfers better").
  The scheduler picks the next due item across the whole combined pool, so a
  Cyrillic prompt is followed by a Hebrew one, then Devanagari, and so on;
  distractors always come from the **target's own script**. Mastery reads across
  the full pool (e.g. "mastered N / 128").
- **New pure module `src/interleave.ts`** — `buildPool(counts)` lays every letter
  of every script into one **round-robin-interleaved** pool (letter 0 of each
  script, then letter 1 of each, …) so mixing starts on the first pass; the
  generic scheduler drives it unchanged. **6 new unit tests (42 total)** incl. an
  integration proving the scheduler alternates scripts and resurfaces a missed
  letter amid the others.
- UI: a scope toggle in Practice; the per-script tabs hide during a mixed
  session; the prompt shows a small script tag. Still zero runtime deps.

## 0.3.0 — Spaced-repetition scheduler wired into Practice

- **New pure module `src/scheduler.ts`** — a Leitner / SM-2-lite scheduler
  measured in **sessions** (no wall-clock, no `Date`), the "core module" of HL02.
  Each item tracks a Leitner box, `dueAtSession`, lapses, and reps; a correct
  answer promotes the box and expands the interval (1 → 3 → 7 → 15 → 30 sessions),
  a wrong answer drops it to box 0 (due again immediately). `pickNext` returns the
  most-overdue item deterministically (ties → fewest reps → lowest index), falling
  back to the soonest-due so practice never stalls.
- **Practice mode now uses it**: instead of a random letter each question, the
  **scheduler chooses** which letter to ask, so missed letters resurface soon and
  mastered ones fade back — real spaced repetition. Each answer advances the
  session clock and feeds `review`. A **"mastered N / total"** read-out joins the
  score line. (Randomness stays only in distractor choice + answer position.)
- **14 new unit tests (36 total)** covering promotion/expansion, lapse-reset,
  `pickNext` ordering + tie-breaks + the never-stall fallback, immutability, and
  an integration check (a correct streak masters an item; a miss resurfaces).

## 0.2.0 — Practice mode (recall drill)

- **New "Practice" mode** alongside "Browse": a recall drill that shows a
  letter's **sound** and asks the learner to pick the matching **glyph** from
  four options, then reveals right/wrong, shows the answer's decomposition, and
  tracks a running **score** (correct / total / %). Recognition builds reading;
  recall (sound → glyph) is the harder second half.
- **Confusable distractors**: wrong answers are drawn from the same script and
  ranked by confusability (same role / same false-friend status rank higher), so
  the choices are meaningfully hard rather than random noise.
- **New pure module `src/drill.ts`** — `buildDrillQuestion`, `confusabilityOrder`,
  `checkAnswer`, and immutable scoring (`record`/`accuracy`). All randomness is
  **injected by the UI** (target, distractor pick, answer position), so the core
  stays deterministic; **10 new unit tests** (22 total) incl. edge cases
  (small inventories, sloppy choosers, clamping) and a real-data check.
- UI toggle wired in `main.ts` with vanilla DOM (still **zero runtime deps**);
  `main.ts` holds the only `Math.random`.

## 0.1.0 — HL02 MVP: break a script apart and write it

- **New app** (`script-writing-visualizer`): the companion "how to write it"
  surface for the Human Languages curriculum. Renders each non-Latin letter with
  its **component pieces**, a numbered **stroke order**, and a **false-friend**
  badge, for pen-and-paper practice.
- **Reads the canonical script data directly** from
  `code/learning/human-languages/data/scripts/*.json` (no copy — the app cannot
  drift from the curriculum). Ships Cyrillic, Hebrew, Chinese, Arabic, Devanagari.
- **Pure core** (`src/core.ts`) covered by unit tests, including an integration
  check against the real curriculum data (every letter has a glyph + pieces +
  stroke order; Cyrillic flags в/р/с/н as false friends).
- Framework-free vanilla-DOM UI; zero runtime dependencies.
- **Scope:** v1 is read + decompose only (no handwriting capture, no scheduler
  yet) — the first slice of the `HL02` spec.
