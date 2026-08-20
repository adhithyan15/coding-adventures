import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { renderBookChapterModalities } from "../src/book.js";
import { generatedBookOutputs } from "../src/book-cli.js";
import { defaultCurriculumRoot, loadTrackChapters } from "../src/loader.js";
import type { ChapterModality } from "../src/modality.js";

function chapter(overrides: Partial<ChapterModality> = {}): ChapterModality {
  return {
    language: "test",
    chapter: 1,
    lessonCount: 2,
    voice: 1,
    sight: 1,
    pen: 0,
    coreVoice: 1,
    modalities: ["voice", "sight"],
    drivablePrefix: 1,
    firstNonVoiceLesson: "TEST-C01-b",
    ...overrides,
  };
}

describe("book chapter modality projection", () => {
  it("prints full modes and the core-drivable prefix with font-independent signs", () => {
    const tex = renderBookChapterModalities("test", [chapter()]);
    expect(tex).toContain("\\hlvoicesign{}~\\textbf{voice} (1)");
    expect(tex).toContain("\\hlsightsign{}~\\textbf{eyes} (1)");
    expect(tex).not.toContain("\\hlpensign{}~\\textbf{pen}");
    expect(tex).toContain("\\textbf{Hands-free start:} first 1 of 2 lessons.");
    expect(tex).not.toContain("AddToHook");
    expect(tex).not.toMatch(/[🚗👁✍]/u);
  });

  it("says all when the whole chapter is reachable hands-free", () => {
    const tex = renderBookChapterModalities("test", [
      chapter({ sight: 0, voice: 2, modalities: ["voice"], drivablePrefix: 2 }),
    ]);
    expect(tex).toContain("\\textbf{Hands-free start:} all 2 lessons.");
  });

  it("names an empty hands-free prefix without the awkward 'first zero' phrasing", () => {
    const tex = renderBookChapterModalities("test", [chapter({ drivablePrefix: 0 })]);
    expect(tex).toContain("\\textbf{Hands-free start:} none of the 2 lessons.");
    expect(tex).not.toContain("first 0");
  });

  it("rejects cross-track, duplicate, empty, or impossible chapter data", () => {
    expect(() => renderBookChapterModalities("test", [])).toThrow(/no chapter modality data/);
    expect(() =>
      renderBookChapterModalities("test", [chapter({ language: "other" })]),
    ).toThrow(/belongs to other/);
    expect(() => renderBookChapterModalities("test", [chapter(), chapter()])).toThrow(
      /duplicate or invalid/,
    );
    expect(() =>
      renderBookChapterModalities("test", [chapter({ drivablePrefix: 3 })]),
    ).toThrow(/invalid modality counts/);
  });

  it("covers every generated and handwritten chapter opening in all 23 books", () => {
    const root = defaultCurriculumRoot();
    const outputs = generatedBookOutputs(root);
    const modalityFiles = [...outputs.entries()].filter(([path]) =>
      path.endsWith("/book/chapter-modalities.tex"),
    );
    expect(modalityFiles).toHaveLength(23);

    const ledgers = loadTrackChapters(root);
    // +1: Tamil chapter 39. Its opening renders "first 3 of 4 lessons", because the
    // writing lesson is last in the chapter — the placement rule TA-W19 established.
    expect(ledgers.flatMap((track) => track.chapters)).toHaveLength(1004); // +1: Marwadi writing-first greeting starter
    for (const track of ledgers) {
      const path = `${track.language}/book/chapter-modalities.tex`;
      const tex = outputs.get(path);
      expect(tex, path).toBeDefined();
      for (const entry of track.chapters) {
        expect(tex, `${path} chapter ${entry.chapter}`).toContain(
          `\\csname hlchaptermodality${entry.chapter}\\endcsname`,
        );
      }
      const book = readFileSync(join(root, track.language, "book", "book.tex"), "utf8");
      expect(book).toContain("\\input{chapter-modalities}");
      for (const entry of track.chapters) {
        const chapterInput = book.match(
          new RegExp(`\\\\input\\{(chapters/ch${String(entry.chapter).padStart(2, "0")}-[^}]+)\\}`),
        )?.[1];
        expect(chapterInput, `${track.language} book chapter ${entry.chapter}`).toBeDefined();
        const relative = `${track.language}/book/${chapterInput}.tex`;
        const chapterTex = outputs.get(relative) ?? readFileSync(join(root, relative), "utf8");
        expect(chapterTex, relative).toMatch(
          new RegExp(`\\\\chapter\\{[^}]+\\}[\\s\\S]*?\\\\label\\{[^}]+\\}\\s*\\\\hlchaptermodality\\{${entry.chapter}\\}`),
        );
      }
    }
  });
});
