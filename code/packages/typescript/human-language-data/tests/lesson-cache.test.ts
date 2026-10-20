// lesson-cache.test.ts — the parsed-corpus cache must be invisible except in speed.
//
// `src/lesson-cache.ts` lets the test suite parse each track's lessons once and
// read the result back everywhere else. That is only acceptable if no test can
// tell the difference, so this file pins the four ways a cache goes wrong:
//
//   HIT         a stored entry is actually used (otherwise it saves nothing)
//   STALE       an edited lesson is never answered from the old entry
//   CORRUPT     a truncated, flipped, empty or foreign file is a miss, not data
//   RACE        concurrent writers never expose a half-written entry
//
// plus the security boundary from the module header: entries are only read
// from a private directory, from private regular files, never through a link.
//
// Every case builds its own fixture curriculum and its own cache directory in a
// temp dir, and points the process-wide configuration at them for the duration
// of the case — so nothing here touches the real corpus's cache.

import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  configureLessonCache,
  decodeEntry,
  defaultLessonCacheDirectory,
  directoryDigest,
  encodeEntry,
  lessonCacheFor,
  lessonCacheKey,
  openLessonCache,
  privatelyOwned,
  suiteLessonCacheOptions,
  type LessonCache,
} from "../src/lesson-cache.js";
import { defaultCurriculumRoot, loadLessons, loadTrackLessons } from "../src/loader.js";
import { parseLesson, type ParsedLesson } from "../src/parse.js";

const here = dirname(fileURLToPath(import.meta.url));
const POSIX = typeof process.getuid === "function";

let scratch: string;
let root: string;
let cacheDir: string;

/** One small, valid lesson. `chapter` is omitted on purpose in some: NaN must survive. */
function lesson(id: string, headword: string, chapter?: number): string {
  return [
    "---",
    `id: ${id}`,
    "type: word",
    `headword: ${headword}`,
    "gloss: a test word",
    `concept_tag: test.${id}`,
    ...(chapter === undefined ? [] : [`chapter: ${chapter}`]),
    "sounds: [a, b]",
    "---",
    "",
    `# ${headword}`,
    "",
    "## Warm-up",
    "",
    "Say it out loud.",
    "",
    "## Wrap-up recall",
    "",
    `What is ${headword}?`,
    "",
  ].join("\n");
}

function writeLesson(language: string, file: string, source: string): void {
  const dir = join(root, language, "lessons");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, file), source, "utf8");
}

/** What `loadTrackLessons` must return, computed with no cache anywhere near it. */
function freshParse(language: string): ParsedLesson[] {
  const dir = join(root, language, "lessons");
  return readdirSync(dir)
    .sort()
    .filter((file) => file.endsWith(".md"))
    .map((file) => parseLesson(readFileSync(join(dir, file), "utf8"), language));
}

/** Entries (not temp files) currently in the cache directory. */
function entries(): string[] {
  return readdirSync(cacheDir).filter((name) => name.endsWith(".v8")).sort();
}

function activeCache(): LessonCache {
  const cache = lessonCacheFor(root);
  if (cache === undefined) throw new Error("cache did not turn on for the fixture root");
  return cache;
}

/** The key `loadTrackLessons` will compute for `language`, recomputed independently. */
function keyFor(language: string): string {
  const dir = join(root, language, "lessons");
  const files = readdirSync(dir)
    .sort()
    .filter((file) => file.endsWith(".md"))
    .map((name) => ({ name, bytes: readFileSync(join(dir, name)) }));
  return activeCache().key({ language, script: undefined, files });
}

beforeEach(() => {
  scratch = mkdtempSync(join(tmpdir(), "hl-lesson-cache-"));
  root = join(scratch, "curriculum");
  cacheDir = join(scratch, "cache");
  mkdirSync(root);
});

afterEach(() => {
  // Back to whatever the suite's setup file configured, so the next file in
  // this worker sees the real-corpus cache exactly as it would have.
  configureLessonCache(suiteLessonCacheOptions());
  rmSync(scratch, { recursive: true, force: true });
});

