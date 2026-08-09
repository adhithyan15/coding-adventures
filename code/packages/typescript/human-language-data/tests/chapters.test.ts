// HL05 chapter-capability gates (HL-C03).
//
// Each gate gets a fixture that fires it and a control that does not, because a rule
// asserted only in its failing direction cannot tell "the gate works" from "the gate
// always fires". The corpus block at the bottom pins the first published snapshot.

import { describe, expect, it } from "vitest";
import {
  chapterTitleFromTex,
  loadChapterPolicy,
  loadEverything,
  loadTrackChapters,
} from "../src/loader.js";
import { CHAPTER_GATE_CODES, runChapterGates } from "../src/chapters.js";
import { parseLesson } from "../src/parse.js";
import type { BookCorpus, ChapterPolicy, TrackChapters } from "../src/types.js";

const POLICY: ChapterPolicy = {
  version: 1,
  payoffRepresentativeness: 0.5,
  maxNewAtomsPerLesson: 3,
  maxNewAtomsPerChapter: 12,
  maxLinearisableTableColumns: 3,
};

it("reads complete LaTeX chapter titles that contain nested formatting commands", () => {
  expect(chapterTitleFromTex("\\chapter{\\emph{Ser} and \\emph{Estar} --- Two Ways to Be}", "fallback"))
    .toBe("\\emph{Ser} and \\emph{Estar} --- Two Ways to Be");
});

/** A lesson that introduces the given atoms via its block directive. */
function lesson(id: string, chapter: number, introduces: string[] = []) {
  const directive =
    introduces.length > 0
      ? `<!-- hl-knowledge: introduces=[${introduces.join(", ")}]; assesses=[] -->\n\n`
      : "";
  return parseLesson(
    `---\nschema_version: 2\nid: ${id}\nchapter: ${chapter}\ntype: word\n` +
      `headword: hola\ngloss: hello\nconcept_tag: GREETING-HELLO\n---\n\n` +
      `# ${id}\n\n## Warm-up\n\n${directive}Say it.\n`,
    "spanish",
  );
}

function books(chapters: Array<{ chapter: number; title: string }>): BookCorpus {
  return {
    books: [
      {
        language: "spanish",
        chapters: chapters.map((c) => ({ ...c, label: `ch:${c.chapter}`, tex: "x".repeat(200) })),
      },
    ],
  } as unknown as BookCorpus;
}

function ledger(chapters: TrackChapters["chapters"]): TrackChapters[] {
  return [{ version: 1, language: "spanish", chapters }];
}

const GOOD_CHAPTER = {
  chapter: 1,
  title: "Greetings",
  label: "ch:1",
  canDo: "I can greet someone.",
  spineNodes: [],
  payoff: {
    lesson: "ES-C01-practice",
    kind: "dialogue" as const,
    summary: "A greeting.",
    assesses: ["ES-LEX-HOLA"],
  },
};

describe("the gate catalogue", () => {
  it("publishes all nine HL05 codes", () => {
    expect(CHAPTER_GATE_CODES).toHaveLength(9);
    expect(CHAPTER_GATE_CODES).toContain("chapter-missing-capability");
    expect(CHAPTER_GATE_CODES).toContain("pattern-multiple-atoms");
  });
});

