import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { defaultCurriculumRoot } from "../src/loader.js";
import type { ParsedLesson } from "../src/parse.js";
import {
  generatedTrackProgressOutputs,
  runTrackProgress,
  TRACK_PROGRESS_DIR,
} from "../src/track-progress-cli.js";
import {
  buildTrackProgress,
  renderTrackProgressCard,
  renderTrackProgressTable,
} from "../src/track-progress.js";
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
    expect(renderTrackProgressCard(tracks[1]!)).toContain("# Alpha progress");
  });

  it("keeps every committed per-language card byte-current", () => {
    const root = defaultCurriculumRoot();
    const outputs = generatedTrackProgressOutputs(root);
    expect(outputs.size).toBe(23);
    for (const [relative, expected] of outputs) {
      expect(relative.startsWith(`${TRACK_PROGRESS_DIR}/`)).toBe(true);
      expect(readFileSync(join(root, relative), "utf8"), relative).toBe(expected);
    }
    expect(runTrackProgress(["--check"], root)).toBe(0);
  });

  it("changes only one output when one track changes", () => {
    const root = defaultCurriculumRoot();
    const outputs = generatedTrackProgressOutputs(root);
    const spanish = outputs.get("progress/spanish.md")!;
    const changed = new Map(outputs);
    changed.set("progress/spanish.md", spanish.replace("Canonical lessons:", "Canonical lessons changed:"));
    expect([...outputs.keys()].filter((path) => outputs.get(path) !== changed.get(path))).toEqual([
      "progress/spanish.md",
    ]);
  });

  it("rejects an unknown CLI mode", () => {
    const stderr = vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runTrackProgress(["--wat"])).toBe(2);
    expect(stderr).toHaveBeenCalledWith(expect.stringContaining("usage:"));
    stderr.mockRestore();
  });
});
