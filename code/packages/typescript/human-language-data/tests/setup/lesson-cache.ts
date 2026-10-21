// tests/setup/lesson-cache.ts — turn the parsed-lesson cache on in every test file.
//
// Listed in `vitest.config.ts` as a `setupFiles` entry, so it runs inside each
// worker, in the same module graph as the test file it precedes. That matters:
// the cache's on/off switch is module state in `src/lesson-cache.ts`, and
// Vitest gives every test file a fresh copy of that module. A switch thrown in
// some other process (or in a previous file's module graph) would not be seen.
//
// Only the real curriculum root is cached. Fixture corpora built in temp
// directories are parsed exactly as before — see `lessonCacheFor`.
//
// This file deliberately imports nothing but `lesson-cache.ts`, which itself
// imports only Node built-ins. Importing `loader.ts` just to call
// `defaultCurriculumRoot()` would load the loader's whole module graph into
// every test file, including the many that never touch the corpus. So the root
// comes from `suiteLessonCacheOptions()`, and `tests/lesson-cache.test.ts`
// asserts that it matches `defaultCurriculumRoot()` — if the two ever drift,
// the cache would silently stay off and the suite would quietly be slow again.
import { configureLessonCache, suiteLessonCacheOptions } from "../../src/lesson-cache.js";

configureLessonCache(suiteLessonCacheOptions());
