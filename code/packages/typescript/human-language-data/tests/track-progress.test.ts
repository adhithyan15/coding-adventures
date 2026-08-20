import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { defaultCurriculumRoot } from "../src/loader.js";
import type { ParsedLesson } from "../src/parse.js";
import {
  generatedTrackProgressReadme,
  replaceTrackProgressSection,
  runTrackProgress,
  TRACK_PROGRESS_END,
  TRACK_PROGRESS_START,
} from "../src/track-progress-cli.js";
import { buildTrackProgress, renderTrackProgressTable } from "../src/track-progress.js";
import type { BookCorpus, LanguageCurriculum, LanguageRegistry } from "../src/types.js";

const lesson = (language: string) => ({ language }) as ParsedLesson;

describe("track progress", () => {
  it("derives registry-ordered counts from lessons, maps, and book data", () => {
    const registry: LanguageRegistry = {
      version: 1,
      languages: [
        { id: "beta", name: "Beta", family: "Test", script: "perso-arabic", status: "active", bridges: [] },
        { id: "alpha", name: "Alpha", family: "Test", script: "latin", status: "active", bridges: [] },
      ],
    };
    const curricula = [
      {
        version: 1,
        language: "alpha",
        path: [{ id: "p", spine_node: "s", lessons: ["a1", "a2"], before: [], inline: [], after: [] }],
        spine: {},
        extensions: [{ id: "e", stage: "A1", kind: "required", category: "grammar", canDo: "x", prerequisites: [], lessons: ["a2", "a3"] }],
      },
    ] as LanguageCurriculum[];
    const books: BookCorpus = {
      books: [{ language: "alpha", entrypoint: "alpha/book/book.tex", chapters: [
        { language: "alpha", chapter: 1, slug: "one", title: "One", source: "one.tex", tex: "" },
        { language: "alpha", chapter: 3, slug: "three", title: "Three", source: "three.tex", tex: "" },
      ] }],
    };

    const tracks = buildTrackProgress(
      registry,
      [lesson("alpha"), lesson("alpha"), lesson("beta")],
      curricula,
      books,
      [{ language: "alpha", chapter: 3 }, { language: "alpha", chapter: 3 }],
    );

    expect(tracks.map((track) => track.id)).toEqual(["beta", "alpha"]);
    expect(tracks[0]).toMatchObject({ canonicalLessons: 1, mappedLessons: 0, bookChapters: 0 });
    expect(tracks[1]).toMatchObject({ canonicalLessons: 2, mappedLessons: 3, bookChapters: 2, latestBookChapter: 3, generatedBookChapters: 1 });
    expect(renderTrackProgressTable(tracks)).toContain(
      "| [Alpha](./alpha/README.md) | Test / Latin | 2 | 3 | 2 chapters; through Ch. 3; 1 generated |",
    );
    expect(renderTrackProgressTable(tracks)).toContain("Test / Perso-Arabic");
  });

  it("replaces one marker pair and preserves CRLF", () => {
    const source = `before\r\n${TRACK_PROGRESS_START}\r\nstale\r\n${TRACK_PROGRESS_END}\r\nafter\r\n`;
    expect(replaceTrackProgressSection(source, "| a |\n| b |")).toBe(
      `before\r\n${TRACK_PROGRESS_START}\r\n| a |\r\n| b |\r\n${TRACK_PROGRESS_END}\r\nafter\r\n`,
    );
  });

  it("fails closed on absent, repeated, or reversed markers", () => {
    expect(() => replaceTrackProgressSection("none", "table")).toThrow(/exactly one/);
    expect(() => replaceTrackProgressSection(`${TRACK_PROGRESS_START}${TRACK_PROGRESS_START}${TRACK_PROGRESS_END}`, "table")).toThrow(/exactly one/);
    expect(() => replaceTrackProgressSection(`${TRACK_PROGRESS_END}${TRACK_PROGRESS_START}`, "table")).toThrow(/exactly one/);
  });

  it("keeps the committed top-level table byte-current and complete", () => {
    const root = defaultCurriculumRoot();
    const expected = generatedTrackProgressReadme(root);
    expect(readFileSync(join(root, "README.md"), "utf8")).toBe(expected);
    expect(expected.match(/^\| \[[^\]]+\]\(\.\/[^/]+\/README\.md\)/gm)).toHaveLength(23);
    expect(runTrackProgress(["--check"], root)).toBe(0);
  });

  it("rejects an unknown CLI mode", () => {
    const stderr = vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runTrackProgress(["--wat"])).toBe(2);
    expect(stderr).toHaveBeenCalledWith(expect.stringContaining("usage:"));
    stderr.mockRestore();
  });
});
