// narration-cache.test.ts — the narration cache must be invisible except in speed.
//
// `src/narration-cache.ts` lets the suite narrate each track once per run and
// read the chapters back everywhere else. Like the lesson cache, that is only
// acceptable if no test can tell the difference, so this file pins:
//
//   EXACT       a cached corpus is exactly what an uncached `corpusNarration`
//               returns — same chapters, same order — across several tracks
//   HIT         a stored entry is actually used, and only the track that
//               changed is narrated again
//   STALE       a lesson edit, a title edit or a policy edit is never answered
//               from the old entry, and the edit shows in what comes back
//   SCOPED      off for every root but the configured one, off when the lesson
//               cache is off (there is no lesson key to name the lessons by)
//
// and the boundary from the module header: the narration cache opens files only
// through the lesson cache's own store, so the directory rules are that store's.
// One case per rule is enough to prove the narration cache really is behind
// them; `tests/lesson-cache.test.ts` proves the rules themselves in depth.
//
// Every fixture case builds its own curriculum and its own cache directories in
// a temp dir and points the process-wide configuration at them, then puts the
// suite's real configuration back — so nothing here touches the real entries.

import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  configureLessonCache,
  defaultLessonCacheDirectory,
  lessonCacheFor,
  suiteLessonCacheOptions,
  type CachePlatform,
} from "../src/lesson-cache.js";
import { defaultCurriculumRoot, loadLessons, loadTrackLessons } from "../src/loader.js";
import {
  configureNarrationCache,
  defaultNarrationCacheDirectory,
  narrationCacheFor,
  narrationCacheKey,
  openNarrationCache,
  suiteNarrationCacheOptions,
  type NarrationCacheKeyInput,
} from "../src/narration-cache.js";
import {
  chapterTitles,
  corpusNarration,
  corpusNarrationOptions,
  policyTableWidth,
} from "../src/narration-cli.js";
import { narrationChapters } from "../src/narration.js";

const POSIX = typeof process.getuid === "function";

let scratch: string;
let root: string;
let lessonDir: string;
let narrationDir: string;

/** One small lesson. Two chapters' worth per track is enough to show the order. */
function lesson(id: string, headword: string, chapter: number, sequence: number): string {
  return [
    "---",
    "schema_version: 2",
    `id: ${id}`,
    `sequence: ${sequence}`,
    `chapter: ${chapter}`,
    "type: word",
    `headword: ${headword}`,
    "gloss: a test word",
    "concept_tag: GREETING-HELLO",
    "---",
    "",
    `# ${headword} — a test word`,
    "",
    "## Warm-up",
    "",
    "[PAUSE 2s] Say it once.",
    "",
    "## Guided Practice",
    "",
    `- [YOU SAY: "${headword}"]`,
    "",
  ].join("\n");
}

function write(relative: string, contents: string): void {
  const path = join(root, relative);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, contents, "utf8");
}

function writePolicy(width: number): void {
  write(
    "core/chapter-policy.json",
    `${JSON.stringify({
      version: 1,
      payoffRepresentativeness: 0.5,
      maxNewAtomsPerLesson: 3,
      maxNewAtomsPerChapter: 12,
      maxLinearisableTableColumns: width,
    })}\n`,
  );
}

function writeTitle(language: string, title: string): void {
  write(
    `${language}/chapters.json`,
    `${JSON.stringify({
      version: 1,
      language,
      chapters: [
        {
          chapter: 1,
          title,
          label: `ch:${language}-first`,
          canDo: "I can say hello.",
          spineNodes: [],
          payoff: { lesson: `${language}-C01-a`, kind: "dialogue", summary: "hi", assesses: [] },
        },
      ],
    })}\n`,
  );
}

/** Narration entries currently on disk, as `<track>` -> entry name. */
function entries(): Map<string, string> {
  return new Map(
    readdirSync(narrationDir)
      .filter((name) => name.endsWith(".v8"))
      .map((name) => [name.slice(0, name.lastIndexOf("-")), name]),
  );
}

