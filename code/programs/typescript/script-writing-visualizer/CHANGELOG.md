# Changelog

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
