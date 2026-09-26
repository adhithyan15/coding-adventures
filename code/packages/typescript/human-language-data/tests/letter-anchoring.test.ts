/**
 * letter-anchoring.test.ts — HL-C443: is each letter written from a word the
 * reader already knows, and does every letter read eventually get written?
 *
 * Glyphs are named constants, as in script-closure.test.ts, so a maintainer who
 * cannot read Tamil can still see what each fixture holds.
 */

import { describe, expect, it } from "vitest";
import { mkdtempSync, mkdirSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { measureLetterAnchoring } from "../src/letter-anchoring.js";
import { defaultCurriculumRoot, loadLessons } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";
import { loadLetterAnchoringCeilingPins } from "./letter-anchoring-ceiling-pins.js";

const CEILING_DIR = fileURLToPath(new URL("./letter-anchoring-ceilings/", import.meta.url));
const CEILINGS = loadLetterAnchoringCeilingPins(CEILING_DIR);

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

  it("counts a digit no word holds as a numeral, not cold", () => {
    // TELUGU DIGIT ONE, TELUGU LETTER KA.
    const report = measureLetterAnchoring([
      lesson("TE-S1", 10, { type: "writing", headword: "\u0C67", language: "telugu" }),
      lesson("TE-S2", 20, { type: "writing", headword: "\u0C15", language: "telugu" }),
    ]);
    const telugu = track(report, "telugu");
    expect(telugu.letterLessons.map((entry) => entry.anchoring)).toEqual(["numeral", "cold"]);
    expect(telugu.numeral).toBe(1);
    expect(telugu.cold).toBe(1);
    expect(report.summary.numeral).toBe(1);
  });

  it("does not let a cold letter hide behind a digit in the same set", () => {
    const report = measureLetterAnchoring([
      lesson("TE-S1", 10, { type: "writing", headword: "\u0C67, \u0C15", language: "telugu" }),
    ]);
    expect(track(report, "telugu").letterLessons.map((entry) => entry.anchoring)).toEqual(["cold"]);
  });

  it("anchors a digit that an earlier word holds", () => {
    // KANNADA DIGIT ONE + NE: "first", as on a sign.
    const report = measureLetterAnchoring([
      lesson("KA-C1", 10, { headword: "\u0CE7\u0CA8\u0CC7", language: "kannada" }),
      lesson("KA-S1", 20, { type: "writing", chapter: 2, headword: "\u0CE7", language: "kannada" }),
    ]);
    expect(track(report, "kannada").letterLessons.map((entry) => entry.anchoring)).toEqual(["anchored"]);
  });

  it("anchors a capital in a word that holds its small letter", () => {
    // CYRILLIC SMALL LETTER YA ("I"), then the CAPITAL YA written on its own.
    const report = measureLetterAnchoring([
      lesson("RU-C1", 10, { headword: "\u044F", language: "russian" }),
      lesson("RU-S1", 20, { type: "writing", chapter: 2, headword: "\u042F", language: "russian" }),
      lesson("RU-S2", 30, { type: "writing", chapter: 3, headword: "\u0416", language: "russian" }),
    ]);
    expect(track(report, "russian").letterLessons.map((entry) => entry.anchoring)).toEqual(["anchored", "cold"]);
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

describe("letter-anchoring ceiling owner discovery", () => {
  function fixture(): string {
    return mkdtempSync(join(tmpdir(), "letter-anchoring-ceilings-"));
  }

  it("rejects empty, unsafe, nested, and malformed owner sets", () => {
    const root = fixture();
    try {
      expect(() => loadLetterAnchoringCeilingPins(root)).toThrow(/must not be empty/);
      writeFileSync(join(root, "bad.txt"), "{}\n");
      expect(() => loadLetterAnchoringCeilingPins(root)).toThrow(/unsafe/);
      rmSync(join(root, "bad.txt"));

      mkdirSync(join(root, "nested.json"));
      expect(() => loadLetterAnchoringCeilingPins(root)).toThrow(/real file/);
      rmSync(join(root, "nested.json"), { recursive: true });

      writeFileSync(join(root, "toy.json"), '{"cold":0,"buildsToward":0,"unwritten":-1}\n');
      expect(() => loadLetterAnchoringCeilingPins(root)).toThrow(/non-negative/);
      writeFileSync(join(root, "toy.json"), '{"cold":0,"buildsToward":0,"unwritten":0,"extra":0}\n');
      expect(() => loadLetterAnchoringCeilingPins(root)).toThrow(/expected exactly/);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("rejects uppercase and case-fold-colliding owner names", () => {
    const root = fixture();
    const pin = '{"cold":0,"buildsToward":0,"unwritten":0}\n';
    try {
      writeFileSync(join(root, "Toy.json"), pin);
      expect(() => loadLetterAnchoringCeilingPins(root)).toThrow(/use lowercase/);
      try {
        writeFileSync(join(root, "toy.json"), pin, { flag: "wx" });
      } catch {
        return;
      }
      expect(() => loadLetterAnchoringCeilingPins(root)).toThrow(/case-fold/);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("rejects a symlink owner without opening its target", () => {
    const root = fixture();
    const outside = fixture();
    const target = join(outside, "target.json");
    writeFileSync(target, '{"cold":0,"buildsToward":0,"unwritten":0}\n');
    try {
      try {
        symlinkSync(target, join(root, "toy.json"), "file");
      } catch {
        return;
      }
      expect(() => loadLetterAnchoringCeilingPins(root)).toThrow(/real file/);
    } finally {
      rmSync(root, { recursive: true, force: true });
      rmSync(outside, { recursive: true, force: true });
    }
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

describe("combinations", () => {
  it("measures a voiced kana by its parts: が is written once か and ゛ are", () => {
    const report = measureLetterAnchoring([
      lesson("JA-C1", 10, { headword: "ありがとう", language: "japanese" }),
      lesson("JA-W1", 20, { type: "writing", headword: "か", language: "japanese" }),
      lesson("JA-W2", 30, { type: "writing", headword: "\u309B", language: "japanese" }),
    ]);
    expect(track(report, "japanese").unwritten).not.toContain("が");
    expect(track(report, "japanese").unwritten).not.toContain("か");
    expect(track(report, "japanese").unwritten).not.toContain("\u309B");
  });

  it("still owes the mark when only the base kana has been written", () => {
    const report = measureLetterAnchoring([
      lesson("JA-C1", 10, { headword: "ご", language: "japanese" }),
      lesson("JA-W1", 20, { type: "writing", headword: "こ", language: "japanese" }),
    ]);
    expect(track(report, "japanese").unwritten).toEqual(["\u309B"]);
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
  // Measured 2026-09-25 (HL-C443). Every non-Latin track now writes every
  // letter its words use: tamil (chapters 109-112), arabic (46-49), persian
  // (22-26), urdu (34-37), and the long tail -- hindi 128, sanskrit 65, marwadi
  // 43, gujarati 45, bengali 41, malayalam 113-114, russian 28. Japanese reached
  // 0 without a lesson: が ご ざ じ ぽ are measured as combinations, and every
  // base kana and both marks already had lessons. The biggest anchoring debts
  // are marathi (43 cold) and gujarati (34 cold), whose letter lessons open the
  // track before any word does. Marwadi's 49 builds-toward are its "र, ा, then
  // राम" chapters, where the word follows the letter in the same chapter.
  //
  // Japanese's cold fell 6 -> 2 and its builds-toward rose 32 -> 35 when voiced
  // kana began to count by their parts: ありがとう now shows か and ゛ to the
  // eye, so the chapter-3 lessons for か, さ and ゛ (and the chapter-18 ゜) have
  // a word nearby instead of none, and ど is fully anchored. Better, not worse.
  //
  // Arabic's cold (5 -> 7) and builds-toward (4 -> 5) rose when the measure
  // learned to read a letters-then-word headword ("ا م — سلام"). No content
  // got worse: those lessons were invisible before, and now they are counted.
  //
  // Hindi's cold fell 9 -> 4 when five letter lessons moved to sit right after
  // the word that holds their letter: ृ after कृपया, ऋ after ऋतु, झ after साँझ,
  // औ after औरत, ण after प्रणाम. The four left need more than a move: घ and ढ
  // depend on each other in the script ladder, and ऊ and ओ have no word before
  // chapter 96.
  //
  // Telugu (10 -> 0), Kannada (10 -> 0) and Punjabi (7 -> 3) fell when digit
  // lessons began to count as `numeral`: no word is spelled with ౧ or ೨, so a
  // digit could never be anchored by a word, and its real anchor is the amount
  // the reader already knows. Punjabi's 3 left are real letter sets. Arabic's
  // builds-toward (5 -> 4) and Bengali's (12 -> 11) fell with the anchor words
  // of HL-C443's earlier passes.
  //
  // Marathi's cold fell 37 -> 6 when its opening runways began each chapter
  // with short words (दुःख, आभाळ, अमृत, घागर, टाळी, उलट ...) read before the
  // letters they hold. The six left: chapter 1's five (the chapter is at its
  // twelve-atom budget, and हो is written in the lesson itself), and ँ, which
  // no everyday Marathi word uses.
  //
  // Gujarati's cold fell 34 -> 4 (and builds-toward 5 -> 4) the same way:
  // thirteen words (આહાર, છીપ, કણ, શક, અથાણું, દિવાળી, ચપ્પલ, બગલો, ઉખાણું,
  // એકલું, ધૂળ, ઈંટ, ઢોલ) open chapters 1 and 3-7. The four left are chapter
  // 2's દ, ય, ધ and ૃ: that chapter is at its twelve-atom budget.
  //
  // Russian's last cold letter (1 -> 0) was the capital Я of chapter 14. The
  // small я is the word "I", taught in chapter 2; a case pair is one letter in
  // two forms, so the capital is now anchored by that word, the way が is
  // anchored by か and ゛. The small я keeps its own lesson in chapter 28.
  //
  // Arabic's last two cold lessons (2 -> 0) were the mark sets of chapter 2.
  // Headwords are written without marks, so no word held them. Two vocalized
  // words now come first: مُدَرِّس (fatha, kasra, damma, shadda) and أَهْلًا
  // (sukun, tanwin).
  //
  // Malayalam ഉ (3 -> 2) now follows ഉടുപ്പ് (uṭuppŭ, a dress), and Japanese わ
  // (2 -> 1) follows わに (wani, a crocodile): こんにちは says "wa" with は, so
  // no earlier word held わ. What is left: Malayalam's ഒ and ഏ ഴ, anchored by
  // number words its numbers chapter shows only in romanization, and Japanese
  // め, whose chapter is at its twelve-atom budget.
  //
  // Punjabi (3 -> 2): the chapter-20 ੌ lesson now follows ਮੌਸਮ (mausam, the
  // weather). The two sets left, ਟ ਠ ਡ (chapter 3) and ੜ ਘ ਦ (chapter 7), sit in
  // chapters whose R1 retrievals are one to three lessons apart, so a word
  // inserted there pushes older atoms out of R1.
  it("measures every non-Latin track", () => {
    expect(report.tracks.map((entry) => entry.language)).toEqual(Object.keys(CEILINGS));
  });

  for (const [language, { cold, buildsToward, unwritten }] of Object.entries(CEILINGS)) {
    it(`${language} does not get less gentle to write`, () => {
      const measured = track(report, language);
      expect(measured.cold, `${language} cold letter lessons`).toBeLessThanOrEqual(cold);
      expect(measured.buildsToward, `${language} builds-toward letter lessons`).toBeLessThanOrEqual(buildsToward);
      expect(measured.unwritten.length, `${language} unwritten: ${measured.unwritten.join(" ")}`).toBeLessThanOrEqual(unwritten);
    });
  }
});
