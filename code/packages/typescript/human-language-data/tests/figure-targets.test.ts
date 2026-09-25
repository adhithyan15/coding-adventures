// HL-C443 — a single-letter writing lesson gets its filmstrip without anyone
// declaring it, and every filmstrip the curriculum resolves is actually PRINTED.
//
// The second half is the gate that was missing: three filmstrip SVGs were
// generated for months and no book showed any of them, because nothing checked
// that a generated figure reached a page.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { resolvedFigureTargets } from "../src/figure-cli.js";
import type { FigureTarget, ScriptFilmstripTarget } from "../src/figure.js";
import {
  DERIVED_FILMSTRIP_SCRIPTS,
  filmstripCandidates,
  filmstripImageMarkdown,
  withDerivedFilmstrips,
  withFilmstripImages,
  writingLetterOf,
} from "../src/figure-targets.js";
import { defaultCurriculumRoot, loadLessons } from "../src/loader.js";
import type { ParsedLesson } from "../src/parse.js";

function lesson(
  id: string,
  options: { language?: string; type?: string; headword?: string; blocks?: string[] } = {},
): ParsedLesson {
  const blocks = (options.blocks ?? ["Warm-up", "Writing: x", "Wrap-up Recall"]).map((title) => ({
    type: "explanation",
    title,
    markdown: `body of ${title}\n`,
  }));
  return {
    language: options.language ?? "tamil",
    realization: {
      lessonId: id,
      type: options.type ?? "writing",
      headword: options.headword ?? "அ",
    },
    blocks,
  } as unknown as ParsedLesson;
}

describe("which lessons are filmstrip candidates", () => {
  it("takes a one-letter writing lesson with a Writing block", () => {
    expect(writingLetterOf(lesson("TA-S1"))).toBe("அ");
  });

  it("counts a letter as one grapheme, so a consonant with its vowel sign is still one", () => {
    expect(writingLetterOf(lesson("TA-S2", { headword: "கா" }))).toBe("கா");
  });

  it("leaves several letters, a word, a non-writing lesson or a missing Writing block alone", () => {
    expect(writingLetterOf(lesson("TA-S3", { headword: "வ, க" }))).toBeUndefined();
    expect(writingLetterOf(lesson("TA-S4", { headword: "வணக்கம்" }))).toBeUndefined();
    expect(writingLetterOf(lesson("TA-C5", { type: "word" }))).toBeUndefined();
    expect(writingLetterOf(lesson("TA-S6", { blocks: ["Warm-up", "Guided Practice"] }))).toBeUndefined();
  });

  it("only draws candidates from switched-on tracks, into that track's own book", () => {
    const candidates = filmstripCandidates([
      lesson("TA-S1"),
      lesson("BN-S1", { language: "bengali", headword: "অ" }),
    ]);
    expect(candidates).toEqual([
      {
        kind: "script-filmstrip",
        lessonId: "TA-S1",
        script: "tamil",
        glyph: "அ",
        output: "tamil/book/figures/TA-S1-filmstrip.svg",
      },
    ]);
    expect(Object.keys(DERIVED_FILMSTRIP_SCRIPTS)).toContain("tamil");
  });
});

describe("merging declared and derived targets", () => {
  const derived: ScriptFilmstripTarget[] = filmstripCandidates([
    lesson("TA-S1"),
    lesson("TA-S2", { headword: "ங" }),
  ]);

  it("keeps only the letters the ductus data cites", () => {
    const merged = withDerivedFilmstrips([], derived, (_script, glyph) => glyph === "அ");
    expect(merged.map((target) => target.lessonId)).toEqual(["TA-S1"]);
  });

  it("lets a declared target win for its lesson", () => {
    const declared: FigureTarget = {
      kind: "script-filmstrip",
      lessonId: "TA-S1",
      script: "tamil",
      glyph: "ஆ",
      output: "tamil/book/figures/TA-S1-custom.svg",
    };
    const merged = withDerivedFilmstrips([declared], derived, () => true);
    expect(merged.filter((target) => target.lessonId === "TA-S1")).toEqual([declared]);
  });
});