describe("the chapter gates", () => {
  it("CONTROL: a well-formed chapter fires nothing", () => {
    const report = runChapterGates({
      books: books([{ chapter: 1, title: "Greetings" }]),
      lessons: [lesson("ES-C01-hola", 1, ["ES-LEX-HOLA"]), lesson("ES-C01-practice", 1)],
      trackChapters: ledger([GOOD_CHAPTER]),
      policy: POLICY,
    });
    expect(report.findings).toEqual([]);
    expect(report.tracks[0]?.clean).toBe(true);
  });

  it("chapter-missing-capability: a book chapter with no ledger entry", () => {
    const report = runChapterGates({
      books: books([
        { chapter: 1, title: "Greetings" },
        { chapter: 2, title: "Farewells" },
      ]),
      lessons: [lesson("ES-C01-hola", 1, ["ES-LEX-HOLA"]), lesson("ES-C01-practice", 1)],
      trackChapters: ledger([GOOD_CHAPTER]),
      policy: POLICY,
    });
    expect(report.findings.map((f) => f.code)).toEqual(["chapter-missing-capability"]);
    expect(report.findings[0]?.chapter).toBe(2);
    expect(report.summary.chaptersWithoutCapability).toBe(1);
  });

  it("chapter-unknown-payoff-lesson: the payoff names a lesson that does not exist", () => {
    const report = runChapterGates({
      books: books([{ chapter: 1, title: "Greetings" }]),
      lessons: [lesson("ES-C01-hola", 1, ["ES-LEX-HOLA"])],
      trackChapters: ledger([GOOD_CHAPTER]),
      policy: POLICY,
    });
    expect(report.findings.map((f) => f.code)).toContain("chapter-unknown-payoff-lesson");
  });

  it("chapter-payoff-not-closed: the payoff assesses an atom taught LATER", () => {
    const report = runChapterGates({
      books: books([{ chapter: 1, title: "Greetings" }]),
      lessons: [
        lesson("ES-C01-practice", 1),
        // Taught in chapter 2 — the reader does not have it yet at chapter 1.
        lesson("ES-C02-adios", 2, ["ES-LEX-ADIOS"]),
      ],
      trackChapters: ledger([
        { ...GOOD_CHAPTER, payoff: { ...GOOD_CHAPTER.payoff, assesses: ["ES-LEX-ADIOS"] } },
      ]),
      policy: POLICY,
    });
    expect(report.findings.map((f) => f.code)).toContain("chapter-payoff-not-closed");
  });

  it("CONTROL: an atom taught in an EARLIER chapter is closed", () => {
    const report = runChapterGates({
      books: books([{ chapter: 2, title: "Farewells" }]),
      lessons: [lesson("ES-C01-hola", 1, ["ES-LEX-HOLA"]), lesson("ES-C02-practice", 2)],
      trackChapters: ledger([
        {
          ...GOOD_CHAPTER,
          chapter: 2,
          title: "Farewells",
          payoff: { ...GOOD_CHAPTER.payoff, lesson: "ES-C02-practice", assesses: ["ES-LEX-HOLA"] },
        },
      ]),
      policy: POLICY,
    });
    expect(report.findings.map((f) => f.code)).not.toContain("chapter-payoff-not-closed");
  });

  it("chapter-payoff-not-representative: assessing one atom of four is below the floor", () => {
    const report = runChapterGates({
      books: books([{ chapter: 1, title: "Greetings" }]),
      lessons: [
        lesson("ES-C01-a", 1, ["A1", "A2", "A3", "A4"]),
        lesson("ES-C01-practice", 1),
      ],
      trackChapters: ledger([
        { ...GOOD_CHAPTER, payoff: { ...GOOD_CHAPTER.payoff, assesses: ["A1"] } },
      ]),
      policy: POLICY,
    });
    const codes = report.findings.map((f) => f.code);
    expect(codes).toContain("chapter-payoff-not-representative");
    // Half of four clears the same floor — the rule is a threshold, not a demand for all.
    const ok = runChapterGates({
      books: books([{ chapter: 1, title: "Greetings" }]),
      lessons: [lesson("ES-C01-a", 1, ["A1", "A2", "A3", "A4"]), lesson("ES-C01-practice", 1)],
      trackChapters: ledger([
        { ...GOOD_CHAPTER, payoff: { ...GOOD_CHAPTER.payoff, assesses: ["A1", "A2"] } },
      ]),
      policy: POLICY,
    });
    expect(ok.findings.map((f) => f.code)).not.toContain("chapter-payoff-not-representative");
  });

  it("chapter-duplicate: two entries for one chapter number", () => {
    const report = runChapterGates({
      books: books([{ chapter: 1, title: "Greetings" }]),
      lessons: [lesson("ES-C01-hola", 1, ["ES-LEX-HOLA"]), lesson("ES-C01-practice", 1)],
      trackChapters: ledger([GOOD_CHAPTER, { ...GOOD_CHAPTER }]),
      policy: POLICY,
    });
    expect(report.findings.map((f) => f.code)).toContain("chapter-duplicate");
  });

  it("chapter-title-drift: the book and the ledger disagree about the name", () => {
    const report = runChapterGates({
      books: books([{ chapter: 1, title: "Hello and Good Day" }]),
      lessons: [lesson("ES-C01-hola", 1, ["ES-LEX-HOLA"]), lesson("ES-C01-practice", 1)],
      trackChapters: ledger([GOOD_CHAPTER]),
      policy: POLICY,
    });
    expect(report.findings.map((f) => f.code)).toContain("chapter-title-drift");
  });

  it("collects across every chapter instead of stopping at the first", () => {
    const report = runChapterGates({
      books: books([
        { chapter: 1, title: "Greetings" },
        { chapter: 2, title: "Farewells" },
        { chapter: 3, title: "Numbers" },
      ]),
      lessons: [lesson("ES-C01-practice", 1)],
      trackChapters: ledger([GOOD_CHAPTER]),
      policy: POLICY,
    });
    // Chapters 2 and 3 are both missing; the run does not stop after chapter 2.
    expect(report.findings.filter((f) => f.code === "chapter-missing-capability")).toHaveLength(2);
  });
});

