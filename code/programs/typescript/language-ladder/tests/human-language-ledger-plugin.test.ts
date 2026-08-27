import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  CURRICULUM_MODULE_PREFIX,
  CHAPTER_MODULE_PREFIX,
  LEDGER_INDEX_ID,
  RESOLVED_LEDGER_INDEX_ID,
  humanLanguageLedgerPlugin,
  loadHumanLanguageLedgerModule,
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
    const plugin = humanLanguageLedgerPlugin({ curriculumRoot: root });
    expect(plugin.resolveId?.call({} as never, LEDGER_INDEX_ID, undefined, {} as never))
      .toBe(RESOLVED_LEDGER_INDEX_ID);
    expect(plugin.resolveId?.call({} as never, `${CURRICULUM_MODULE_PREFIX}spanish`, undefined, {} as never))
      .toBe(`\0${CURRICULUM_MODULE_PREFIX}spanish`);
    expect(plugin.resolveId?.call({} as never, `${CHAPTER_MODULE_PREFIX}../../etc`, undefined, {} as never))
      .toBeNull();
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
    expect(watched.every((path) => path.includes("/spanish/curriculum.d/"))).toBe(true);
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
    expect(watched.every((path) => path.includes("/spanish/chapters.d/"))).toBe(true);
  });
});