/** `corpusNarration(root)` with both caches off: the answer the cache must reproduce. */
function uncached(): ReturnType<typeof corpusNarration> {
  configureNarrationCache(undefined);
  configureLessonCache(undefined);
  try {
    return corpusNarration(root);
  } finally {
    expect(configureLessonCache({ root, directory: lessonDir })).toBe(true);
    expect(configureNarrationCache({ root, directory: narrationDir })).toBe(true);
  }
}

beforeEach(() => {
  scratch = mkdtempSync(join(tmpdir(), "hl-narration-cache-"));
  root = join(scratch, "curriculum");
  lessonDir = join(scratch, "lesson-cache");
  narrationDir = join(scratch, "narration-cache");
  writePolicy(3);
  // Two tracks whose chapters would interleave if they were sorted by chapter
  // number alone, so a per-track concatenation in the wrong order would show.
  write("alpha/lessons/01-a.md", lesson("alpha-C01-a", "uno", 1, 10));
  write("alpha/lessons/02-b.md", lesson("alpha-C02-b", "dos", 2, 10));
  write("beta/lessons/01-a.md", lesson("beta-C01-a", "eins", 1, 10));
  write("beta/lessons/02-b.md", lesson("beta-C01-b", "zwei", 1, 20));
  write("beta/lessons/03-c.md", lesson("beta-C03-c", "drei", 3, 10));
  writeTitle("alpha", "First Words");
});

afterEach(() => {
  // Back to what the suite's setup file configured, so the next file in this
  // worker sees the real caches exactly as it would have.
  configureLessonCache(suiteLessonCacheOptions());
  configureNarrationCache(suiteNarrationCacheOptions());
  rmSync(scratch, { recursive: true, force: true });
});

describe("the key", () => {
  const base: NarrationCacheKeyInput = {
    language: "alpha",
    maxLinearisableTableColumns: 3,
    titles: new Map([["alpha/1", "First Words"]]),
    lessonKey: "a".repeat(64),
  };
  const key = (change: Partial<NarrationCacheKeyInput>, digest = "digest") =>
    narrationCacheKey({ ...base, ...change }, digest);

  it("changes with every input that can change a track's narration", () => {
    const keys = [
      key({}),
      key({ language: "beta" }),
      key({ maxLinearisableTableColumns: 2 }),
      key({ titles: new Map([["alpha/1", "Other Words"]]) }),
      key({ titles: new Map([["alpha/2", "First Words"]]) }),
      key({ titles: new Map() }),
      key({ lessonKey: "b".repeat(64) }),
      key({}, "another source digest"),
    ];
    expect(new Set(keys).size).toBe(keys.length);
    expect(key({})).toMatch(/^[0-9a-f]{64}$/);
  });

  it("ignores another track's titles, which this track's chapters never read", () => {
    expect(
      key({ titles: new Map([["alpha/1", "First Words"], ["beta/1", "Erste Wörter"]]) }),
    ).toBe(key({}));
    // A prefix that merely LOOKS like this track's is another track's.
    expect(key({ titles: new Map([["alpha/1", "First Words"], ["alphabet/1", "x"]]) })).toBe(
      key({}),
    );
  });

  it("does not depend on the order the titles were inserted in", () => {
    const one = new Map([["alpha/1", "One"], ["alpha/2", "Two"]]);
    const other = new Map([["alpha/2", "Two"], ["alpha/1", "One"]]);
    expect(key({ titles: one })).toBe(key({ titles: other }));
  });

  it("cannot be forged by moving bytes between a title's key and its text", () => {
    expect(key({ titles: new Map([["alpha/1", "ab"]]) })).not.toBe(
      key({ titles: new Map([["alpha/1a", "b"]]) }),
    );
  });
});

