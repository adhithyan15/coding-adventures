/**
 * script-closure.test.ts — HL11: was the reader ever taught the letters?
 *
 * Glyphs are built from code points rather than typed, so a maintainer who
 * cannot read Tamil can still see what each fixture contains — and so a fixture
 * cannot drift into a lookalike from another script, which would silently change
 * what the test measures.
 */

import { describe, it, expect } from "vitest";
import { parseLesson } from "../src/parse.js";
import { measureScriptClosure } from "../src/script-closure.js";
import { loadEverything } from "../src/loader.js";

// TAMIL LETTER KA, MA, NA; TAMIL SIGN VIRAMA.
const KA = "க";
const MA = "ம";
const NA = "ந";
const VIRAMA = "்";

interface Options {
  type?: string;
  delivery?: string;
  headword?: string;
  romanization?: string;
  body?: string;
}

function lesson(id: string, sequence: number, o: Options = {}) {
  const front = [
    "---",
    "schema_version: 2",
    `id: ${id}`,
    `sequence: ${sequence}`,
    "chapter: 1",
    `type: ${o.type ?? "word"}`,
    `headword: "${o.headword ?? "x"}"`,
    'gloss: x',
    "concept_tag: GREETING-HELLO",
    ...(o.romanization === undefined ? [] : [`romanization: "${o.romanization}"`]),
    ...(o.delivery === undefined ? [] : [`delivery: ${o.delivery}`]),
    "---",
  ].join("\n");
  return parseLesson(
    `${front}\n\n# ${id}\n\n## Warm-up\n\n${o.body ?? "Say it."}\n`,
    "tamil",
  );
}

describe("closure", () => {
  it("flags a lesson whose body shows a glyph nothing taught", () => {
    const report = measureScriptClosure([
      lesson("TA-1", 10, { body: `Read this: ${KA}${MA}` }),
    ]);
    expect(report.summary.violations).toBe(1);
    expect(report.violations[0]?.glyphs).toBe([KA, MA].sort().join(""));
  });

  it("does not flag the same glyph once a script lesson has taught it", () => {
    const report = measureScriptClosure([
      lesson("TA-W1", 10, { type: "writing", body: `This letter: ${KA}` }),
      lesson("TA-2", 20, { body: `Read this: ${KA}` }),
    ]);
    expect(report.summary.violations).toBe(0);
  });

  it("respects reading order, not file order", () => {
    // The teaching lesson comes LATER in sequence, so the word lesson is still
    // in debt even though the corpus contains the letter somewhere. A ramp is a
    // claim about order; measuring it out of order measures nothing.
    const report = measureScriptClosure([
      lesson("TA-W1", 90, { type: "writing", body: `This letter: ${KA}` }),
      lesson("TA-2", 20, { body: `Read this: ${KA}` }),
    ]);
    expect(report.summary.violations).toBe(1);
    expect(report.violations[0]?.lessonId).toBe("TA-2");
  });

  it("accepts `delivery: script` as teaching, not just `type: writing`", () => {
    const report = measureScriptClosure([
      lesson("TA-W1", 10, { delivery: "script", body: `This letter: ${KA}` }),
      lesson("TA-2", 20, { body: `Read this: ${KA}` }),
    ]);
    expect(report.summary.violations).toBe(0);
    expect(report.tracks[0]?.scriptLessons).toBe(1);
  });

  it("never puts a script lesson in debt to itself", () => {
    const report = measureScriptClosure([
      lesson("TA-W1", 10, { type: "writing", body: `Brand new: ${KA}${MA}${NA}` }),
    ]);
    expect(report.summary.violations).toBe(0);
    expect(report.tracks[0]?.taughtGlyphs).toBe(3);
  });

  it("orders violations steepest first, as a work queue", () => {
    const report = measureScriptClosure([
      lesson("TA-1", 10, { body: `${KA}` }),
      lesson("TA-2", 20, { body: `${KA}${MA}${NA}${VIRAMA}` }),
    ]);
    expect(report.violations.map((v) => v.lessonId)).toEqual(["TA-2", "TA-1"]);
  });
});