describe("the entry format", () => {
  it("round-trips parsed lessons exactly, including the NaN of an unset chapter", () => {
    const lessons = [
      parseLesson(lesson("one", "uno", 3), "spanish"),
      parseLesson(lesson("two", "dos"), "spanish"),
    ];
    expect(Number.isNaN(lessons[1]!.realization.chapter)).toBe(true);
    const decoded = decodeEntry(encodeEntry(lessons));
    // `toStrictEqual` treats NaN as equal to NaN and checks prototypes, so this
    // is the "no test can tell the difference" property for real parser output.
    expect(decoded).toStrictEqual(lessons);
    expect(Number.isNaN(decoded![1]!.realization.chapter)).toBe(true);
  });

  it("refuses anything short of a complete, matching entry", () => {
    const good = encodeEntry([parseLesson(lesson("one", "uno"), "spanish")]);
    const flipped = Buffer.from(good);
    flipped[flipped.length - 3]! ^= 0x20;
    const wrongMagic = Buffer.from(good);
    wrongMagic[0] = 0x00;
    expect(decodeEntry(good)).toHaveLength(1);
    expect(decodeEntry(Buffer.alloc(0))).toBeUndefined();
    expect(decodeEntry(good.subarray(0, 20))).toBeUndefined(); // shorter than the header
    expect(decodeEntry(good.subarray(0, good.length - 1))).toBeUndefined(); // truncated payload
    expect(decodeEntry(flipped)).toBeUndefined(); // digest mismatch
    expect(decodeEntry(wrongMagic)).toBeUndefined();
  });

  it("refuses a well-formed entry whose payload is not a lesson array", () => {
    // Header and digest are both correct; only the SHAPE is wrong. The loader
    // indexes the result as an array, so anything else must be a miss.
    const asObject = encodeEntry({ not: "an array" } as unknown as ParsedLesson[]);
    expect(decodeEntry(asObject)).toBeUndefined();
  });

  it("refuses a payload whose digest matches but which V8 cannot read", () => {
    // A correct magic and a correct digest around bytes that are not a V8
    // payload: the digest proves the bytes are the ones that were written, not
    // that they are readable, so the deserialize must be guarded as well.
    const garbage = Buffer.from([0xff, 0x0f, 0xfe, 0xfe, 0xfe]);
    const magic = encodeEntry([]).subarray(0, 8);
    const forged = Buffer.concat([magic, createHash("sha256").update(garbage).digest(), garbage]);
    expect(decodeEntry(forged)).toBeUndefined();
  });
});

describe("the key", () => {
  const base = { language: "spanish", script: undefined, files: [{ name: "a.md", bytes: Buffer.from("bc") }] };

  it("changes with every input that can change the parse", () => {
    const key = lessonCacheKey(base, "src-1");
    expect(key).toMatch(/^[0-9a-f]{64}$/);
    expect(lessonCacheKey(base, "src-1")).toBe(key);
    expect(lessonCacheKey(base, "src-2")).not.toBe(key); // the parser's own code
    expect(lessonCacheKey({ ...base, language: "french" }, "src-1")).not.toBe(key);
    expect(lessonCacheKey({ ...base, script: "latin" }, "src-1")).not.toBe(key);
    expect(lessonCacheKey({ ...base, script: "" }, "src-1")).not.toBe(key);
    expect(
      lessonCacheKey({ ...base, files: [{ name: "a.md", bytes: Buffer.from("bd") }] }, "src-1"),
    ).not.toBe(key);
    expect(lessonCacheKey({ ...base, files: [] }, "src-1")).not.toBe(key);
  });

  it("cannot be forged by moving bytes between a name and its content", () => {
    const left = lessonCacheKey(
      { ...base, files: [{ name: "a", bytes: Buffer.from("bc") }] },
      "src",
    );
    const right = lessonCacheKey(
      { ...base, files: [{ name: "ab", bytes: Buffer.from("c") }] },
      "src",
    );
    expect(left).not.toBe(right);
  });

  it("digests the package source by path and content", () => {
    const dir = join(scratch, "src");
    mkdirSync(join(dir, "nested"), { recursive: true });
    writeFileSync(join(dir, "a.ts"), "xy");
    writeFileSync(join(dir, "nested", "b.ts"), "");
    const before = directoryDigest(dir);
    writeFileSync(join(dir, "a.ts"), "x");
    writeFileSync(join(dir, "nested", "b.ts"), "y");
    expect(directoryDigest(dir)).not.toBe(before);
  });

  it.runIf(POSIX)("follows a linked source file to the code that actually runs", () => {
    const dir = join(scratch, "src");
    mkdirSync(dir);
    const target = join(scratch, "real-parse.ts");
    writeFileSync(target, "export const version = 1;");
    symlinkSync(target, join(dir, "parse.ts"));
    const before = directoryDigest(dir);
    writeFileSync(target, "export const version = 2;");
    expect(directoryDigest(dir)).not.toBe(before);
  });
});