describe.runIf(POSIX)("corpusNarration through the cache", () => {
  beforeEach(() => {
    expect(configureLessonCache({ root, directory: lessonDir })).toBe(true);
    expect(configureNarrationCache({ root, directory: narrationDir })).toBe(true);
  });

  it("returns exactly what an uncached narration returns, in corpus order", () => {
    const cached = corpusNarration(root);
    expect(cached.map((chapter) => `${chapter.language}/${chapter.chapter}`)).toEqual([
      "alpha/1",
      "alpha/2",
      "beta/1",
      "beta/3",
    ]);
    expect(cached[0]?.title).toBe("First Words");
    expect(cached).toStrictEqual(uncached());
    expect([...entries().keys()]).toEqual(["alpha", "beta"]);
    // And the next read, now a hit, is still indistinguishable.
    expect(corpusNarration(root)).toStrictEqual(cached);
  });

  it("answers a hit from the entry, and re-narrates only the track that changed", () => {
    corpusNarration(root);
    const before = entries();
    corpusNarration(root);
    expect(entries()).toEqual(before); // A hit writes nothing.

    write("beta/lessons/02-b.md", lesson("beta-C01-b", "zwo", 1, 20));
    const after = corpusNarration(root);
    const now = entries();
    expect(now.get("alpha")).toBe(before.get("alpha"));
    expect(now.get("beta")).not.toBe(before.get("beta")); // Replaced, not added beside.
    expect(now.size).toBe(2);
    // The edit is what came back — the stale entry was not.
    expect(JSON.stringify(after)).toContain("zwo");
    expect(after).toStrictEqual(uncached());
  });

  it("keys on the chapter titles and the policy width", () => {
    corpusNarration(root);
    const before = entries();

    writeTitle("alpha", "Very First Words");
    const retitled = corpusNarration(root);
    expect(retitled[0]?.title).toBe("Very First Words");
    expect(entries().get("alpha")).not.toBe(before.get("alpha"));
    expect(entries().get("beta")).toBe(before.get("beta"));
    expect(retitled).toStrictEqual(uncached());

    const titled = entries();
    writePolicy(2);
    expect(corpusNarration(root)).toStrictEqual(uncached());
    expect(entries().get("alpha")).not.toBe(titled.get("alpha"));
    expect(entries().get("beta")).not.toBe(titled.get("beta"));
  });

  it("narrates uncached when the lesson cache is off: no lesson key, no entry", () => {
    configureLessonCache(undefined);
    expect(corpusNarration(root)).toStrictEqual(uncached());
    configureLessonCache(undefined);
    corpusNarration(root);
    expect(entries().size).toBe(0);
  });

  it("is off for every curriculum root except the configured one", () => {
    expect(narrationCacheFor(root)).toBeDefined();
    expect(narrationCacheFor(join(scratch, "elsewhere"))).toBeUndefined();
    configureNarrationCache(undefined);
    expect(narrationCacheFor(root)).toBeUndefined();
  });

  it("is told each served track's lesson key by the loader, and only when caching", () => {
    const reported = new Map<string, string>();
    const lessons = loadLessons(root, (language, key) => reported.set(language, key));
    expect([...reported.keys()]).toEqual(["alpha", "beta"]);
    // The key the loader reports is the one the lesson cache files the track under.
    const lessonEntries = readdirSync(lessonDir).filter((name) => name.endsWith(".v8"));
    expect(lessonEntries.sort()).toEqual(
      [...reported].map(([language, key]) => `${language}-${key}.v8`).sort(),
    );
    expect(lessons.length).toBe(5);

    configureLessonCache(undefined);
    const silent: string[] = [];
    loadLessons(root, (language) => silent.push(language));
    expect(silent).toEqual([]);
  });
});

describe.runIf(POSIX)("the directory rules are the lesson cache's", () => {
  it("refuses a cache directory that group or other can reach", () => {
    mkdirSync(narrationDir, { mode: 0o700 });
    chmodSync(narrationDir, 0o755);
    expect(openNarrationCache(narrationDir)).toBeUndefined();
    expect(configureNarrationCache({ root, directory: narrationDir })).toBe(false);
  });

  it("on (simulated) Windows, accepts only the one default directory", () => {
    const win32: CachePlatform = {
      kind: "win32",
      defaultDirectory: narrationDir,
      noFollow: 0,
      open: (path, flags, mode) => {
        throw new Error(`not opened in this case: ${path} ${flags} ${mode}`);
      },
      fstat: () => {
        throw new Error("not opened in this case");
      },
      rename: () => {
        throw new Error("not written in this case");
      },
    };
    const elsewhere = join(scratch, "elsewhere");
    mkdirSync(elsewhere, { mode: 0o700 });
    expect(openNarrationCache(elsewhere, win32)).toBeUndefined();
    expect(openNarrationCache(lessonDir, win32)).toBeUndefined();
    expect(openNarrationCache(narrationDir, win32)).toBeDefined();
  });
});