describe("corpus snapshot", () => {
  // The first published measurement (HL-C03). These are DEBT counts, not a pass mark:
  // the gates are report-only precisely because this debt predates them. Ratchet them
  // DOWN as tracks are authored; never up.
  //
  // The `payoffsNotClosed: 0` line is the one that matters most. It was 279 — every
  // authored chapter — on the first run, which was a bug in this module rather than the
  // corpus: `introduces.knowledge` is a FLAT dotted frontmatter key plus a block-level
  // directive, and reading a nested `introduces.knowledge` object returned undefined for
  // every lesson, emptying the "taught so far" set. A gate that reports total failure is
  // reporting on itself.
  it("pins the first published chapter-gate snapshot", () => {
    const { books: corpus, lessons } = loadEverything();
    const report = runChapterGates({
      books: corpus,
      lessons,
      trackChapters: loadTrackChapters(),
      policy: loadChapterPolicy(),
    });
    // HL-C63 authored the 98 capabilities that had lagged behind already published,
    // handwritten chapters. The book total therefore stays at 513 while the declared
    // capability total catches up from 415 to 513 and the missing-capability debt falls
    // from 98 to zero. A future chapter without a `canDo`/`payoff` will move these totals
    // apart again, which is exactly what this trio is here to catch.
    expect(report.summary.bookChapters).toBe(513); // +16: vocabulary wave 4, 4 tracks x 4 chapters
    expect(report.summary.declaredChapters).toBe(513); // +98: handwritten capability closure
    expect(report.summary.chaptersWithoutCapability).toBe(0);
    expect(report.summary.payoffsNotClosed).toBe(0);
    expect(report.summary.unknownPayoffLessons).toBe(0);
    expect(report.summary.titleDrift).toBe(0);
    expect(report.summary.duplicateChapters).toBe(0);
    // 24 -> 25 with russian chapter 3. Its payoff is the last lesson by sequence --
    // the chapter has no terminal practice lesson -- so it assesses 6 of 18 atoms,
    // below the 0.5 floor. That is recorded in the chapter's own `payoff.note` and is
    // a deliberate trade: a chapter with an opening and a thin payoff is better than
    // one with neither, and HL-C25 exists to author real payoff lessons.
    // Still 25, but the Tamil member CHANGED and the total hides it. tamil:1 left the
    // list (it introduces no atoms now, so the representativeness gate skips it
    // entirely rather than passing it), and tamil:13 joined at 1/4 — it gained
    // TA-W04-i-sign-write-nandri when the writing strand was spread out and its payoff
    // was not widened. Both are recorded in tamil/chapters.json's own notes.
    // Empty `assesses` lists on schema-v1 chapters remain unscored rather than pretending
    // to pass. Chapter 10 now assesses all nine atoms in its terminal checkpoint, so
    // the current typed-atom corpus still exposes 27 genuinely thin payoffs.
    expect(report.summary.payoffsNotRepresentative).toBe(27);
  });

  it("names the tracks whose chapter debt is already zero", () => {
    const { books: corpus, lessons } = loadEverything();
    const report = runChapterGates({
      books: corpus,
      lessons,
      trackChapters: loadTrackChapters(),
      policy: loadChapterPolicy(),
    });
    // Capability closure makes nine more tracks clean. Tracks omitted from this list
    // still carry typed-atom payoff debt, even though none lacks a chapter capability.
    expect(report.tracks.filter((t) => t.clean).map((t) => t.language).sort()).toEqual([
      "bengali",
      "chinese",
      "french",
      "gujarati",
      "italian",
      "japanese",
      "kannada",
      "latin",
      "marathi",
      "portuguese",
      "punjabi",
      "telugu",
    ]);
  });

  it("finds no pattern lessons yet, because HL-C05 has not landed", () => {
    const { books: corpus, lessons } = loadEverything();
    const report = runChapterGates({
      books: corpus,
      lessons,
      trackChapters: loadTrackChapters(),
      policy: loadChapterPolicy(),
    });
    const pattern = report.findings.filter((f) => f.code.startsWith("pattern-"));
    // Zero is the correct answer, not a stub: the rules are wired so the first authored
    // pattern lesson is checked the moment it exists.
    expect(pattern).toEqual([]);
  });
});
