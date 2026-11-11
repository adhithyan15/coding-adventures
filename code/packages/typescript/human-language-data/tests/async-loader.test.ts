// async-loader.test.ts — faster source I/O must not change the corpus.
//
// The asynchronous route is useful only if it still reads every source byte
// for each cache key, preserves the sorted walk, and refuses the same bad
// inputs as the synchronous loader. The fixture stays small so these are
// behavior tests, not timing tests that depend on the host's disk.

import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  configureLessonCache,
  hostCachePlatform,
  suiteLessonCacheOptions,
} from "../src/lesson-cache.js";
import {
  loadEverything,
  loadEverythingAsync,
  loadLessons,
  loadLessonsAsync,
  loadTrackLessons,
  loadTrackLessonsAsync,
  readLessonSourcesBounded,
} from "../src/loader.js";

let scratch: string;
let root: string;

function lesson(id: string, headword: string): string {
  return [
    "---",
    `id: ${id}`,
    "type: word",
    `headword: ${headword}`,
    "gloss: a test word",
    `concept_tag: test.${id}`,
    "---",
    "",
    `# ${headword}`,
    "",
    "## Warm-up",
    "",
    "Say it out loud.",
    "",
  ].join("\n");
}

function writeLesson(language: string, name: string, source: string): void {
  const dir = join(root, language, "lessons");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, name), source, "utf8");
}

beforeEach(() => {
  scratch = mkdtempSync(join(tmpdir(), "hl-async-loader-"));
  root = join(scratch, "curriculum");
  mkdirSync(root);
});

afterEach(() => {
  configureLessonCache(suiteLessonCacheOptions());
  rmSync(scratch, { recursive: true, force: true });
});

describe("bounded asynchronous lesson loading", () => {
  it("keeps the complete loader result identical on a small authored corpus", async () => {
    writeLesson("spanish", "001-uno.md", lesson("uno", "uno"));
    mkdirSync(join(root, "concepts"), { recursive: true });
    writeFileSync(join(root, "concepts", "taxonomy.json"), '{"version":1,"concepts":{}}');
    mkdirSync(join(root, "core", "sound-tags.d"), { recursive: true });
    writeFileSync(join(root, "core", "languages.json"),
      '{"version":1,"languages":[{"id":"spanish","name":"Spanish","family":"Test","script":"latin","status":"active","bridges":[]}]}');
    writeFileSync(join(root, "core", "spine.json"), '{"version":1,"nodes":[]}');
    writeFileSync(join(root, "core", "sound-tags.d", "_meta.json"),
      `${JSON.stringify({ version: 1 }, null, 2)}\n`);
    writeFileSync(join(root, "core", "sound-tags.d", "spanish.json"),
      `${JSON.stringify({ language: "spanish", tags: [] }, null, 2)}\n`);
    expect(configureLessonCache(undefined)).toBe(false);

    expect(await loadEverythingAsync(root)).toStrictEqual(loadEverything(root));
  });
  it("matches the synchronous track and whole-corpus order without a cache", async () => {
    writeLesson("spanish", "002-dos.md", lesson("dos", "dos"));
    writeLesson("spanish", "001-uno.md", lesson("uno", "uno"));
    writeLesson("french", "001-un.md", lesson("un", "un"));
    mkdirSync(join(root, "german", "lessons"), { recursive: true });
    expect(configureLessonCache(undefined)).toBe(false);

    expect(await loadTrackLessonsAsync("spanish", root)).toStrictEqual(
      loadTrackLessons("spanish", root),
    );
    const all = await loadLessonsAsync(root);
    expect(all).toStrictEqual(loadLessons(root));
    expect(all.map((entry) => entry.language)).toEqual(["french", "spanish", "spanish"]);
    expect(all.map((entry) => entry.realization.headword)).toEqual(["un", "uno", "dos"]);
  });

  it("uses the same content-addressed cache key on a cold and warm read", async () => {
    writeLesson("spanish", "001-uno.md", lesson("uno", "uno"));
    const cacheDir = join(scratch, "cache");
    // The Windows cache accepts only its configured default directory. Give
    // this isolated fixture its own simulated default, as cache-policy tests do.
    expect(configureLessonCache({ root, directory: cacheDir }, hostCachePlatform(cacheDir))).toBe(true);
    const keys: string[] = [];
    const first = await loadLessonsAsync(root, (language, key) => keys.push(`${language}:${key}`));
    expect(keys).toHaveLength(1);
    const syncKeys: string[] = [];
    expect(loadLessons(root, (language, key) => syncKeys.push(`${language}:${key}`))).toStrictEqual(first);
    expect(syncKeys).toEqual(keys);
    expect(await loadLessonsAsync(root)).toStrictEqual(first);

    // A same-size source edit must invalidate; mtime and size are not the key.
    const path = join(root, "spanish", "lessons", "001-uno.md");
    writeFileSync(path, readFileSync(path, "utf8").replace("headword: uno", "headword: une"));
    const editedKeys: string[] = [];
    const edited = await loadLessonsAsync(root, (language, key) => editedKeys.push(`${language}:${key}`));
    expect(edited).toStrictEqual(loadLessons(root));
    expect(edited[0]?.realization.headword).toBe("une");
    expect(editedKeys).not.toEqual(keys);
  });

  it("refuses unsafe track ids and invalid lessons instead of dropping them", async () => {
    await expect(loadTrackLessonsAsync("../outside", root)).rejects.toThrow(/unsafe language id/);
    writeLesson("spanish", "001-bad.md", lesson("not a valid id", "malo"));
    await expect(loadLessonsAsync(root)).rejects.toThrow(/stable hyphenated token/);
  });

  it("keeps file reads bounded, ordered, and drained after a failure", async () => {
    const names = Array.from({ length: 40 }, (_, index) => `${index}.md`);
    let active = 0;
    let maximum = 0;
    const reader = async (path: string): Promise<Buffer> => {
      active++;
      maximum = Math.max(maximum, active);
      await new Promise((resolve) => setTimeout(resolve, 1));
      active--;
      return Buffer.from(path);
    };
    const sources = await readLessonSourcesBounded(root, names, reader);
    expect(sources.map((source) => source.name)).toEqual(names);
    expect(sources.map((source) => source.bytes.toString())).toEqual(
      names.map((name) => join(root, name)),
    );
    expect(maximum).toBeGreaterThan(1);
    expect(maximum).toBeLessThanOrEqual(16);
    expect(active).toBe(0);

    const failingReader = async (path: string): Promise<Buffer> => {
      active++;
      await new Promise((resolve) => setTimeout(resolve, 1));
      active--;
      if (path.endsWith("7.md")) throw new Error("source read failed");
      return Buffer.from(path);
    };
    await expect(readLessonSourcesBounded(root, names, failingReader)).rejects.toThrow(
      /source read failed/,
    );
    expect(active).toBe(0);
  });
});
