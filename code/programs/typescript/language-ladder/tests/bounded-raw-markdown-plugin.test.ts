import { join, resolve } from "node:path";
import { describe, expect, it, vi } from "vitest";
import {
  MAX_CONCURRENT_RAW_READS,
  boundedRawMarkdownPlugin,
  createConcurrencyLimiter,
  rawCurriculumMarkdownFile,
} from "../bounded-raw-markdown-plugin.ts";

// The production build failed on Windows with EMFILE because Vite opened every
// `?raw` lesson file at once. These tests pin the three pieces of the fix: the
// limiter really caps in-flight work, the id filter claims only curriculum
// Markdown raw imports, and the module it emits is the one Vite would emit.

const ROOT = resolve("/repo/code/learning/human-languages");
const LESSON = join(ROOT, "latin", "lessons", "LA-C1-salve.md");

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let settle!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    settle = done;
  });
  return { promise, resolve: settle };
}

describe("createConcurrencyLimiter", () => {
  it("never runs more than the limit at once, and runs every task", async () => {
    const limit = createConcurrencyLimiter(3);
    let active = 0;
    let peak = 0;
    const results = await Promise.all(
      Array.from({ length: 40 }, (_, index) =>
        limit(async () => {
          active += 1;
          peak = Math.max(peak, active);
          await new Promise((wake) => setTimeout(wake, index % 4));
          active -= 1;
          return index;
        }),
      ),
    );
    expect(peak).toBe(3);
    expect(results).toEqual(Array.from({ length: 40 }, (_, index) => index));
  });

  it("releases the slot when a task fails", async () => {
    const limit = createConcurrencyLimiter(1);
    await expect(limit(async () => Promise.reject(new Error("unreadable")))).rejects.toThrow(
      "unreadable",
    );
    await expect(limit(async () => "next")).resolves.toBe("next");
  });

  it("rejects a limit that could never admit a task", () => {
    for (const bad of [0, -1, 1.5, Number.NaN]) {
      expect(() => createConcurrencyLimiter(bad)).toThrow(RangeError);
    }
  });
});

describe("rawCurriculumMarkdownFile", () => {
  it("claims raw Markdown imports inside the curriculum root", () => {
    expect(rawCurriculumMarkdownFile(`${LESSON}?raw`, ROOT)).toBe(LESSON);
    expect(rawCurriculumMarkdownFile(`${LESSON}?import&raw`, ROOT)).toBe(LESSON);
  });

  it("leaves every other id to Vite", () => {
    const outside = resolve("/elsewhere/notes.md");
    const escape = `${join(ROOT, "..", "escape.md")}?raw`;
    for (const id of [
      LESSON,
      `${LESSON}?url`,
      `${LESSON}?rawish`,
      `${join(ROOT, "latin", "data.json")}?raw`,
      `${outside}?raw`,
      escape,
      `\0virtual:${LESSON}?raw`,
      "lessons/LA-C1-salve.md?raw",
    ]) {
      expect(rawCurriculumMarkdownFile(id, ROOT), id).toBeNull();
    }
  });
});

describe("boundedRawMarkdownPlugin", () => {
  it("runs before Vite's asset plugin and caps reads by default", () => {
    const plugin = boundedRawMarkdownPlugin({ curriculumRoot: ROOT });
    expect(plugin.enforce).toBe("pre");
    expect(MAX_CONCURRENT_RAW_READS).toBeGreaterThan(0);
    expect(MAX_CONCURRENT_RAW_READS).toBeLessThanOrEqual(256);
  });

  it("emits Vite's raw module, registers a watch, and bounds in-flight reads", async () => {
    const gates = new Map<string, ReturnType<typeof deferred<string>>>();
    let inFlight = 0;
    let peak = 0;
    const readText = vi.fn(async (file: string) => {
      inFlight += 1;
      peak = Math.max(peak, inFlight);
      const gate = deferred<string>();
      gates.set(file, gate);
      const text = await gate.promise;
      inFlight -= 1;
      return text;
    });
    const plugin = boundedRawMarkdownPlugin({
      curriculumRoot: ROOT,
      maxConcurrentReads: 2,
      readText,
    });
    const load = plugin.load as (this: unknown, id: string) => Promise<unknown>;
    const addWatchFile = vi.fn();
    const files = [1, 2, 3, 4, 5].map((n) => join(ROOT, "latin", "lessons", `LA-C${n}.md`));
    const pending = files.map((file) => load.call({ addWatchFile }, `${file}?raw`));

    await vi.waitFor(() => expect(gates.size).toBe(2));
    expect(peak).toBe(2);
    for (const file of files) {
      await vi.waitFor(() => expect(gates.has(file)).toBe(true));
      gates.get(file)!.resolve(`# ${file}\n"quoted" \\ text`);
    }
    const results = await Promise.all(pending);

    expect(peak).toBe(2);
    expect(readText).toHaveBeenCalledTimes(5);
    expect(addWatchFile.mock.calls.map(([file]) => file)).toEqual(files);
    expect(results[0]).toEqual({
      code: `export default ${JSON.stringify(`# ${files[0]}\n"quoted" \\ text`)}`,
      map: { mappings: "" },
      moduleType: "js",
    });
  });

  it("returns null for ids it does not own, without reading", async () => {
    const readText = vi.fn();
    const plugin = boundedRawMarkdownPlugin({ curriculumRoot: ROOT, readText });
    const load = plugin.load as (this: unknown, id: string) => Promise<unknown>;
    await expect(load.call({ addWatchFile: vi.fn() }, `${LESSON}?url`)).resolves.toBeNull();
    expect(readText).not.toHaveBeenCalled();
  });
});
