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
import { configureLessonCache, suiteLessonCacheOptions } from "../../src/lesson-cache.js";
import { loadLessons } from "../../src/loader.js";

export default function setup(): void {
  const options = suiteLessonCacheOptions();
  if (!configureLessonCache(options)) return; // Off here (Windows, unsafe directory): parse in the tests.
  try {
    loadLessons(options.root);
  } catch {
    // A lesson the parser rejects must fail the TESTS that read it, with their
    // names and messages, not abort the run here before any test reports. Those
    // tests reproduce this error themselves, because a failed parse is never
    // cached.
  } finally {
    configureLessonCache(undefined);
  }
}