describe.runIf(POSIX)("loadTrackLessons through the cache", () => {
  beforeEach(() => {
    writeLesson("spanish", "001-uno.md", lesson("uno", "uno", 1));
    writeLesson("spanish", "002-dos.md", lesson("dos", "dos"));
    expect(configureLessonCache({ root, directory: cacheDir })).toBe(true);
  });

  it("writes an entry on a miss, and returns exactly what a fresh parse returns", () => {
    expect(existsSync(cacheDir) && entries()).toEqual([]);
    const first = loadTrackLessons("spanish", root);
    expect(first).toStrictEqual(freshParse("spanish"));
    expect(entries()).toEqual([`spanish-${keyFor("spanish")}.v8`]);
    // And the next read, now a hit, is still indistinguishable.
    expect(loadTrackLessons("spanish", root)).toStrictEqual(first);
  });

  it("answers a hit from the entry rather than by parsing", () => {
    // Plant a sentinel under the real key. If the loader parsed, it would
    // return the real lessons; returning the sentinel proves the hit path ran.
    const sentinel = [parseLesson(lesson("sentinel", "centinela"), "spanish")];
    sentinel.push(parseLesson(lesson("sentinel-two", "centinela"), "spanish"));
    activeCache().write("spanish", keyFor("spanish"), sentinel);
    expect(loadTrackLessons("spanish", root)).toStrictEqual(sentinel);
  });

  it("re-parses after any lesson edit and retires the superseded entry", () => {
    const original = loadTrackLessons("spanish", root);
    const staleEntry = entries();
    // A one-byte edit that keeps the file's SIZE: the case a (size, mtime)
    // key gets wrong whenever the mtime does not move.
    const path = join(root, "spanish", "lessons", "002-dos.md");
    writeFileSync(path, readFileSync(path, "utf8").replace("headword: dos", "headword: doz"));
    const edited = loadTrackLessons("spanish", root);
    expect(edited).toStrictEqual(freshParse("spanish"));
    expect(edited[1]!.realization.headword).toBe("doz");
    expect(edited).not.toStrictEqual(original);
    expect(entries()).toHaveLength(1);
    expect(entries()).not.toEqual(staleEntry);
  });

  it("re-parses when a lesson is added, removed or renamed", () => {
    loadTrackLessons("spanish", root);
    writeLesson("spanish", "003-tres.md", lesson("tres", "tres"));
    expect(loadTrackLessons("spanish", root)).toHaveLength(3);
    rmSync(join(root, "spanish", "lessons", "001-uno.md"));
    expect(loadTrackLessons("spanish", root)).toStrictEqual(freshParse("spanish"));
    expect(loadTrackLessons("spanish", root)).toHaveLength(2);
  });

  it("keys on the declared script, so a track.json edit is a miss", () => {
    loadTrackLessons("spanish", root);
    writeFileSync(join(root, "spanish", "track.json"), JSON.stringify({ script: "cyrillic" }));
    expect(loadTrackLessons("spanish", root).map((l) => l.script)).toEqual([
      "cyrillic",
      "cyrillic",
    ]);
  });

  it("keeps each track's entries separate", () => {
    writeLesson("french", "001-un.md", lesson("un", "un"));
    loadTrackLessons("spanish", root);
    loadTrackLessons("french", root);
    expect(entries().map((name) => name.split("-")[0])).toEqual(["french", "spanish"]);
  });

  it.each([
    ["truncated", (bytes: Buffer) => bytes.subarray(0, Math.floor(bytes.length / 2))],
    ["empty", () => Buffer.alloc(0)],
    ["bit-flipped", (bytes: Buffer) => {
      const copy = Buffer.from(bytes);
      copy[copy.length - 5]! ^= 0x01;
      return copy;
    }],
    ["not an entry at all", () => Buffer.from("garbage that is not a cache entry")],
  ])("treats a %s entry as a miss and replaces it", (_label, damage) => {
    loadTrackLessons("spanish", root);
    const [name] = entries();
    const path = join(cacheDir, name!);
    writeFileSync(path, damage(readFileSync(path)), { mode: 0o600 });
    expect(loadTrackLessons("spanish", root)).toStrictEqual(freshParse("spanish"));
    // Replaced with a good entry, which the next load can use.
    expect(decodeEntry(readFileSync(path))).toStrictEqual(freshParse("spanish"));
  });

  it("does not let a lesson parse error be cached away", () => {
    writeLesson("spanish", "003-bad.md", lesson("not a valid id", "malo"));
    expect(() => loadTrackLessons("spanish", root)).toThrow(/stable hyphenated token/);
    expect(entries()).toEqual([]);
    expect(() => loadTrackLessons("spanish", root)).toThrow(/stable hyphenated token/);
  });

  it("is off for every curriculum root except the configured one", () => {
    const other = join(scratch, "other");
    mkdirSync(join(other, "spanish", "lessons"), { recursive: true });
    writeFileSync(join(other, "spanish", "lessons", "001-uno.md"), lesson("uno", "uno"));
    expect(lessonCacheFor(other)).toBeUndefined();
    loadTrackLessons("spanish", other);
    expect(entries()).toEqual([]);
  });

  it("is off entirely once unconfigured", () => {
    expect(configureLessonCache(undefined)).toBe(false);
    expect(lessonCacheFor(root)).toBeUndefined();
    loadTrackLessons("spanish", root);
    expect(entries()).toEqual([]);
  });

  it("serves loadLessons track by track", () => {
    writeLesson("french", "001-un.md", lesson("un", "un"));
    const all = loadLessons(root);
    expect(all.map((l) => l.language)).toEqual(["french", "spanish", "spanish"]);
    expect(entries()).toHaveLength(2);
    expect(loadLessons(root)).toStrictEqual(all);
  });
});