describe("what the key does not need to name", () => {
  // The key names the lessons (by their lesson key), the width, the titles and
  // the package source. That is complete only while the narrator is a pure
  // function of its arguments. If `narration.ts` or anything it imports ever
  // read a curriculum file, an environment variable or the clock, that would be
  // an input the key cannot see, and an edit to it would be answered from a
  // stale entry — through the very test that is CI's narration gate. So the
  // runtime import graph is walked here and every module in it is checked.
  const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..", "src");

  // Every way a module can pull in another at load time. `import type` and
  // `export type` are erased, so they are not runtime edges; everything else is
  // followed, quoted either way.
  const STATIC_EDGES = [
    /^import\s+(?!type\b)[^;]*?\bfrom\s+["']([^"']+)["']/gms, // import x from "…"
    /^export\s+(?!type\b)[^;]*?\bfrom\s+["']([^"']+)["']/gms, // export { x } from "…"
    /^import\s+["']([^"']+)["']/gm, //                            import "…" (side effect)
  ];

  // Not followed but refused outright: a dynamic edge is one this walk cannot
  // resolve, and the rest are the ways a module reads something that is not an
  // argument — the process (its environment, argv, cwd), the global object, the
  // clock, chance, or code built from a string. Fail-closed on purpose: a comment
  // that merely says "process" fails too, and is cheaper to reword than an unseen
  // input is to find.
  const BANNED: readonly [string, RegExp][] = [
    ["dynamic import()", /\bimport\s*\(/],
    ["require()", /\brequire\s*\(/],
    ["process", /\bprocess\b/],
    ["globalThis", /\bglobalThis\b/],
    ["Date", /\bDate\b/],
    ["Math.random", /\bMath\.random\b/],
    ["performance", /\bperformance\b/],
    ["eval()", /\beval\s*\(/],
    ["new Function", /\bnew\s+Function\b/],
  ];

  function runtimeGraph(entry: string): Map<string, string> {
    const seen = new Map<string, string>();
    const visit = (file: string): void => {
      if (seen.has(file)) return;
      const source = readFileSync(join(srcDir, file), "utf8");
      seen.set(file, source);
      for (const pattern of STATIC_EDGES) {
        for (const match of source.matchAll(pattern)) {
          const specifier = match[1]!;
          if (specifier.startsWith("./")) visit(specifier.slice(2).replace(/\.js$/, ".ts"));
          else seen.set(`<external ${specifier}>`, "");
        }
      }
    };
    visit(entry);
    return seen;
  }

  /** Every banned construct in `graph`, as `"<module>: <what>"`. */
  function bannedIn(graph: ReadonlyMap<string, string>): string[] {
    return [...graph].flatMap(([name, source]) =>
      BANNED.filter(([, pattern]) => pattern.test(source)).map(([what]) => `${name}: ${what}`),
    );
  }

  it("is a narrator that reads no file, environment variable or clock", () => {
    const graph = runtimeGraph("narration.ts");
    expect(graph.size).toBeGreaterThan(5); // The walk found the real graph.
    const external = [...graph.keys()].filter((name) => name.startsWith("<external"));
    expect(external).toEqual([]);
    expect(bannedIn(graph)).toEqual([]);
  });

  it("would see every kind of edge and every banned read (the walk itself, tested)", () => {
    const probe = (source: string): string[] =>
      bannedIn(new Map([["probe.ts", source]])).map((line) => line.slice("probe.ts: ".length));
    const edges = (source: string): string[] =>
      STATIC_EDGES.flatMap((pattern) => [...source.matchAll(pattern)].map((match) => match[1]!));
    expect(
      edges(
        [
          'import { a } from "./a.js";',
          "import b, {\n  c,\n} from './b.js';",
          'import type { T } from "./type-only.js";',
          'export { d } from "./d.js";',
          'export * from "node:fs";',
          'export type { U } from "./type-only-too.js";',
          'import "./side-effect.js";',
        ].join("\n"),
      ),
    ).toEqual(["./a.js", "./b.js", "./d.js", "node:fs", "./side-effect.js"]);
    expect(probe('const m = await import("./x.js");')).toEqual(["dynamic import()"]);
    expect(probe('const fs = require("node:fs");')).toEqual(["require()"]);
    expect(probe("const home = process.env.HOME;")).toEqual(["process"]);
    expect(probe("const g = globalThis;")).toEqual(["globalThis"]);
    expect(probe("const now = Date();")).toEqual(["Date"]);
    expect(probe("const t = Date.now();")).toEqual(["Date"]);
    expect(probe("const r = Math.random();")).toEqual(["Math.random"]);
    expect(probe("const f = new Function('return 1');")).toEqual(["new Function"]);
    expect(probe("const pure = (x: number) => x + 1;")).toEqual([]);
  });

  it("names every narration option in the key", () => {
    // `corpusNarrationOptions` is the only constructor of the options that reach
    // `narrationChapters`, and `narrateCorpusByTrack` copies them into the key by
    // name. If a field is ever added there, this fails until the key — and this
    // list — learn about it.
    expect(Object.keys(corpusNarrationOptions(3, new Map())).sort()).toEqual([
      "maxLinearisableTableColumns",
      "titles",
    ]);
    const keyed: (keyof NarrationCacheKeyInput)[] = [
      "language",
      "lessonKey",
      "maxLinearisableTableColumns",
      "titles",
    ];
    for (const option of Object.keys(corpusNarrationOptions(3, new Map()))) {
      expect(keyed).toContain(option);
    }
  });
});

describe("the suite's own configuration", () => {
  it("caches the real curriculum root, beside the lesson cache", () => {
    // The same alarm as the lesson cache's: if this root drifted from the
    // loader's, the cache would silently never turn on.
    expect(resolve(suiteNarrationCacheOptions().root)).toBe(resolve(defaultCurriculumRoot()));
    expect(defaultNarrationCacheDirectory()).toBe(
      join(dirname(defaultLessonCacheDirectory()), "narration"),
    );
    expect(narrationCacheFor(defaultCurriculumRoot())?.directory).toBe(
      realpathSync.native(defaultNarrationCacheDirectory()),
    );
    // Narrating needs a lesson key, so the lesson cache must be on here too.
    expect(lessonCacheFor(defaultCurriculumRoot())).toBeDefined();
  });

  // The fixture cases prove the mechanism; this proves it on real data, through
  // whatever entry this run's global setup wrote. The smallest real track keeps
  // the fresh narration it is compared against well inside the budget. (The
  // whole corpus is compared byte for byte, cached, by narration-cli's `--check`
  // case against files generated without the cache.)
  it("answers a real track exactly as a fresh narration does", () => {
    const realRoot = defaultCurriculumRoot();
    const track = "chinese";
    expect(readdirSync(join(realRoot, track, "lessons")).length).toBeGreaterThan(100);
    const cached = corpusNarration(realRoot).filter((chapter) => chapter.language === track);
    configureNarrationCache(undefined);
    configureLessonCache(undefined);
    const fresh = narrationChapters(loadTrackLessons(track, realRoot), {
      maxLinearisableTableColumns: policyTableWidth(realRoot),
      titles: chapterTitles(realRoot),
    });
    expect(cached.length).toBe(fresh.length);
    expect(cached).toStrictEqual(fresh);
    // It did come from the cache: the entry for this track is on disk.
    expect(
      readdirSync(defaultNarrationCacheDirectory()).some((name) => name.startsWith(`${track}-`)),
    ).toBe(true);
  });
});
