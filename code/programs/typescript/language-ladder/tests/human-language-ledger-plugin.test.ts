import { mkdtempSync, mkdirSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { ViteDevServer } from "vite";
import { describe, expect, it, vi } from "vitest";
import {
  CURRICULUM_MODULE_PREFIX,
  CHAPTER_MODULE_PREFIX,
  LEDGER_INDEX_ID,
  RESOLVED_LEDGER_INDEX_ID,
  humanLanguageLedgerPlugin,
  ledgerModuleIdsForShardPath,
  loadHumanLanguageLedgerModule,
  resolveHumanLanguageLedgerId,
} from "../human-language-ledger-plugin.ts";
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

describe("the bounded human-language ledger virtual modules", () => {
  it("resolves only the public index and its safe per-track children", () => {
    expect(resolveHumanLanguageLedgerId(LEDGER_INDEX_ID)).toBe(RESOLVED_LEDGER_INDEX_ID);
    expect(resolveHumanLanguageLedgerId(`${CURRICULUM_MODULE_PREFIX}spanish`))
      .toBe(`\0${CURRICULUM_MODULE_PREFIX}spanish`);
    expect(resolveHumanLanguageLedgerId(`${CHAPTER_MODULE_PREFIX}../../etc`))
      .toBeNull();
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
        loadHumanLanguageLedgerModule(RESOLVED_LEDGER_INDEX_ID, temporary, () => {}),
      ).toThrow(/unsafe registry track id/);

      writeFileSync(
        join(temporary, "core", "languages.json"),
        JSON.stringify({ languages: [{ id: "spanish" }, { id: "spanish" }] }),
      );
      expect(() =>
        loadHumanLanguageLedgerModule(RESOLVED_LEDGER_INDEX_ID, temporary, () => {}),
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
        loadHumanLanguageLedgerModule(RESOLVED_LEDGER_INDEX_ID, temporary, () => {}),
      ).toThrow(/ledger parent escapes curriculum root/);
    } finally {
      rmSync(temporary, { recursive: true, force: true });
      rmSync(outside, { recursive: true, force: true });
    }
  });

  it("invalidates add/unlink modules and explicitly reloads the browser", () => {
    const watched = vi.fn();
    const invalidated = vi.fn();
    const sent = vi.fn();
    const indexModule = { id: RESOLVED_LEDGER_INDEX_ID };
    const curriculumId = `\0${CURRICULUM_MODULE_PREFIX}spanish`;
    const curriculumModule = { id: curriculumId };
    let onAll: ((event: string, path: string) => void) | undefined;
    const server = {
      watcher: {
        add: watched,
        on: vi.fn((event: string, handler: (event: string, path: string) => void) => {
          if (event === "all") onAll = handler;
        }),
      },
      moduleGraph: {
        getModuleById: vi.fn((id: string) => {
          if (id === RESOLVED_LEDGER_INDEX_ID) return indexModule;
          if (id === curriculumId) return curriculumModule;
          return undefined;
        }),
        invalidateModule: invalidated,
      },
      ws: { send: sent },
    } as unknown as ViteDevServer;
    const configure = humanLanguageLedgerPlugin({ curriculumRoot: root }).configureServer;
    expect(typeof configure).toBe("function");
    (configure as (server: ViteDevServer) => void)(server);

    const shard = join(root, "spanish", "curriculum.d", "9999-ES-PATH-NEW.json");
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
    onAll?.("add", join(root, "..", "outside", "chapters.d", "0001.json"));
    expect(sent).toHaveBeenCalledTimes(1);
  });

  it("keeps the eager index proportional to tracks, not authored shards", () => {
    const watched: string[] = [];
    const source = loadHumanLanguageLedgerModule(
      RESOLVED_LEDGER_INDEX_ID,
      root,
      (path) => watched.push(path),
    )!;
    const tracks = loadLanguageRegistry(root).languages.map((track) => track.id);

    expect(source.match(/curriculumLoaders\[/g)).toHaveLength(tracks.length);
    expect(source.match(/chapterLoaders\[/g)).toHaveLength(tracks.length);
    expect(source).not.toMatch(/curriculum\.d|chapters\.d|\.json/);
    expect(watched).toContain(join(root, "core", "languages.json"));
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
    expect(watched.some((path) => path.endsWith("spine.d/_meta.json"))).toBe(true);
    expect(watched.some((path) => path.endsWith("core/spine.json"))).toBe(false);
  });

  it("assembles one curriculum module from only that track's shards", () => {
    const watched: string[] = [];
    const source = loadHumanLanguageLedgerModule(
      `\0${CURRICULUM_MODULE_PREFIX}spanish`,
      root,
      (path) => watched.push(path),
    )!;
    const spanish = loadLanguageCurricula(root).find((plan) => plan.language === "spanish");
    expect(defaultExport(source)).toEqual(spanish);
    expect(watched.length).toBeGreaterThan(100);
    const dir = join(root, "spanish", "curriculum.d");
    expect(watched.every((path) => path.startsWith(`${dir}/`))).toBe(true);
  });

  it("assembles current authored chapter capabilities, preserving stale-book detection", () => {
    const watched: string[] = [];
    const source = loadHumanLanguageLedgerModule(
      `\0${CHAPTER_MODULE_PREFIX}spanish`,
      root,
      (path) => watched.push(path),
    )!;
    const spanish = loadTrackChapters(root).find((ledger) => ledger.language === "spanish");
    expect(defaultExport(source)).toEqual(spanish);
    expect(watched.length).toBeGreaterThan(100);
    const dir = join(root, "spanish", "chapters.d");
    expect(watched.every((path) => path.startsWith(`${dir}/`))).toBe(true);
  });
});
