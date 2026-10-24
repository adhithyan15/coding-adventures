import { readFile } from "node:fs/promises";
import { isAbsolute, relative } from "node:path";
import type { Plugin } from "vite";

/**
 * Throttle `vite build`'s reads of the lesson corpus.
 *
 * WHY THIS EXISTS. `src/lesson-sources.ts` globs every lesson Markdown file
 * with `{ query: "?raw" }`. At build time Rolldown resolves all ~39,000 of
 * those imports and calls every module's `load` hook as fast as it can, and
 * Vite's built-in `vite:asset` plugin answers each `?raw` load with a bare
 * `fsp.readFile`. Nothing bounds how many are open at once.
 *
 * macOS and Linux shrug that off. The Windows CI runner does not: `vite build`
 * died with tens of thousands of
 *
 *     [plugin vite:asset] Error: EMFILE: too many open files, open '...\IT-C277-istante.md'
 *
 * on both vite 8.1.5 (#17522) and 8.3.4 (#17972). The app's own runtime loader
 * already had the same problem and the same cure (`mapWithConcurrency` with
 * `LESSON_LOAD_CONCURRENCY = 64` in `src/lessons.ts`, #17925); this is that
 * cure applied one layer down, to the bundler.
 *
 * WHAT IT DOES. It runs before `vite:asset` (`enforce: "pre"`) and claims only
 * `.md?raw` ids that live under the curriculum root. For those it does exactly
 * what `vite:asset` would — read the file as UTF-8 and return
 * `export default "<contents>"` — except that at most `limit` reads are in
 * flight. Every other id falls through untouched (`return null`).
 *
 * It is build-only (`apply: "build"`): the dev server loads lessons on demand,
 * one request at a time, and keeping Vite's own handler there keeps its
 * watch/HMR behaviour exactly as it was.
 *
 *     id                                             handled here?
 *     ---------------------------------------------  -------------
 *     <root>/italian/lessons/IT-C277-istante.md?raw  yes
 *     <root>/italian/lessons/IT-C277-istante.md      no  (no ?raw)
 *     <root>/data/scripts/latin.json?raw             no  (not .md)
 *     /elsewhere/README.md?raw                       no  (outside root)
 *     \0virtual:thing?raw                            no  (virtual id)
 */

/** How many lesson files the build may hold open at once. Mirrors `LESSON_LOAD_CONCURRENCY`. */
export const RAW_LOAD_CONCURRENCY = 64;

/** Same test `vite:asset` uses for its raw branch: `?raw` or `&raw` as a whole query key. */
const RAW_QUERY = /(\?|&)raw(?:&|$)/;

export interface RawLoadThrottleOptions {
  /** Absolute path of `code/learning/human-languages`; only files under it are claimed. */
  readonly curriculumRoot: string;
  /** Maximum reads in flight. Defaults to {@link RAW_LOAD_CONCURRENCY}. */
  readonly limit?: number;
  /** Injected for tests; defaults to `fs.promises.readFile(path, "utf8")`. */
  readonly readText?: (path: string) => Promise<string>;
}

/**
 * A counting semaphore with direct hand-off.
 *
 * When a holder releases while someone is queued, the slot passes straight to
 * the next waiter instead of being returned to the pool. Returning it first
 * would open a gap in which a brand-new caller could grab the slot before the
 * woken waiter runs, briefly putting `limit + 1` reads in flight.
 */
function createSlots(limit: number): <T>(task: () => Promise<T>) => Promise<T> {
  let active = 0;
  const waiting: Array<() => void> = [];

  const acquire = (): Promise<void> => {
    if (active < limit) {
      active += 1;
      return Promise.resolve();
    }
    return new Promise((resolve) => waiting.push(resolve));
  };

  const release = (): void => {
    const next = waiting.shift();
    if (next) next();
    else active -= 1;
  };

  return async (task) => {
    await acquire();
    try {
      return await task();
    } finally {
      release();
    }
  };
}

/** True when `file` is `root` itself or somewhere beneath it. */
function isWithin(root: string, file: string): boolean {
  const rel = relative(root, file);
  return rel === "" || (!rel.startsWith("..") && !isAbsolute(rel));
}

export function rawLoadThrottlePlugin(options: RawLoadThrottleOptions): Plugin {
  const limit = options.limit ?? RAW_LOAD_CONCURRENCY;
  if (!Number.isInteger(limit) || limit < 1) {
    throw new RangeError(`rawLoadThrottlePlugin: limit must be a positive integer, got ${limit}`);
  }
  const readText = options.readText ?? ((path: string) => readFile(path, "utf8"));
  const withSlot = createSlots(limit);

  return {
    name: "language-ladder:raw-load-throttle",
    apply: "build",
    enforce: "pre",
    async load(id) {
      if (id.startsWith("\0") || !RAW_QUERY.test(id)) return null;
      const file = id.split(/[?#]/, 1)[0]!;
      if (!file.endsWith(".md") || !isWithin(options.curriculumRoot, file)) return null;

      const text = await withSlot(() => readText(file));
      return {
        code: `export default ${JSON.stringify(text)}`,
        map: { mappings: "" },
        moduleType: "js",
      };
    },
  };
}
