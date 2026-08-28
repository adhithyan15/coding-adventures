import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { ViteDevServer } from "vite";
import { describe, expect, it, vi } from "vitest";
import {
  BOOK_HASH_MODULE_PREFIX,
  CURRICULUM_MODULE_PREFIX,
  CHAPTER_MODULE_PREFIX,
  LEDGER_INDEX_ID,
  RESOLVED_LEDGER_INDEX_ID,
  humanLanguageLedgerPlugin,
  ledgerModuleIdsForShardPath,
  loadHumanLanguageLedgerModule,
  resolveHumanLanguageLedgerId,
  validatedBookHashTracks,
} from "../human-language-ledger-plugin.ts";
import { readGeneratedBookHashManifest } from "@coding-adventures/human-language-data/src/generated-hash-shards.ts";
import {
  defaultCurriculumRoot,
  loadCurriculumSpine,
  loadLanguageCurricula,
  loadLanguageRegistry,
  loadTrackChapters,
} from "@coding-adventures/human-language-data/src/loader.ts";

const root = defaultCurriculumRoot();

function defaultExport(source: string): unknown {
  const prefix = "export default ";
  expect(source.startsWith(prefix)).toBe(true);
  return JSON.parse(source.slice(prefix.length, -2));
}

function writeBookHashOwner(root: string, language: string): void {
  const ownerDirectory = join(
    root,
    "core",
    "generated-book-hashes",
    `${language}.d`,
  );
  mkdirSync(ownerDirectory, { recursive: true });
  writeFileSync(
    join(ownerDirectory, "_meta.json"),
    JSON.stringify({ version: 1, language, algorithm: "fnv1a64" }),
  );
  writeFileSync(
    join(ownerDirectory, "0001.json"),
    JSON.stringify({
      language,
      chapter: 1,
      sourceHash: "fnv1a64:0123456789abcdef",
      lessonIds: [`${language}-lesson-1`],
      tex: `${language}/book/chapters/ch01.tex`,
    }),
  );
}

function writeLanguageRegistry(
  root: string,
  languages: readonly string[],
): void {
  mkdirSync(join(root, "core"), { recursive: true });
  writeFileSync(
    join(root, "core", "languages.json"),
    JSON.stringify({ languages: languages.map((id) => ({ id })) }),
  );
}

