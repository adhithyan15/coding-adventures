/**
 * letter-anchoring.test.ts — HL-C443: is each letter written from a word the
 * reader already knows, and does every letter read eventually get written?
 *
 * Glyphs are named constants, as in script-closure.test.ts, so a maintainer who
 * cannot read Tamil can still see what each fixture holds.
 */

import { describe, expect, it } from "vitest";
import { measureLetterAnchoring } from "../src/letter-anchoring.js";
import { defaultCurriculumRoot, loadLessons } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";

// TAMIL LETTER KA, MA, VA; TAMIL SIGN VIRAMA.
const KA = "க";
const MA = "ம";
const VA = "வ";
const VIRAMA = "்";
// CJK: person, and its side-radical form.
const REN = "人";
const PERSON_RADICAL = "亻";

interface Options {
  type?: string;
  chapter?: number;
  headword: string;
  language?: string;
}

function lesson(id: string, sequence: number, options: Options) {
  const type = options.type ?? "word";
  const front = [
    "---",
    "schema_version: 2",
    `id: ${id}`,
    `sequence: ${sequence}`,
    `chapter: ${options.chapter ?? 1}`,
    `type: ${type}`,
    `headword: "${options.headword}"`,
    "gloss: x",
    "concept_tag: GREETING-HELLO",
    'romanization: "x"',
    "---",
  ].join("\n");
  const block = type === "writing" ? "## Writing: the letter" : "## Warm-up";
  return parseLesson(`${front}\n\n# ${id}\n\n${block}\n\nSay it.\n`, options.language ?? "tamil");
}

function track(report: ReturnType<typeof measureLetterAnchoring>, language = "tamil") {
  return report.tracks.find((entry) => entry.language === language)!;
}

describe("anchoring", () => {
  it("anchors a letter that an earlier word headword holds", () => {
    const report = measureLetterAnchoring([
      lesson("TA-C1", 10, { headword: `${VA}${KA}` }),
      lesson("TA-S1", 20, { type: "writing", headword: VA }),
    ]);
    expect(track(report).letterLessons.map((entry) => entry.anchoring)).toEqual(["anchored"]);
  });

  it("calls a letter whose word comes later in the same chapter builds-toward", () => {
    const report = measureLetterAnchoring([
      lesson("TA-S1", 10, { type: "writing", headword: MA }),
      lesson("TA-C1", 20, { headword: `${MA}${KA}` }),
    ]);
    expect(track(report).buildsToward).toBe(1);
    expect(track(report).cold).toBe(0);
  });

  it("calls a letter cold when its word is in a later chapter, or nowhere", () => {
    const report = measureLetterAnchoring([
      lesson("TA-S1", 10, { type: "writing", headword: MA }),
      lesson("TA-S2", 20, { type: "writing", headword: KA }),
      lesson("TA-C2", 30, { chapter: 2, headword: MA }),
    ]);
    expect(track(report).letterLessons.map((entry) => entry.anchoring)).toEqual(["cold", "cold"]);
  });

  it("needs every glyph of a consonant-plus-sign letter to be anchored", () => {
    const report = measureLetterAnchoring([
      lesson("TA-C1", 10, { headword: KA }),
      lesson("TA-S1", 20, { type: "writing", chapter: 1, headword: `${KA}${VIRAMA}` }),
    ]);
    expect(track(report).cold).toBe(1);
  });

  it("does not let a writing lesson's own headword make a letter known", () => {
    // Copying a whole word is practice, not a word the reader has learned.
    const report = measureLetterAnchoring([
      lesson("TA-W1", 10, { type: "writing", headword: `${VA}${KA}` }),
      lesson("TA-S1", 20, { type: "writing", chapter: 1, headword: VA }),
    ]);
    expect(track(report).cold).toBe(1);
  });

  it("reports a Han component with no visible anchor as unmeasured, not cold", () => {
    const report = measureLetterAnchoring([
      lesson("ZH-S1", 10, { type: "writing", headword: PERSON_RADICAL, language: "chinese" }),
      lesson("ZH-S2", 20, { type: "writing", headword: REN, language: "chinese" }),
    ]);
    const chinese = track(report, "chinese");
    expect(chinese.unmeasured).toBe(2);
    expect(chinese.cold).toBe(0);
  });

  it("skips a Latin-script track: its reader can already write the alphabet", () => {
    const report = measureLetterAnchoring([lesson("ES-C1", 10, { headword: "hola", language: "spanish" })]);
    expect(report.tracks).toEqual([]);
  });
});

