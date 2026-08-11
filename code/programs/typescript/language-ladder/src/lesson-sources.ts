/**
 * The lesson-source loader map, kept OUT of the eager chunk (HL-C110).
 *
 * `import.meta.glob` compiles to an object literal with one entry per matching
 * file: the full path as a key, and an arrow function that dynamically imports
 * the batch it lives in. The lesson BODIES are already lazy — that is what the
 * `lessons-*` chunks are — but the map itself is code, and wherever it is
 * imported, it lands.
 *
 * It landed in the eager chunk, because `main.ts` built its id set at module
 * load. At 1,793 lessons that map was ~27 kB of paths and wrappers, and it grew
 * by roughly 222 bytes per lesson added — enough that five lessons moved the
 * eager chunk 1.1 kB toward a 500 kB ceiling it had already broken once.
 *
 * So the map lives here alone, and `lessons.ts` reaches it through `import()`.
 * Nothing else may import this module statically; doing so puts all of it back.
 */
export const LESSON_SOURCE_LOADERS = import.meta.glob(
  "../../../../learning/human-languages/*/lessons/*.md",
  { query: "?raw", import: "default" },
) as Record<string, () => Promise<string>>;