describe("the bounded human-language ledger virtual modules", () => {
  it("resolves only the public index and its safe per-track children", () => {
    expect(resolveHumanLanguageLedgerId(LEDGER_INDEX_ID)).toBe(
      RESOLVED_LEDGER_INDEX_ID,
    );
    expect(
      resolveHumanLanguageLedgerId(`${CURRICULUM_MODULE_PREFIX}spanish`),
    ).toBe(`\0${CURRICULUM_MODULE_PREFIX}spanish`);
    expect(
      resolveHumanLanguageLedgerId(`${CHAPTER_MODULE_PREFIX}../../etc`),
    ).toBeNull();
    expect(
      resolveHumanLanguageLedgerId(`${BOOK_HASH_MODULE_PREFIX}spanish`),
    ).toBe(`\0${BOOK_HASH_MODULE_PREFIX}spanish`);
    expect(
      resolveHumanLanguageLedgerId(`${BOOK_HASH_MODULE_PREFIX}spanish/0001`),
    ).toBeNull();
  });

  it("rejects unsafe and duplicate registry track ids before joining paths", () => {
    const temporary = mkdtempSync(join(tmpdir(), "language-ledger-registry-"));
    try {
      mkdirSync(join(temporary, "core"));
      writeFileSync(
        join(temporary, "core", "languages.json"),
        JSON.stringify({ languages: [{ id: "../outside" }] }),
      );
      expect(() =>
        loadHumanLanguageLedgerModule(
          RESOLVED_LEDGER_INDEX_ID,
          temporary,
          () => {},
        ),
      ).toThrow(/unsafe registry track id/);

      writeFileSync(
        join(temporary, "core", "languages.json"),
        JSON.stringify({ languages: [{ id: "spanish" }, { id: "spanish" }] }),
      );
      expect(() =>
        loadHumanLanguageLedgerModule(
          RESOLVED_LEDGER_INDEX_ID,
          temporary,
          () => {},
        ),
      ).toThrow(/duplicate registry track id/);
    } finally {
      rmSync(temporary, { recursive: true, force: true });
    }
  });

  it("rejects a safe track id whose directory symlink escapes the curriculum root", () => {
    const temporary = mkdtempSync(join(tmpdir(), "language-ledger-root-"));
    const outside = mkdtempSync(join(tmpdir(), "language-ledger-outside-"));
    try {
      mkdirSync(join(temporary, "core"));
      writeFileSync(
        join(temporary, "core", "languages.json"),
        JSON.stringify({ languages: [{ id: "spanish" }] }),
      );
      symlinkSync(outside, join(temporary, "spanish"));
      expect(() =>
        loadHumanLanguageLedgerModule(
          RESOLVED_LEDGER_INDEX_ID,
          temporary,
          () => {},
        ),
      ).toThrow(/ledger parent escapes curriculum root/);
    } finally {
      rmSync(temporary, { recursive: true, force: true });
      rmSync(outside, { recursive: true, force: true });
    }
  });

  it("rejects an unregistered but otherwise valid generated-book owner", () => {
    const temporary = mkdtempSync(
      join(tmpdir(), "language-ledger-extra-owner-"),
    );
    try {
      writeBookHashOwner(temporary, "spanish");
      writeBookHashOwner(temporary, "ghost");

      expect(() => validatedBookHashTracks(temporary, ["spanish"])).toThrow(
        /generated book hash owner "ghost\.d" is not registered/,
      );
    } finally {
      rmSync(temporary, { recursive: true, force: true });
    }
  });

  it("rejects a registered language whose generated-book owner is missing", () => {
    const temporary = mkdtempSync(
      join(tmpdir(), "language-ledger-missing-owner-"),
    );
    try {
      writeBookHashOwner(temporary, "spanish");

      expect(() =>
        validatedBookHashTracks(temporary, ["spanish", "french"]),
      ).toThrow(
        /registered language "french" has no generated book hash owner/,
      );
    } finally {
      rmSync(temporary, { recursive: true, force: true });
    }
  });

  it("rejects an intermediate generated-hash symlink that escapes the configured root", () => {
    const temporary = mkdtempSync(
      join(tmpdir(), "language-ledger-owner-root-"),
    );
    const outside = mkdtempSync(
      join(tmpdir(), "language-ledger-owner-outside-"),
    );
    try {
      writeLanguageRegistry(temporary, ["spanish"]);
      mkdirSync(join(outside, "spanish.d"));
      symlinkSync(outside, join(temporary, "core", "generated-book-hashes"));

      expect(() =>
        loadHumanLanguageLedgerModule(
          `\0${BOOK_HASH_MODULE_PREFIX}spanish`,
          temporary,
          () => {},
        ),
      ).toThrow(/ledger parent escapes curriculum root/);
    } finally {
      rmSync(temporary, { recursive: true, force: true });
      rmSync(outside, { recursive: true, force: true });
    }
  });

  it("rejects direct loading of an unregistered book-hash virtual child", () => {
    const temporary = mkdtempSync(
      join(tmpdir(), "language-ledger-ghost-child-"),
    );
    try {
      mkdirSync(join(temporary, "core"));
      writeFileSync(
        join(temporary, "core", "languages.json"),
        JSON.stringify({ languages: [{ id: "spanish" }] }),
      );

      expect(() =>
        loadHumanLanguageLedgerModule(
          `\0${BOOK_HASH_MODULE_PREFIX}ghost`,
          temporary,
          () => {},
        ),
      ).toThrow(/book hash track "ghost" is not registered/);
    } finally {
      rmSync(temporary, { recursive: true, force: true });
    }
  });

  it("invalidates add/unlink modules and explicitly reloads the browser", () => {
    const watched = vi.fn();
    const invalidated = vi.fn();
    const sent = vi.fn();
    const indexModule = { id: RESOLVED_LEDGER_INDEX_ID };
    const curriculumId = `\0${CURRICULUM_MODULE_PREFIX}spanish`;
    const curriculumModule = { id: curriculumId };
    const bookHashId = `\0${BOOK_HASH_MODULE_PREFIX}spanish`;
    const bookHashModule = { id: bookHashId };
    let onAll: ((event: string, path: string) => void) | undefined;
    const server = {
      watcher: {
        add: watched,
        on: vi.fn(
          (event: string, handler: (event: string, path: string) => void) => {
            if (event === "all") onAll = handler;
          },
        ),
      },
      moduleGraph: {
        getModuleById: vi.fn((id: string) => {
          if (id === RESOLVED_LEDGER_INDEX_ID) return indexModule;
          if (id === curriculumId) return curriculumModule;
          if (id === bookHashId) return bookHashModule;
          return undefined;
        }),
        invalidateModule: invalidated,
      },
      ws: { send: sent },
    } as unknown as ViteDevServer;
    const configure = humanLanguageLedgerPlugin({
      curriculumRoot: root,
    }).configureServer;
    expect(typeof configure).toBe("function");
    (configure as (server: ViteDevServer) => void)(server);

    const shard = join(
      root,
      "spanish",
      "curriculum.d",
      "9999-ES-PATH-NEW.json",
    );
    expect(ledgerModuleIdsForShardPath(shard, root)).toEqual([
      RESOLVED_LEDGER_INDEX_ID,
      curriculumId,
    ]);
    onAll?.("add", shard);
    expect(invalidated).toHaveBeenCalledWith(indexModule);
    expect(invalidated).toHaveBeenCalledWith(curriculumModule);
    expect(sent).toHaveBeenCalledWith({ type: "full-reload", path: "*" });

    onAll?.("change", shard);
    expect(sent).toHaveBeenCalledTimes(1);

    const bookShard = join(
      root,
      "core",
      "generated-book-hashes",
      "spanish.d",
      "9999.json",
    );
    expect(ledgerModuleIdsForShardPath(bookShard, root)).toEqual([
      RESOLVED_LEDGER_INDEX_ID,
      bookHashId,
    ]);
    onAll?.("unlink", bookShard);
    expect(invalidated).toHaveBeenCalledWith(bookHashModule);
    expect(sent).toHaveBeenCalledTimes(2);

    const resurrectedMonolith = join(
      root,
      "core",
      "generated-book-hashes",
      "spanish.json",
    );
    expect(ledgerModuleIdsForShardPath(resurrectedMonolith, root)).toEqual([
      RESOLVED_LEDGER_INDEX_ID,
    ]);
    invalidated.mockClear();
    onAll?.("add", resurrectedMonolith);
    expect(invalidated).toHaveBeenCalledTimes(1);
    expect(invalidated).toHaveBeenCalledWith(indexModule);
    expect(sent).toHaveBeenCalledTimes(3);

    const malformedOwner = join(
      root,
      "core",
      "generated-book-hashes",
      "ghost.tmp",
    );
    expect(ledgerModuleIdsForShardPath(malformedOwner, root)).toEqual([
      RESOLVED_LEDGER_INDEX_ID,
    ]);
    invalidated.mockClear();
    onAll?.("unlink", malformedOwner);
    expect(invalidated).toHaveBeenCalledTimes(1);
    expect(invalidated).toHaveBeenCalledWith(indexModule);
    expect(sent).toHaveBeenCalledTimes(4);

    onAll?.("add", join(root, "..", "outside", "chapters.d", "0001.json"));
    expect(sent).toHaveBeenCalledTimes(4);
  });

  it("keeps the eager index proportional to tracks, not authored shards", () => {
    const watched: string[] = [];
    const source = loadHumanLanguageLedgerModule(
      RESOLVED_LEDGER_INDEX_ID,
      root,
      (path) => watched.push(path),
    )!;
    const tracks = loadLanguageRegistry(root).languages.map(
      (track) => track.id,
    );

    expect(source.match(/curriculumLoaders\[/g)).toHaveLength(tracks.length);
    expect(source.match(/chapterLoaders\[/g)).toHaveLength(tracks.length);
    expect(source.match(/bookHashLoaders\[/g)).toHaveLength(tracks.length);
    expect(source).not.toMatch(
      /curriculum\.d|chapters\.d|generated-book-hashes|\.json/,
    );
    expect(watched).toContain(join(root, "core", "languages.json"));
  });

  it("does not make the browser discover generated chapter owners", () => {
    const consumer = readFileSync(
      join(process.cwd(), "src", "bookhashes.ts"),
      "utf8",
    );
    expect(consumer).toContain("bookHashLoaders");
    expect(consumer).not.toContain("import.meta.glob");
    expect(consumer).not.toContain("generated-book-hashes/*.json");
  });

  it("assembles the shared spine from its canonical shards", () => {
    const watched: string[] = [];
    const source = loadHumanLanguageLedgerModule(
      RESOLVED_LEDGER_INDEX_ID,
      root,
      (path) => watched.push(path),
    )!;
    const match = /export const spine = (.*);\n/.exec(source);
    expect(match).not.toBeNull();
    expect(JSON.parse(match![1])).toEqual(loadCurriculumSpine(root));
    expect(watched.some((path) => path.endsWith("spine.d/_meta.json"))).toBe(
      true,
    );
    expect(watched.some((path) => path.endsWith("core/spine.json"))).toBe(
      false,
    );
  });

  it("assembles one curriculum module from only that track's shards", () => {
    const watched: string[] = [];
    const source = loadHumanLanguageLedgerModule(
      `\0${CURRICULUM_MODULE_PREFIX}spanish`,
      root,
      (path) => watched.push(path),
    )!;
    const spanish = loadLanguageCurricula(root).find(
      (plan) => plan.language === "spanish",
    );
    expect(defaultExport(source)).toEqual(spanish);
    expect(watched.length).toBeGreaterThan(100);
    const dir = join(root, "spanish", "curriculum.d");
    expect(watched.slice(1).every((path) => path.startsWith(`${dir}/`))).toBe(
      true,
    );
  });

  it("assembles current authored chapter capabilities, preserving stale-book detection", () => {
    const watched: string[] = [];
    const source = loadHumanLanguageLedgerModule(
      `\0${CHAPTER_MODULE_PREFIX}spanish`,
      root,
      (path) => watched.push(path),
    )!;
    const spanish = loadTrackChapters(root).find(
      (ledger) => ledger.language === "spanish",
    );
    expect(defaultExport(source)).toEqual(spanish);
    expect(watched.length).toBeGreaterThan(100);
    const dir = join(root, "spanish", "chapters.d");
    expect(watched.slice(1).every((path) => path.startsWith(`${dir}/`))).toBe(
      true,
    );
  });

  it("reconstructs and watches a bounded book-hash child without corpus discovery", () => {
    const temporary = mkdtempSync(join(tmpdir(), "language-ledger-book-hash-"));
    const ownerDirectory = join(
      temporary,
      "core",
      "generated-book-hashes",
      "spanish.d",
    );
    const manifest = {
      version: 1,
      algorithm: "fnv1a64",
      chapters: [
        {
          language: "spanish",
          chapter: 1,
          sourceHash: "fnv1a64:0123456789abcdef",
          lessonIds: ["spanish-lesson-1"],
          tex: "spanish/book/chapters/ch01.tex",
        },
      ],
    };
    try {
      writeBookHashOwner(temporary, "spanish");
      writeLanguageRegistry(temporary, ["spanish"]);

      const watched: string[] = [];
      const source = loadHumanLanguageLedgerModule(
        `\0${BOOK_HASH_MODULE_PREFIX}spanish`,
        temporary,
        (path) => watched.push(path),
      )!;

      expect(defaultExport(source)).toEqual(manifest);
      expect(watched).toEqual([
        join(temporary, "core", "languages.json"),
        join(ownerDirectory, "0001.json"),
        join(ownerDirectory, "_meta.json"),
      ]);
    } finally {
      rmSync(temporary, { recursive: true, force: true });
    }
  });

  it("assembles one generated book manifest from only that track's chapter owners", () => {
    const watched: string[] = [];
    const source = loadHumanLanguageLedgerModule(
      `\0${BOOK_HASH_MODULE_PREFIX}spanish`,
      root,
      (path) => watched.push(path),
    )!;
    const logicalPath = join(
      root,
      "core",
      "generated-book-hashes",
      "spanish.json",
    );
    const expected = readGeneratedBookHashManifest(logicalPath);

    expect(defaultExport(source)).toEqual(expected.manifest);
    expect(watched).toEqual([
      join(root, "core", "languages.json"),
      ...expected.sourcePaths,
    ]);
    expect(watched.length).toBeGreaterThan(100);
    const dir = join(root, "core", "generated-book-hashes", "spanish.d");
    expect(watched.slice(1).every((path) => path.startsWith(`${dir}/`))).toBe(
      true,
    );
  });
});
