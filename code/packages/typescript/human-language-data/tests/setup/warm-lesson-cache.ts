// tests/setup/warm-lesson-cache.ts — parse the real corpus ONCE, before any worker starts.
//
// Listed in `vitest.config.ts` as `globalSetup`, which Vitest runs a single
// time in the main process before it spawns the workers. Loading every track
// here writes one cache entry per track (see `src/lesson-cache.ts`), so every
// worker that follows deserializes the corpus instead of each re-parsing it.
//
// Without this the cache still works — the first worker to miss a track parses
// and writes it — but on a cold cache (every CI run: `npm ci` deletes
// `node_modules`, and the cache with it) the first three or four workers would
// all miss at once and parse the same 98 MB side by side, which is the exact
// contention the cache exists to remove.
//
// On a warm cache this costs one content hash of every lesson file (~0.5 s):
// the entries are found, nothing is parsed, nothing is written.
//
// The narration cache is warmed here too, for the same cold-start reason. Two
// files narrate the whole corpus — `narration-cli.test.ts` (the `--check` gate)
// and `narration.test.ts` (the corpus assertions) — and both ask for exactly
// `corpusNarration()`, so one narration here serves both. Without it, on a cold
// CI runner both workers narrated all 7,800-odd chapters side by side, and the
// gate did it inside its own 60 s budget. `corpusNarration` loads the lessons
// through the lesson cache first, so this one call warms both caches.
import { configureLessonCache, suiteLessonCacheOptions } from "../../src/lesson-cache.js";
import { configureNarrationCache, suiteNarrationCacheOptions } from "../../src/narration-cache.js";
import { corpusNarration } from "../../src/narration-cli.js";
import { loadLessons } from "../../src/loader.js";

export default function setup(): void {
  const options = suiteLessonCacheOptions();
  if (!configureLessonCache(options)) return; // Off here (unsafe directory, unknown platform): parse in the tests.
  try {
    if (configureNarrationCache(suiteNarrationCacheOptions())) corpusNarration(options.root);
    else loadLessons(options.root);
  } catch {
    // A lesson the parser rejects must fail the TESTS that read it, with their
    // names and messages, not abort the run here before any test reports. Those
    // tests reproduce this error themselves, because a failed parse is never
    // cached. The same holds for a narration error or a bad chapter policy: the
    // tests that narrate reproduce it, because nothing is cached for it.
  } finally {
    configureNarrationCache(undefined);
    configureLessonCache(undefined);
  }
}
