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

  it("covers all 523 generated and handwritten chapter openings in all 22 books", () => {
    const root = defaultCurriculumRoot();
    const outputs = generatedBookOutputs(root);
    const modalityFiles = [...outputs.entries()].filter(([path]) =>
      path.endsWith("/book/chapter-modalities.tex"),
    );
    expect(modalityFiles).toHaveLength(22);

    const ledgers = loadTrackChapters(root);
    // +1: Tamil chapter 39. Its opening renders "first 3 of 4 lessons", because the
    // writing lesson is last in the chapter — the placement rule TA-W19 established.
    expect(ledgers.flatMap((track) => track.chapters)).toHaveLength(885); // tamil pre-A1 tranche: +35 lessons, +7 chapters (chapters 44-50) //HL-C136 wave I: 42 lessons, 6 chapters — 'Pointing, and Asking', six deixis words per Indic track, re-measured against the merged base after main landed HL-C128 steps 3-10 and HL-C127 // +4: HL-C98 // +15: vocabulary wave 5 // +4: HL-C88 slices 5-6 // +3: HL-C88 slice 8 // +12: vocabulary wave 6 // +3: HL-C113 (B1 si-condition rung) // +3: HL-C113 preterite plural // HL-C113 preterite close // HL-C113: HL-C113 imperfect subjunctive // HL-C137 wave II: +36 lessons, +6 chapters — the first adjectives (big/small/good/new/old) and the placement rule behind them, one chapter per Indic track // HL-C152: +5 lessons, +1 chapter — Spanish realizes SPINE-NEGATE-AND-ASK, completing A2 at 5/5 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C163: +6 -- Sanskrit chapter 16 // HL-C165: +11 -- Sanskrit chapters 17 and 18 // HL-C166: +11 -- Sanskrit chapters 19 and 20 // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C175: +5 -- chapter 272, reading between the lines // HL-C177: +5 -- chapter 273, C1 closes // HL-C178: +5 -- chapter 274, C2 opens // HL-C179: +5 -- chapter 275, fine shades // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C187: +20 -- verb tranche across the five behind tracks // HL-C189: +8 -- Tamil and Sanskrit verb tranche // HL-C190: see/say verbs across four tracks // HL-C192: +24 family words // HL-C194: +16 Spanish words // HL: +35 -- Sanskrit chapters 24-30, 35 pre-A1 vocabulary lessons // HL-C200: +35 telugu pre-A1 lessons, +7 chapters (chapters 46-52) // kannada pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // malayalam pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // hindi pre-A1 tranche: +35 lessons, +7 chapters (chapters 45-51) // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // sanskrit pre-A1 round 2: +35 lessons, +7 chapters (chapters 31-37) // telugu pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // kannada pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59)
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