describe.runIf(POSIX)("the security boundary", () => {
  it("refuses a cache directory that group or other can reach", () => {
    mkdirSync(cacheDir, { mode: 0o700 });
    chmodSync(cacheDir, 0o755);
    expect(openLessonCache(cacheDir)).toBeUndefined();
    chmodSync(cacheDir, 0o700);
    expect(openLessonCache(cacheDir)).toBeDefined();
  });

  it("creates the directory private when it does not exist", () => {
    const cache = openLessonCache(join(cacheDir, "deeper", "still"));
    expect(cache).toBeDefined();
  });

  it("refuses a cache directory that is itself a symlink, even to a private one", () => {
    const real = join(scratch, "real-cache");
    mkdirSync(real, { mode: 0o700 });
    symlinkSync(real, cacheDir);
    expect(openLessonCache(cacheDir)).toBeUndefined();
  });

  it("refuses a path that exists but is not a directory", () => {
    writeFileSync(cacheDir, "not a directory");
    expect(openLessonCache(cacheDir)).toBeUndefined();
  });

  it("never reads an entry readable by group or other", () => {
    writeLesson("spanish", "001-uno.md", lesson("uno", "uno"));
    configureLessonCache({ root, directory: cacheDir });
    const sentinel = [parseLesson(lesson("sentinel", "centinela"), "spanish")];
    const path = join(cacheDir, `spanish-${keyFor("spanish")}.v8`);
    writeFileSync(path, encodeEntry(sentinel), { mode: 0o644 });
    chmodSync(path, 0o644); // in case the umask stripped the bits being tested
    expect(loadTrackLessons("spanish", root)).toStrictEqual(freshParse("spanish"));
  });

  it("never follows a symlink planted under an entry's name", () => {
    writeLesson("spanish", "001-uno.md", lesson("uno", "uno"));
    configureLessonCache({ root, directory: cacheDir });
    const sentinel = [parseLesson(lesson("sentinel", "centinela"), "spanish")];
    const elsewhere = join(scratch, "planted.v8");
    writeFileSync(elsewhere, encodeEntry(sentinel), { mode: 0o600 });
    symlinkSync(elsewhere, join(cacheDir, `spanish-${keyFor("spanish")}.v8`));
    expect(loadTrackLessons("spanish", root)).toStrictEqual(freshParse("spanish"));
    // The planted target was not touched, let alone deleted through the link.
    expect(decodeEntry(readFileSync(elsewhere))).toStrictEqual(sentinel);
  });

  it("only trusts files owned by the current user with no group/other bits", () => {
    const uid = process.getuid!();
    expect(privatelyOwned({ uid, mode: 0o100600 }, uid)).toBe(true);
    expect(privatelyOwned({ uid, mode: 0o40700 }, uid)).toBe(true);
    expect(privatelyOwned({ uid: uid + 1, mode: 0o100600 }, uid)).toBe(false);
    expect(privatelyOwned({ uid, mode: 0o100640 }, uid)).toBe(false);
    expect(privatelyOwned({ uid, mode: 0o100602 }, uid)).toBe(false);
  });

  it("writes entries mode 0600", () => {
    writeLesson("spanish", "001-uno.md", lesson("uno", "uno"));
    configureLessonCache({ root, directory: cacheDir });
    loadTrackLessons("spanish", root);
    expect(statSync(join(cacheDir, entries()[0]!)).mode & 0o777).toBe(0o600);
  });
});

