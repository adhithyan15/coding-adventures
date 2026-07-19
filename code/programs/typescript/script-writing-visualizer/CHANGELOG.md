# Changelog

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
