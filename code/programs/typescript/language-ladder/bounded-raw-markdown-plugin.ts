import { readFile } from "node:fs/promises";
import { isAbsolute, relative, resolve } from "node:path";
import type { Plugin } from "vite";

/**
 * bounded-raw-markdown-plugin — read the lesson corpus a few files at a time.
 *
 * WHY THIS EXISTS
 * ---------------
 * `src/lesson-sources.ts` globs every lesson Markdown file with `?raw`:
 *
 *     import.meta.glob(".../human-languages/*\/lessons/*.md", { query: "?raw" })
 *
 * For each match, Vite's built-in `vite:asset` plugin answers the load with
 *
 *     `export default ${JSON.stringify(await fsp.readFile(file, "utf-8"))}`
 *
 * and the bundler calls that hook for every module it can reach, all at once.
 * Nothing caps how many of those reads are in flight. With ~29,600 lesson
 * files, the production build asks the OS for tens of thousands of open file
 * handles at the same moment:
 *
 *     platform   open-file ceiling           what happened
 *     --------   -------------------------   ---------------------------------
 *     Windows    CRT handle table (~8,192)   EMFILE x 15,738 -> build failed
 *     Linux      `ulimit -n 1024` repro      EMFILE x 21,899 -> build failed
 *     Linux CI   ulimit in the 10^5 range    passes, by headroom alone
 *
 * The build passed on Windows the day before and failed after `main` merged in
 * more lessons: the corpus only grows, so an uncapped fan-out is a failure that
 * was always going to arrive. Raising the OS limit would just move the cliff.
 *
 * WHAT IT DOES
 * ------------
 * This plugin runs BEFORE `vite:asset` (`enforce: "pre"`) and answers exactly
 * the loads that fan out: `?raw` Markdown files inside the curriculum root. It
 * returns the same result Vite would (an `export default` of the file's text, an
 * empty source map, and `moduleType: "js"`), but every read waits for one of
 * `maxConcurrentReads` slots. Anything else (other files, other queries, files
 * outside the curriculum) returns `null`, which hands the id on to Vite's own
 * handling unchanged.
 *
 * Think of a library reading room with 64 seats: everyone still gets their
 * book, they just queue at the door instead of all trying to sit at once.
 */

/** In-flight file reads allowed at once. Far below every platform's ceiling. */
export const MAX_CONCURRENT_RAW_READS = 64;

/** Vite's own test for a `?raw` import: `raw` as a whole query key. */
const RAW_QUERY = /(?:\?|&)raw(?:&|$)/;

/**
 * Run async tasks with at most `limit` of them in flight.
 *
 * A counting semaphore: `active` is the number of running tasks, `waiting` is
 * a FIFO of callbacks for tasks that arrived when every slot was busy. When a
 * task settles (success OR failure, so one bad read cannot leak a slot) it
 * hands its slot straight to the next waiter.
 */
export function createConcurrencyLimiter(
  limit: number,
): <T>(task: () => Promise<T>) => Promise<T> {
  if (!Number.isInteger(limit) || limit < 1) {
    throw new RangeError("concurrency limit must be a positive integer");
  }
  let active = 0;
  const waiting: Array<() => void> = [];

  const release = (): void => {
    const next = waiting.shift();
    if (next === undefined) {
      active -= 1;
    } else {
      next(); // the slot passes directly to the next task; `active` is unchanged
    }
  };

  return async <T>(task: () => Promise<T>): Promise<T> => {
    if (active < limit) {
      active += 1;
    } else {
      await new Promise<void>((acquire) => waiting.push(acquire));
    }
    try {
      return await task();
    } finally {
      release();
    }
  };
}

/**
 * The absolute Markdown path behind a `?raw` module id, or `null` when this
 * plugin should leave the id to Vite.
 *
 *     id                                         result
 *     -----------------------------------------  -------------------------
 *     <root>/latin/lessons/LA-C1.md?raw          <root>/latin/lessons/LA-C1.md
 *     <root>/latin/lessons/LA-C1.md?import&raw   <root>/latin/lessons/LA-C1.md
 *     <root>/latin/lessons/LA-C1.md              null  (not a raw import)
 *     <root>/latin/data.json?raw                 null  (not Markdown)
 *     /elsewhere/notes.md?raw                    null  (outside the curriculum)
 *     <root>/../escape.md?raw                    null  (resolves outside)
 *     \0virtual:thing.md?raw                     null  (virtual module)
 */
export function rawCurriculumMarkdownFile(
  id: string,
  curriculumRoot: string,
): string | null {
  if (id.startsWith("\0")) return null;
  const queryStart = id.indexOf("?");
  if (queryStart < 0 || !RAW_QUERY.test(id.slice(queryStart))) return null;
  const path = id.slice(0, queryStart);
  if (!path.toLowerCase().endsWith(".md") || !isAbsolute(path)) return null;
  const file = resolve(path);
  const fromRoot = relative(resolve(curriculumRoot), file);
  if (fromRoot === "" || fromRoot.startsWith("..") || isAbsolute(fromRoot)) {
    return null;
  }
  return file;
}

export interface BoundedRawMarkdownPluginOptions {
  /** Directory whose `?raw` Markdown imports this plugin serves. */
  readonly curriculumRoot: string;
  /** Reads allowed in flight at once (default {@link MAX_CONCURRENT_RAW_READS}). */
  readonly maxConcurrentReads?: number;
  /** File reader; injectable so tests can observe concurrency. */
  readonly readText?: (file: string) => Promise<string>;
}

export function boundedRawMarkdownPlugin(
  options: BoundedRawMarkdownPluginOptions,
): Plugin {
  const { curriculumRoot } = options;
  const readText = options.readText ?? ((file: string) => readFile(file, "utf-8"));
  const limit = createConcurrencyLimiter(
    options.maxConcurrentReads ?? MAX_CONCURRENT_RAW_READS,
  );

  return {
    name: "language-ladder:bounded-raw-markdown",
    enforce: "pre",
    async load(id) {
      const file = rawCurriculumMarkdownFile(id, curriculumRoot);
      if (file === null) return null;
      // Same watch registration Vite's raw loader performs, so `vite dev`
      // still reloads a lesson when its Markdown is edited.
      this.addWatchFile(file);
      const text = await limit(() => readText(file));
      return {
        code: `export default ${JSON.stringify(text)}`,
        map: { mappings: "" },
        moduleType: "js",
      };
    },
  };
}