describe("where POSIX ownership does not exist", () => {
  it("stays off, so Windows parses exactly as before", () => {
    const original = Object.getOwnPropertyDescriptor(process, "getuid");
    Object.defineProperty(process, "getuid", { value: undefined, configurable: true });
    try {
      expect(openLessonCache(cacheDir)).toBeUndefined();
    } finally {
      if (original) Object.defineProperty(process, "getuid", original);
      else delete (process as { getuid?: unknown }).getuid;
    }
  });
});

describe.runIf(POSIX)("concurrent writers", () => {
  // Real processes, not interleaved calls: the property is about what the
  // KERNEL lets a reader see while other processes write, which no amount of
  // single-process sequencing can demonstrate. The children load the module
  // straight from source with Node's type stripping, which is why
  // `lesson-cache.ts` imports nothing at runtime but Node built-ins.
  const modulePath = join(here, "..", "src", "lesson-cache.ts");

  /**
   * Start one writer process. The payload goes through a file, not argv: the
   * point is a payload large enough that one write takes many syscalls, and
   * that is far past the kernel's argument-size limit.
   */
  function writer(directory: string, key: string, payload: unknown, rounds: number, tag: string) {
    const payloadPath = join(scratch, `payload-${tag}.json`);
    writeFileSync(payloadPath, JSON.stringify(payload));
    const scriptPath = join(scratch, `writer-${tag}.mjs`);
    writeFileSync(
      scriptPath,
      [
        `import { readFileSync } from "node:fs";`,
        `import { openLessonCache } from ${JSON.stringify(pathToFileURL(modulePath).href)};`,
        `const cache = openLessonCache(${JSON.stringify(directory)});`,
        `if (!cache) { console.error("cache refused"); process.exit(2); }`,
        `const payload = JSON.parse(readFileSync(${JSON.stringify(payloadPath)}, "utf8"));`,
        `for (let i = 0; i < ${rounds}; i++) cache.write("spanish", ${JSON.stringify(key)}, payload);`,
      ].join("\n"),
    );
    return new Promise<number>((resolveExit, reject) => {
      const child = spawn(
        process.execPath,
        ["--experimental-strip-types", "--no-warnings", scriptPath],
        { stdio: ["ignore", "ignore", "inherit"] },
      );
      child.on("error", reject);
      child.on("exit", (code) => resolveExit(code ?? -1));
    });
  }

  it("never exposes a partial entry to a reader, and leaves no temp files", async () => {
    mkdirSync(cacheDir, { mode: 0o700 });
    const cache = openLessonCache(cacheDir)!;
    const key = "a".repeat(64);
    // Two DIFFERENT payloads under one key. Real writers always agree (the key
    // is a hash of the content), so this is strictly harsher than reality: a
    // reader must still only ever see one whole payload or the other. They are
    // large enough that a write takes many syscalls, so a torn read would show.
    const big = (tag: string) =>
      Array.from({ length: 400 }, (_, i) => ({ tag, i, text: `${tag}`.repeat(500) }));
    const a = big("a");
    const b = big("b");
    const writers = [
      writer(cacheDir, key, a, 25, "a1"),
      writer(cacheDir, key, b, 25, "b"),
      writer(cacheDir, key, a, 25, "a2"),
    ];
    let reads = 0;
    let hits = 0;
    let done = false;
    const settled = Promise.all(writers).finally(() => {
      done = true;
    });
    while (!done) {
      const value = cache.read("spanish", key);
      reads += 1;
      if (value !== undefined) {
        hits += 1;
        expect(isDeepStrictEqual(value, a) || isDeepStrictEqual(value, b)).toBe(true);
      }
      await new Promise((tick) => setImmediate(tick));
    }
    expect(await settled).toEqual([0, 0, 0]);
    expect(reads).toBeGreaterThan(0);
    // After the dust settles: exactly one complete entry, no stray temp files.
    expect(readdirSync(cacheDir)).toEqual([`spanish-${key}.v8`]);
    const final = cache.read("spanish", key);
    expect(isDeepStrictEqual(final, a) || isDeepStrictEqual(final, b)).toBe(true);
    expect(hits).toBeLessThanOrEqual(reads);
  });
});