describe("placing the image", () => {
  const target = filmstripCandidates([lesson("TA-S1")])[0]!;

  it("puts the filmstrip at the top of the Writing block and nowhere else", () => {
    const [placed] = withFilmstripImages([lesson("TA-S1")], [target]);
    const writing = placed!.blocks.find((block) => block.title.startsWith("Writing"))!;
    expect(writing.markdown.startsWith(filmstripImageMarkdown(target))).toBe(true);
    expect(writing.markdown).toContain("body of Writing: x");
    expect(placed!.blocks.filter((block) => block.markdown.includes("filmstrip")).length).toBe(1);
  });

  it("falls back to the first Script block when a track presents its letter there", () => {
    const scriptOnly = lesson("ZH-S1", { language: "tamil", blocks: ["Warm-up", "Script — the letter", "Wrap-up Recall"] });
    expect(writingLetterOf(scriptOnly)).toBe("அ");
    const [placed] = withFilmstripImages([scriptOnly], filmstripCandidates([scriptOnly]));
    expect(placed!.blocks[1]!.markdown.startsWith("![How அ is written")).toBe(true);
  });

  it("prefers a Writing block over an earlier Script block", () => {
    const both = lesson("TA-S9", { blocks: ["Script — shape", "Writing: strokes", "Wrap-up Recall"] });
    const [placed] = withFilmstripImages([both], filmstripCandidates([both]));
    expect(placed!.blocks[0]!.markdown).not.toContain("filmstrip");
    expect(placed!.blocks[1]!.markdown).toContain("filmstrip");
  });

  it("places a declared target on a word lesson's first script-introducing block", () => {
    // FA-C03-chist teaches چ inside a word lesson: no Writing or Script block,
    // so it is never a derived candidate, but a DECLARED filmstrip still prints.
    const word = lesson("FA-C3", { type: "word", blocks: ["Warm-up", "You'll want to know", "Wrap-up Recall"] });
    word.blocks[1]!.knowledge = { introduces: ["FA-LEX-X", "FA-SCRIPT-CHE"], assesses: [] } as never;
    expect(writingLetterOf(word)).toBeUndefined();
    const declared: ScriptFilmstripTarget = {
      kind: "script-filmstrip",
      lessonId: "FA-C3",
      script: "perso-arabic",
      glyph: "چ",
      output: "persian/book/figures/FA-C3-filmstrip.svg",
    };
    const [placed] = withFilmstripImages([word], [declared]);
    expect(placed!.blocks[1]!.markdown.startsWith("![How چ is written")).toBe(true);
  });

  it("leaves a lesson that already prints its figure by hand alone", () => {
    const authored = lesson("TA-S1");
    authored.blocks[0]!.markdown = "![hand placed](figures/TA-S1-filmstrip.svg)\n";
    const [placed] = withFilmstripImages([authored], [target]);
    expect(placed).toBe(authored);
  });

  it("does not modify the lessons it was given", () => {
    const authored = lesson("TA-S1");
    withFilmstripImages([authored], [target]);
    expect(authored.blocks[1]!.markdown).toBe("body of Writing: x\n");
  });
});

describe("the shared figure macro", () => {
  it("bounds a block figure's height as well as its width", () => {
    // A filmstrip of a tall, narrow letter scaled to the line width alone ran
    // hundreds of points off the page in CI's XeLaTeX build (persian and urdu
    // alef). No test here compiles TeX, so the bound itself is pinned.
    const visual = readFileSync(join(defaultCurriculumRoot(), "_shared", "visual.tex"), "utf8");
    const block = visual.slice(visual.indexOf("\\newcommand{\\hlblockfigure}"));
    const options = /\\includegraphics\[([^\]]*)\]/.exec(block)?.[1] ?? "";
    expect(options).toMatch(/\bwidth=/);
    expect(options).toMatch(/\bheight=/);
    expect(options).toContain("keepaspectratio");
  });
});

describe("the real corpus", () => {
  const root = defaultCurriculumRoot();
  const lessons = loadLessons(root);
  const targets = resolvedFigureTargets(root, lessons).filter(
    (target): target is ScriptFilmstripTarget => target.kind === "script-filmstrip",
  );

  it("draws a filmstrip for every Tamil letter lesson whose letter has a cited ductus", () => {
    // 30 lessons, 29 letters (20 lessons / 19 letters on the first rollout; the
    // Script-block fallback added one lesson for one more letter; chapters
    // 109-112 added nine cited letters the reader had read in words since
    // chapter 1 but never written). வ has both TA-S01-va and the guided copy TA-W00, and
    // the ledger draws the letter once. A letter lesson
    // whose glyph has no cited ductus (vowel signs, the pulli) is a candidate
    // that is not drawn; citing its stroke order is what moves this number,
    // never an edit here alone.
    const tamil = targets.filter((target) => target.lessonId.startsWith("TA-"));
    expect(tamil).toHaveLength(30);
    expect(new Set(tamil.map((target) => target.glyph)).size).toBe(29);
  });

  it("draws every switched-on track exactly the letters its ductus cites", () => {
    // Rollouts: tamil; then the four Devanagari tracks, which share one cited
    // ductus (a letter drawn once serves four books); then every other track
    // with any cited ductus. Kannada, malayalam and telugu cite only vowels and
    // chillus, so only those letter lessons print a filmstrip until their
    // consonants are sourced. Tracks that present a letter under "## Script"
    // rather than "## Writing:" (chinese, japanese, urdu, persian, most russian)
    // get the filmstrip in that Script block.
    const counts: Record<string, number> = {};
    for (const target of targets) {
      const prefix = target.lessonId.split("-")[0]!;
      counts[prefix] = (counts[prefix] ?? 0) + 1;
    }
    expect(counts).toEqual({
      AR: 15,
      FA: 26,
      GU: 33,
      HI: 42,
      JA: 23,
      KA: 13,
      ML: 13,
      MR: 42,
      MW: 32,
      RU: 18,
      SA: 39,
      TA: 30,
      TE: 9,
      UR: 27,
      ZH: 58,
    });
  });

  it("prints every resolved filmstrip in its chapter", () => {
    // The gate HL-C443 was missing. A figure that is generated but never placed
    // is a figure no reader sees.
    const byLesson = new Map(lessons.map((entry) => [entry.realization.lessonId, entry]));
    for (const target of targets) {
      const owner = byLesson.get(target.lessonId);
      expect(owner, target.lessonId).toBeDefined();
      const chapterFile = chapterTexFor(root, owner!);
      const pdf = target.output.split("/").pop()!.replace(/\.svg$/, ".pdf");
      expect(readFileSync(chapterFile, "utf8"), `${target.lessonId} in ${chapterFile}`).toContain(pdf);
    }
  });
});

function chapterTexFor(root: string, owner: ParsedLesson): string {
  const targets = JSON.parse(
    readFileSync(
      join(
        root,
        "core",
        "book-generation.d",
        "targets.d",
        `${owner.language}-${String(owner.realization.chapter).padStart(4, "0")}.json`,
      ),
      "utf8",
    ),
  ) as { output: string };
  return join(root, targets.output);
}