describe("the exposure rule", () => {
  it("exempts a headword that carries a romanization", () => {
    // The reader is being SHOWN the word, not asked to read it -- the
    // romanization is the promise that they can use it without decoding.
    const report = measureScriptClosure([
      lesson("TA-1", 10, {
        headword: `${KA}${MA}`, romanization: "kama", body: `${KA}${MA}`,
      }),
    ]);
    expect(report.summary.violations).toBe(0);
    expect(report.tracks[0]?.exposureOnly).toBe(1);
  });

  it("does NOT exempt the same headword without one", () => {
    const report = measureScriptClosure([
      lesson("TA-1", 10, { headword: `${KA}${MA}`, body: `${KA}${MA}` }),
    ]);
    expect(report.summary.violations).toBe(1);
    expect(report.tracks[0]?.headwordsWithoutRomanization).toBe(1);
  });

  it("does not let an exempt headword launder the rest of the lesson", () => {
    // This is the loophole worth guarding: the exemption covers the headword's
    // glyphs, not every glyph in a lesson that happens to have a headword.
    const report = measureScriptClosure([
      lesson("TA-1", 10, {
        headword: `${KA}`, romanization: "ka",
        body: `${KA} and now decode this: ${MA}${NA}`,
      }),
    ]);
    expect(report.summary.violations).toBe(1);
    expect(report.violations[0]?.glyphs).toBe([MA, NA].sort().join(""));
  });

  it("counts an empty romanization as no romanization", () => {
    const report = measureScriptClosure([
      lesson("TA-1", 10, { headword: `${KA}`, romanization: "   ", body: `${KA}` }),
    ]);
    expect(report.summary.violations).toBe(1);
  });

  it("reports where the exposure rule is doing the work", () => {
    // The exposure count sits beside the violation count so the rule cannot
    // quietly become the reason the number looks good.
    const report = measureScriptClosure([
      lesson("TA-1", 10, { headword: `${KA}`, romanization: "ka", body: `${KA}` }),
      lesson("TA-2", 20, { headword: `${MA}`, romanization: "ma", body: `${MA}` }),
    ]);
    expect(report.summary.violations).toBe(0);
    expect(report.summary.exposureOnly).toBe(2);
  });
});

describe("track rollups", () => {
  it("counts glyphs shown, taught, and never taught", () => {
    const report = measureScriptClosure([
      lesson("TA-W1", 10, { type: "writing", body: `${KA}` }),
      lesson("TA-2", 20, { body: `${KA}${MA}` }),
    ]);
    const track = report.tracks[0]!;
    expect(track.taughtGlyphs).toBe(1);
    expect(track.shownGlyphs).toBe(2);
    expect(track.neverTaughtGlyphs).toBe(1);
  });

  it("names a track that teaches no letters at all", () => {
    const report = measureScriptClosure([lesson("TA-1", 10, { body: `${KA}` })]);
    expect(report.summary.tracksTeachingNothing).toBe(1);
    expect(report.tracks[0]?.scriptLessons).toBe(0);
  });

  it("skips Latin-script tracks entirely", () => {
    // Their reader arrives already knowing the alphabet, which is the whole
    // reason this measurement exists for the others.
    const spanish = parseLesson(
      "---\nschema_version: 2\nid: ES-1\nsequence: 10\nchapter: 1\ntype: word\n" +
        "headword: hola\ngloss: hello\nconcept_tag: GREETING-HELLO\n---\n\n# ES-1\n\nhola\n",
      "spanish",
    );
    const report = measureScriptClosure([spanish]);
    expect(report.tracks).toEqual([]);
    expect(report.summary.violations).toBe(0);
  });
});

describe("the real corpus", () => {
  const { lessons } = loadEverything();
  const report = measureScriptClosure(lessons);

  it("measures every non-Latin track and no Latin one", () => {
    expect(report.summary.tracksWithScript).toBeGreaterThanOrEqual(10);
    expect(report.tracks.map((t) => t.language)).not.toContain("spanish");
    expect(report.tracks.map((t) => t.language)).toContain("tamil");
  });

  it("finds far more closure debt than the pace budget ever could", () => {
    // The point of the whole module. HL08's glyph budget flags tens of lessons
    // for arriving too fast; closure flags hundreds for arriving untaught, and
    // a track can satisfy the budget perfectly while teaching nothing.
    expect(report.summary.violations).toBeGreaterThan(500);
    expect(report.summary.tracksTeachingNothing).toBeGreaterThan(5);
  });

  it("every violation names real glyphs and a real lesson", () => {
    const ids = new Set(lessons.map((l) => l.realization.lessonId));
    for (const v of report.violations) {
      expect(ids.has(v.lessonId), v.lessonId).toBe(true);
      expect(v.count).toBe([...v.glyphs].length);
      expect(v.count).toBeGreaterThan(0);
    }
  });

  it("Tamil is in less debt than the tracks with no writing lessons", () => {
    // Tamil has script lessons; Telugu, Kannada, Malayalam and Sanskrit have
    // none. The measurement should reflect that difference rather than flatten
    // it, or it is not measuring what it claims to.
    const tamil = report.tracks.find((t) => t.language === "tamil")!;
    for (const language of ["telugu", "kannada", "malayalam"]) {
      const other = report.tracks.find((t) => t.language === language);
      if (!other) continue;
      expect(other.scriptLessons, language).toBe(0);
      expect(tamil.neverTaughtGlyphs, language).toBeLessThan(other.neverTaughtGlyphs);
    }
  });
});