describe("the suite's own configuration", () => {
  it("caches the real curriculum root on POSIX, from the package-local directory", () => {
    // `tests/setup/lesson-cache.ts` takes its root from
    // `suiteLessonCacheOptions()` rather than importing the loader. If that
    // arithmetic ever drifted from `defaultCurriculumRoot()`, the cache would
    // silently never turn on and the suite would quietly be slow again. This
    // is that alarm.
    expect(resolve(suiteLessonCacheOptions().root)).toBe(resolve(defaultCurriculumRoot()));
    const cache = lessonCacheFor(defaultCurriculumRoot());
    if (POSIX) {
      expect(cache?.directory).toBe(realpathSync(defaultLessonCacheDirectory()));
    } else {
      expect(cache).toBeUndefined();
    }
  });

  // The fixture cases above prove the mechanism. These prove it on the data
  // that matters: every real track, read back through whatever entry this
  // run's global setup wrote, against a parse that never touches the cache.
  // One case per track keeps each well inside the default budget.
  const realRoot = defaultCurriculumRoot();
  const realTracks = readdirSync(realRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && existsSync(join(realRoot, entry.name, "lessons")))
    .map((entry) => entry.name)
    .sort();

  it("finds the real tracks it is about to compare", () => {
    expect(realTracks.length).toBeGreaterThan(20);
  });

  it.each(realTracks)("returns exactly what a fresh parse returns for real %s", (track) => {
    configureLessonCache(suiteLessonCacheOptions());
    const cached = loadTrackLessons(track, realRoot);
    configureLessonCache(undefined);
    const fresh = loadTrackLessons(track, realRoot);
    expect(cached.length).toBe(fresh.length);
    const mismatch = cached.findIndex((value, index) => !isDeepStrictEqual(value, fresh[index]));
    expect(mismatch === -1 ? "identical" : fresh[mismatch]!.realization.lessonId).toBe("identical");
  });
});
