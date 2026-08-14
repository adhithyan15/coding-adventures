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
import { CHAPTER_GATE_CODES, runChapterGates, runPatternGates } from "../src/chapters.js";
import { EXEMPT_TYPES } from "../src/constants.js";
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

function patternLesson(options: {
  introduces?: string[];
  requires?: string[];
  slots?: string;
  examples?: string[];
}) {
  const introduces = options.introduces ?? ["ES-PATTERN-FRAME"];
  const requires = options.requires ?? ["ES-LEX-HABLAR", "ES-LEX-COMER"];
  const slots = options.slots ?? "  infinitive: [ES-LEX-HABLAR, ES-LEX-COMER]";
  const examples = options.examples ?? ["hablaré", "comerás", "hablará"];
  return parseLesson(
    `---\nschema_version: 2\nid: ES-P01-frame\nchapter: 1\ntype: pattern\n` +
      `headword: infinitive + ending\ngloss: a productive frame\n` +
      `requires:\n  knowledge: [${requires.join(", ")}]\n` +
      `introduces:\n  knowledge: [${introduces.join(", ")}]\n` +
      `slots:\n${slots}\n---\n\n# A frame\n\n` +
      `## Warm-up\n<!-- hl-knowledge: introduces=[]; assesses=[] -->\n\nRecall the verbs.\n\n` +
      `## Grammar Lens: the frame\n<!-- hl-knowledge: introduces=[${introduces.join(", ")}]; assesses=[] -->\n\nNotice it.\n\n` +
      `## Guided Practice\n<!-- hl-knowledge: introduces=[]; assesses=[${introduces[0]}] -->\n\n` +
      `${examples.map((example) => `- ${example}`).join("\n")}\n\n` +
      `## Wrap-up Recall\n<!-- hl-knowledge: introduces=[]; assesses=[${introduces[0]}] -->\n\nRecall it.\n`,
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
    expect(EXEMPT_TYPES).toContain("pattern");
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

describe("the pattern gates", () => {
  it("CONTROL: one pattern atom, closed slots, and three productions fire nothing", () => {
    const parsed = patternLesson({});
    expect(parsed.patternSlots).toEqual([
      { name: "infinitive", fillers: ["ES-LEX-HABLAR", "ES-LEX-COMER"] },
    ]);
    expect(runPatternGates([parsed])).toEqual([]);
  });

  it("pattern-multiple-atoms: the pattern atom must be the lesson's only introduction", () => {
    const findings = runPatternGates([
      patternLesson({ introduces: ["ES-PATTERN-FRAME", "ES-GRAMMAR-EXTRA"] }),
    ]);
    expect(findings.map((finding) => finding.code)).toContain("pattern-multiple-atoms");
  });

  it("pattern-missing-production: fewer than three instantiations are not productive", () => {
    const findings = runPatternGates([patternLesson({ examples: ["hablaré", "comerás"] })]);
    expect(findings.map((finding) => finding.code)).toContain("pattern-missing-production");
  });

  it("pattern-missing-production: repeated copies do not count as distinct instantiations", () => {
    const findings = runPatternGates([
      patternLesson({ examples: ["hablaré", "hablaré", "hablaré"] }),
    ]);
    expect(findings.map((finding) => finding.code)).toContain("pattern-missing-production");
  });

  it("pattern-slot-not-closed: every declared filler must be required knowledge", () => {
    const findings = runPatternGates([
      patternLesson({ slots: "  infinitive: [ES-LEX-HABLAR, ES-LEX-VIVIR]" }),
    ]);
    expect(findings.map((finding) => finding.code)).toContain("pattern-slot-not-closed");
    expect(findings[0]?.message).toContain("ES-LEX-VIVIR");
  });

  it("pattern-slot-not-closed: a pattern cannot omit or scalarize its slot list", () => {
    const missing = patternLesson({ slots: "" });
    const scalar = patternLesson({ slots: "  infinitive: ES-LEX-HABLAR" });
    expect(runPatternGates([missing]).map((finding) => finding.code)).toContain(
      "pattern-slot-not-closed",
    );
    expect(runPatternGates([scalar]).map((finding) => finding.code)).toContain(
      "pattern-slot-not-closed",
    );
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
    // +1 to both: Tamil chapter 39, ledgered in chapters.json, declared in
    // book-generation.json and \input into tamil/book/book.tex. All three are needed —
    // the first alone fails the book-cli "ledgered chapter into its book" gate.
    expect(report.summary.bookChapters).toBeGreaterThanOrEqual(725) // FLOOR — content only grows; see the note at the top of this file; // +8: HL-C94 splits the four over-budget opening chapters into twelve // +16: vocabulary wave 4, 4 tracks x 4 chapters // +4: HL-C98 gives the first paradigm one cell per chapter (3 teaching + review + synthesis) // +15: vocabulary wave 5 (persian +3, telugu +6, malayalam +6) // +4: HL-C88 slices 5-6 (Spanish friend-ending chapters) // +3: HL-C88 slice 8 // +12: vocabulary wave 6, round 2 (russian +3, persian +3, urdu +3, bengali +3) // +3: HL-C113 (B1 si-condition rung) // +3: HL-C113 preterite plural // HL-C113 preterite close // HL-C113: HL-C113 imperfect subjunctive
    expect(report.summary.declaredChapters).toBeGreaterThanOrEqual(725) // FLOOR — content only grows; see the note at the top of this file; // +98: handwritten capability closure // +4: HL-C98 // +15: vocabulary wave 5 // +4: HL-C88 slices 5-6 // +3: HL-C88 slice 8 // +12: vocabulary wave 6 // +3: HL-C113 (B1 si-condition rung) // +3: HL-C113 preterite plural // HL-C113 preterite close // HL-C113: HL-C113 imperfect subjunctive
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
    // to pass. Chapter 10 now assesses all nine atoms in its terminal checkpoint.
    // 27 -> 29, and both new members are Tamil, both for the chapter-13 reason. Extending
    // the writing strand to cover chapters 2-3's glyphs dropped a script lesson into four
    // previously all-speaking chapters, and none of those four payoffs was widened:
    //   tamil:25 gained TA-W10's two atoms, so its payoff fell from 2/3 to 2/5 (0.40)
    //   tamil:27 gained TA-W11's three atoms, so its payoff fell from 2/2 to 2/5 (0.40)
    // tamil:29 and tamil:31 took the same kind of hit and did NOT join: each landed on
    // exactly 2/4 (0.50), which clears the floor rather than falling below it. So this is
    // +2 and not +4, and the difference is one atom of arithmetic, not a difference in
    // kind. Both new members are recorded in tamil/chapters.json's own payoff notes.
    expect(report.summary.payoffsNotRepresentative).toBe(69); // +1: HL-C88 slices 5-6 // HL12: +1. One chapter's payoff ratio falls below the 0.5 floor because a recognition segment adds an atom its chapter payoff does not name -- a letter is not a thing the chapter promises the reader can DO // HL12 payment two: +3, all Hindi, and the arithmetic is exact. Chapters 10, 11 and 12 each held 4 atoms with a payoff assessing 2 -- 2/4 = 0.50, sitting exactly ON the floor. One recognition segment adds one atom to each, so each becomes 2/5 = 0.40. The payoffs are NOT widened to absorb it: a chapter promises something the reader can DO with the language, and recognising a character is the other ramp, which HL12 section 2.1 keeps separate on purpose. Recorded in hindi/chapters.json's own payoff notes // HL-C154: Tamil's letter ledger completed — 15 more one-character segments, 24/24 positions taught // HL-C156: the letter ledgers replicated to all six — 85 one-character segments, 133/144 positions taught // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1
  });

  it("names the tracks whose chapter debt is already zero", () => {
    const { books: corpus, lessons } = loadEverything();
    const report = runChapterGates({
      books: corpus,
      lessons,
      trackChapters: loadTrackChapters(),
      policy: loadChapterPolicy(),
    });
    // HL-C156: kannada, telugu leave this list — each now carries
    // one-character script segments whose typed-atom payoff debt is not yet paid.
    // Capability closure makes nine more tracks clean. Tracks omitted from this list
    // still carry typed-atom payoff debt, even though none lacks a chapter capability.
    expect(report.tracks.filter((t) => t.clean).map((t) => t.language).sort()).toEqual([
      "bengali",
      "chinese",
      "french",
      "gujarati",
      "italian",
      "japanese",
      "latin",
      "marathi",
      "portuguese",
      "punjabi",
    ]);
  });

  it("validates the first canonical pattern lesson", () => {
    const { books: corpus, lessons } = loadEverything();
    const report = runChapterGates({
      books: corpus,
      lessons,
      trackChapters: loadTrackChapters(),
      policy: loadChapterPolicy(),
    });
    const patternLessons = lessons.filter((lesson) => lesson.realization.type === "pattern");
    // Still one. The friends arc (HL-C88) teaches productive ENDING rules, and
    // they are deliberately `grammar` rather than `pattern`: this corpus
    // reserves `pattern` for a slot-filling production with a single
    // `-PATTERN-` atom, which an ending correspondence is not.
    expect(patternLessons.map((lesson) => lesson.realization.lessonId)).toEqual([
      "ES-C17-comer-futuro",
    ]);
    expect(patternLessons[0]?.patternSlots).toEqual([
      { name: "infinitive", fillers: ["ES-LEX-COMER", "ES-LEX-BEBER"] },
      { name: "object", fillers: ["ES-LEX-CAFE"] },
    ]);
    expect(report.findings.filter((finding) => finding.code.startsWith("pattern-"))).toEqual([]);
  });

  // ---------------------------------------------------------------------------
  // `sequence` and `chapter` must agree about reading order.
  //
  // A lesson carries both: `sequence` orders lessons within a track, `chapter`
  // groups them. Nothing forced them to agree, and twice in one day an inserted
  // chapter took a sequence INSIDE the span of the chapter it displaced, so
  // sorting the corpus by `sequence` alone walked chapter numbers backwards.
  //
  // Neither instance was caught by anything. `readingOrder` sorts chapter-first,
  // so every existing gate stayed green while `sequence` quietly stopped being a
  // valid standalone ordering key -- and any consumer that trusts it renders
  // chapter 67 before chapter 66.
  //
  // This is the missing check. It is deliberately corpus-wide rather than
  // Spanish-only: the defect is a property of how chapters are inserted, not of
  // one track.
  it("keeps chapter numbers non-decreasing when lessons are sorted by sequence", () => {
    const { lessons } = loadEverything();
    const byTrack = new Map<string, { sequence: number; chapter: number; id: string }[]>();
    for (const lesson of lessons) {
      const sequence = Number(lesson.frontmatter.sequence);
      const chapter = lesson.realization.chapter;
      if (!Number.isFinite(sequence)) continue;
      const id = lesson.realization.lessonId;
      const group = byTrack.get(lesson.language) ?? [];
      group.push({ sequence, chapter, id });
      byTrack.set(lesson.language, group);
    }
    const regressions: string[] = [];
    for (const [language, group] of byTrack) {
      const ordered = [...group].sort((a, b) => a.sequence - b.sequence);
      for (let i = 1; i < ordered.length; i++) {
        const previous = ordered[i - 1]!;
        const current = ordered[i]!;
        if (current.chapter < previous.chapter) {
          regressions.push(
            `${language}: ${previous.id} (seq ${previous.sequence}, ch ${previous.chapter}) ` +
              `is followed by ${current.id} (seq ${current.sequence}, ch ${current.chapter})`,
          );
        }
      }
    }
    expect(regressions).toEqual([]);
  });
});