describe("letter sets", () => {
  it("counts every letter of a letter set as written, and anchors each one", () => {
    const report = measureLetterAnchoring([
      lesson("TA-C1", 10, { headword: `${VA}${KA}` }),
      lesson("TA-W1", 20, { type: "writing", headword: `${VA}, ${KA}` }),
    ]);
    expect(track(report).letterLessons).toEqual([
      { lessonId: "TA-W1", letter: `${VA} ${KA}`, chapter: 1, anchoring: "anchored" },
    ]);
    expect(track(report).unwritten).toEqual([]);
  });

  it("counts the leading letters of a letters-then-word headword", () => {
    const report = measureLetterAnchoring([
      lesson("TA-C1", 10, { headword: `${VA}${KA}` }),
      lesson("TA-W1", 20, { type: "writing", headword: `${VA} ${KA} — ${VA}${KA}` }),
      lesson("TA-W2", 30, { type: "writing", headword: `${MA}, ${VA}${KA}` }),
    ]);
    expect(track(report).letterLessons.map((entry) => entry.letter)).toEqual([`${VA} ${KA}`, MA]);
  });

  it("does not treat a copied word, or a word before a letter, as a letter set", () => {
    const report = measureLetterAnchoring([
      lesson("TA-W1", 10, { type: "writing", headword: `${VA}${KA}` }),
      lesson("TA-W2", 20, { type: "writing", headword: `${VA}${KA} ${MA}` }),
    ]);
    expect(track(report).letterLessons).toEqual([]);
  });

  it("never counts the Arabic tatweel as a letter", () => {
    const report = measureLetterAnchoring([
      lesson("AR-C1", 10, { headword: "\u0628\u0640\u0628", language: "arabic" }),
    ]);
    expect(track(report, "arabic").unwritten).toEqual(["\u0628"]);
  });
});

describe("completeness", () => {
  it("lists every glyph read in a word that no letter lesson writes", () => {
    const report = measureLetterAnchoring([
      lesson("TA-C1", 10, { headword: `${VA}${KA}${MA}` }),
      lesson("TA-S1", 20, { type: "writing", headword: VA }),
    ]);
    expect(track(report).unwritten).toEqual([KA, MA].sort());
    expect(track(report).lettersRead).toBe(3);
    expect(track(report).lettersWritten).toBe(1);
  });
});

describe("the real corpus", () => {
  const report = measureLetterAnchoring(loadLessons(defaultCurriculumRoot()));

  // A RATCHET, per track: [cold, builds-toward, unwritten]. Each may fall and
  // must not rise. A new letter lesson must come after a word that holds its
  // letter. A new word must not bring a letter no letter lesson writes, unless
  // this pin moves in the same change and the commit says why.
  //
  // Measured 2026-09-25 (HL-C443). The biggest completeness debts are persian
  // 18 and urdu 16: letters the reader meets in word after word and never
  // writes. Tamil (chapters 109-112) and arabic (chapters 46-49) each went to 0
  // by writing every such letter from the word it came from. The biggest
  // anchoring debts are marathi (43 cold) and gujarati (34 cold), whose letter
  // lessons open the track before any word does. Marwadi's 49 builds-toward are
  // its "र, ा, then राम" chapters, where the word follows the letter in the
  // same chapter.
  //
  // Arabic's cold (5 -> 7) and builds-toward (4 -> 5) rose when the measure
  // learned to read a letters-then-word headword ("ا م — سلام"). No content
  // got worse: those lessons were invisible before, and now they are counted.
  const CEILINGS: Record<string, [cold: number, buildsToward: number, unwritten: number]> = {
    arabic: [7, 5, 0],
    bengali: [5, 12, 5],
    chinese: [0, 51, 0],
    gujarati: [34, 5, 2],
    hindi: [9, 1, 3],
    japanese: [6, 32, 5],
    kannada: [19, 0, 0],
    malayalam: [12, 12, 9],
    marathi: [43, 4, 0],
    marwadi: [0, 49, 1],
    persian: [0, 4, 18],
    punjabi: [7, 2, 0],
    russian: [1, 0, 1],
    sanskrit: [1, 0, 1],
    tamil: [1, 9, 0],
    telugu: [13, 0, 0],
    urdu: [0, 5, 16],
  };

  it("measures every non-Latin track", () => {
    expect(report.tracks.map((entry) => entry.language)).toEqual(Object.keys(CEILINGS));
  });

  for (const [language, [cold, buildsToward, unwritten]] of Object.entries(CEILINGS)) {
    it(`${language} does not get less gentle to write`, () => {
      const measured = track(report, language);
      expect(measured.cold, `${language} cold letter lessons`).toBeLessThanOrEqual(cold);
      expect(measured.buildsToward, `${language} builds-toward letter lessons`).toBeLessThanOrEqual(buildsToward);
      expect(measured.unwritten.length, `${language} unwritten: ${measured.unwritten.join(" ")}`).toBeLessThanOrEqual(unwritten);
    });
  }
});
