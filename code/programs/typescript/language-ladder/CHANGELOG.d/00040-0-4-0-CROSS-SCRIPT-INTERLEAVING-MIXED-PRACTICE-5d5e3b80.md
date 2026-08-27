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

